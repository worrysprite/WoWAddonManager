use crate::config;
use crate::types::WowInstall;
use crate::wow::{detect, install};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use walkdir::WalkDir;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
	pub path: String,
	pub engine: String,
	pub items: Vec<String>,
	pub log: String,
	pub success: bool,
}

fn resolve_install(install_id: &str) -> Result<WowInstall, String> {
	let cfg = config::load();
	let report = detect::detect_with_extra_roots(&cfg.extra_wow_roots);
	install::find_install_in(&report.installs, install_id)
}

fn should_skip_backup_file(path: &Path) -> bool {
	path.extension()
		.and_then(|ext| ext.to_str())
		.map(|ext| {
			let ext = ext.to_ascii_lowercase();
			ext == "bak" || ext == "old"
		})
		.unwrap_or(false)
}

/// Per-client backup folder is the sync root (`Interface` / `WTF` live directly under it).
fn backup_root_for(install_id: &str) -> Result<PathBuf, String> {
	let backup_root = config::backup_path_for_install(install_id)
		.ok_or_else(|| "未设置备份路径".to_string())?;
	Ok(PathBuf::from(backup_root))
}

fn ensure_trailing_slash(path: &Path) -> String {
	let mut s = path.display().to_string();
	if !s.ends_with('/') && !s.ends_with('\\') {
		s.push('/');
	}
	s
}

fn configure_child(cmd: &mut Command) {
	cmd.stdin(Stdio::null())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped());
	#[cfg(windows)]
	{
		// Prevent a console window from flashing on Windows GUI apps.
		cmd.creation_flags(CREATE_NO_WINDOW);
	}
}

fn rsync_available() -> bool {
	let mut cmd = Command::new("rsync");
	configure_child(&mut cmd);
	cmd.arg("--version");
	match cmd.output() {
		Ok(out) => {
			let ok = out.status.success();
			log::debug!(
				"rsync --version status={} stdout_len={} stderr_len={}",
				out.status,
				out.stdout.len(),
				out.stderr.len()
			);
			ok
		}
		Err(err) => {
			log::info!("系统未找到 rsync，将使用内置镜像同步: {err}");
			false
		}
	}
}

fn run_rsync(src: &Path, dst: &Path) -> Result<String, String> {
	log::info!(
		"rsync 开始 src={} dst={}",
		src.display(),
		dst.display()
	);
	let mut cmd = Command::new("rsync");
	configure_child(&mut cmd);
	cmd.args([
		"-avh",
		"--delete",
		"--exclude=*.bak",
		"--exclude=*.BAK",
		"--exclude=*.old",
		"--exclude=*.OLD",
	])
	.arg(ensure_trailing_slash(src))
	.arg(ensure_trailing_slash(dst));

	let output = cmd
		.output()
		.map_err(|e| format!("启动 rsync 失败: {e}"))?;
	let stdout = String::from_utf8_lossy(&output.stdout).to_string();
	let stderr = String::from_utf8_lossy(&output.stderr).to_string();
	let mut log_text = String::new();
	log_text.push_str(&format!(
		"$ rsync -avh --delete --exclude=*.bak --exclude=*.old \\\n  \"{}\" \\\n  \"{}\"\n\n",
		ensure_trailing_slash(src),
		ensure_trailing_slash(dst)
	));
	if !stdout.trim().is_empty() {
		log_text.push_str(stdout.trim_end());
		log_text.push('\n');
	}
	if !stderr.trim().is_empty() {
		if !log_text.ends_with('\n') {
			log_text.push('\n');
		}
		log_text.push_str("[stderr]\n");
		log_text.push_str(stderr.trim_end());
		log_text.push('\n');
	}
	log::debug!(
		"rsync 结束 status={} stdout_len={} stderr_len={}",
		output.status,
		stdout.len(),
		stderr.len()
	);
	if !output.status.success() {
		log::error!("rsync 失败 status={} detail={}", output.status, log_text);
		return Err(format!(
			"rsync 失败 ({}):\n{}",
			output.status,
			log_text.trim()
		));
	}
	log::info!("rsync 成功 src={} dst={}", src.display(), dst.display());
	Ok(log_text)
}

/// Mirror `src` into `dst`, skipping `.bak` / `.old` (like rsync --exclude).
fn rsync_mirror(src: &Path, dst: &Path) -> Result<(String, String), String> {
	if !src.is_dir() {
		return Err(format!("源目录不存在: {}", src.display()));
	}
	fs::create_dir_all(dst).map_err(|e| format!("无法创建目录 {}: {e}", dst.display()))?;

	if rsync_available() {
		let log = run_rsync(src, dst)?;
		return Ok(("rsync".into(), log));
	}

	let log = mirror_dir_fallback(src, dst)?;
	Ok(("mirror".into(), log))
}

/// Rust fallback when system rsync is unavailable: copy/update then delete extras.
fn mirror_dir_fallback(src: &Path, dst: &Path) -> Result<String, String> {
	log::info!(
		"内置镜像同步开始 src={} dst={}",
		src.display(),
		dst.display()
	);
	let mut kept: HashSet<PathBuf> = HashSet::new();
	let mut copied = 0usize;
	let mut deleted = 0usize;
	let mut lines: Vec<String> = Vec::new();
	lines.push(format!("[mirror] {} -> {}", src.display(), dst.display()));

	for entry in WalkDir::new(src).follow_links(false) {
		let entry = entry.map_err(|e| format!("遍历目录失败 {}: {e}", src.display()))?;
		let path = entry.path();
		let rel = path
			.strip_prefix(src)
			.map_err(|e| format!("路径计算失败: {e}"))?;
		if rel.as_os_str().is_empty() {
			continue;
		}
		if entry.file_type().is_file() && should_skip_backup_file(path) {
			log::trace!("跳过备份文件 {}", path.display());
			continue;
		}
		let target = dst.join(rel);
		kept.insert(rel.to_path_buf());
		if entry.file_type().is_dir() {
			fs::create_dir_all(&target)
				.map_err(|e| format!("无法创建目录 {}: {e}", target.display()))?;
		} else if entry.file_type().is_file() {
			if let Some(parent) = target.parent() {
				fs::create_dir_all(parent)
					.map_err(|e| format!("无法创建目录 {}: {e}", parent.display()))?;
			}
			fs::copy(path, &target).map_err(|e| {
				format!(
					"同步文件失败 {} -> {}: {e}",
					path.display(),
					target.display()
				)
			})?;
			copied += 1;
			if lines.len() < 200 {
				lines.push(format!("+ {}", rel.display()));
			}
			log::trace!("复制 {}", rel.display());
		}
	}

	let mut to_remove: Vec<PathBuf> = Vec::new();
	if dst.is_dir() {
		for entry in WalkDir::new(dst).follow_links(false).contents_first(true) {
			let entry = entry.map_err(|e| format!("遍历备份目录失败 {}: {e}", dst.display()))?;
			let path = entry.path();
			let rel = path
				.strip_prefix(dst)
				.map_err(|e| format!("路径计算失败: {e}"))?;
			if rel.as_os_str().is_empty() {
				continue;
			}
			if entry.file_type().is_file() && should_skip_backup_file(path) {
				continue;
			}
			if !kept.contains(rel) {
				to_remove.push(path.to_path_buf());
			}
		}
	}
	for path in to_remove {
		let rel = path
			.strip_prefix(dst)
			.map(|p| p.display().to_string())
			.unwrap_or_else(|_| path.display().to_string());
		if path.is_dir() {
			fs::remove_dir_all(&path)
				.map_err(|e| format!("同步删除目录失败 {}: {e}", path.display()))?;
		} else if path.is_file() {
			fs::remove_file(&path)
				.map_err(|e| format!("同步删除文件失败 {}: {e}", path.display()))?;
		} else {
			continue;
		}
		deleted += 1;
		if lines.len() < 200 {
			lines.push(format!("- {rel}"));
		}
		log::trace!("删除 {rel}");
	}

	if copied + deleted > 200 {
		lines.push(format!(
			"… 其余变更已省略（复制 {copied}，删除 {deleted}）"
		));
	}
	lines.push(format!("完成：复制 {copied} 个文件，删除 {deleted} 项"));
	let log_text = lines.join("\n") + "\n";
	log::info!(
		"内置镜像同步完成 copied={copied} deleted={deleted} src={} dst={}",
		src.display(),
		dst.display()
	);
	Ok(log_text)
}

fn sync_pair(src_root: &Path, dst_root: &Path, label: &str) -> Result<(Vec<&'static str>, String, String), String> {
	let interface_src = src_root.join("Interface");
	let wtf_src = src_root.join("WTF");
	let has_interface = interface_src.is_dir();
	let has_wtf = wtf_src.is_dir();
	if !has_interface && !has_wtf {
		return Err(format!(
			"{label}目录下未找到 Interface 或 WTF：{}",
			src_root.display()
		));
	}

	fs::create_dir_all(dst_root)
		.map_err(|e| format!("无法创建目标目录 {}: {e}", dst_root.display()))?;

	let mut synced = Vec::new();
	let mut engines = HashSet::new();
	let mut log_chunks = Vec::new();

	if has_interface {
		log::info!("同步 Interface …");
		let (engine, chunk) = rsync_mirror(&interface_src, &dst_root.join("Interface"))?;
		engines.insert(engine);
		log_chunks.push(format!("===== Interface =====\n{chunk}"));
		synced.push("Interface");
	}
	if has_wtf {
		log::info!("同步 WTF …");
		let (engine, chunk) = rsync_mirror(&wtf_src, &dst_root.join("WTF"))?;
		engines.insert(engine);
		log_chunks.push(format!("===== WTF =====\n{chunk}"));
		synced.push("WTF");
	}

	let engine = if engines.len() == 1 {
		engines.into_iter().next().unwrap_or_else(|| "mirror".into())
	} else {
		engines.into_iter().collect::<Vec<_>>().join("+")
	};
	Ok((synced, engine, log_chunks.join("\n")))
}

/// Sync live `Interface` / `WTF` into this client's configured backup folder.
pub fn backup_install(install_id: &str) -> Result<SyncReport, String> {
	log::info!("开始备份 install_id={install_id}");
	let install = resolve_install(install_id)?;
	let install_path = PathBuf::from(&install.install_path);
	let dest_root = backup_root_for(install_id)?;
	log::info!(
		"备份路径 install={} src={} dest={}",
		install.label,
		install_path.display(),
		dest_root.display()
	);
	let (synced, engine, log_text) = match sync_pair(&install_path, &dest_root, "客户端") {
		Ok(v) => v,
		Err(err) => {
			log::error!("备份失败 install={} err={err}", install.label);
			return Err(err);
		}
	};
	log::warn!(
		"备份同步完成 install={} dest={} items={} via={}",
		install.label,
		dest_root.display(),
		synced.join("+"),
		engine
	);
	Ok(SyncReport {
		path: dest_root.display().to_string(),
		engine,
		items: synced.into_iter().map(|s| s.to_string()).collect(),
		log: log_text,
		success: true,
	})
}

/// Sync this client's backup `Interface` / `WTF` back into the live install.
pub fn restore_install(install_id: &str) -> Result<SyncReport, String> {
	log::info!("开始恢复 install_id={install_id}");
	let install = resolve_install(install_id)?;
	let install_path = PathBuf::from(&install.install_path);
	let src_root = backup_root_for(install_id)?;
	if !src_root.join("Interface").is_dir() && !src_root.join("WTF").is_dir() {
		let err = format!("未找到该客户端的备份目录：{}", src_root.display());
		log::error!("{err}");
		return Err(err);
	}
	log::info!(
		"恢复路径 install={} from={} to={}",
		install.label,
		src_root.display(),
		install_path.display()
	);
	let (synced, engine, log_text) = match sync_pair(&src_root, &install_path, "备份") {
		Ok(v) => v,
		Err(err) => {
			log::error!("恢复失败 install={} err={err}", install.label);
			return Err(err);
		}
	};
	log::warn!(
		"恢复同步完成 install={} from={} items={} via={}",
		install.label,
		src_root.display(),
		synced.join("+"),
		engine
	);
	Ok(SyncReport {
		path: src_root.display().to_string(),
		engine,
		items: synced.into_iter().map(|s| s.to_string()).collect(),
		log: log_text,
		success: true,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::{SystemTime, UNIX_EPOCH};

	#[test]
	fn mirror_syncs_and_skips_bak_old() {
		let stamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let root = std::env::temp_dir().join(format!("wam-backup-{stamp}"));
		let client = root.join("client");
		let backup = root.join("backup");
		let _ = fs::remove_dir_all(&root);
		fs::create_dir_all(client.join("Interface/AddOns/Foo")).unwrap();
		fs::write(client.join("Interface/AddOns/Foo/Foo.toc"), "## Interface: 16001\n").unwrap();
		fs::write(client.join("Interface/AddOns/Foo/Foo.lua.bak"), "-- old\n").unwrap();
		fs::write(client.join("Interface/AddOns/Foo/Foo.xml.old"), "<!-- old -->\n").unwrap();
		fs::create_dir_all(client.join("WTF/Account")).unwrap();
		fs::write(client.join("WTF/Account/config.wtf"), "SET foo \"bar\"\n").unwrap();
		fs::write(client.join("WTF/Account/config.wtf.bak"), "SET foo \"old\"\n").unwrap();

		let (engine, log) =
			rsync_mirror(&client.join("Interface"), &backup.join("Interface")).unwrap();
		assert!(!engine.is_empty());
		assert!(log.contains("Interface") || log.contains("Foo") || log.contains("mirror") || log.contains("sending") || log.contains("total") || log.contains("+"));
		rsync_mirror(&client.join("WTF"), &backup.join("WTF")).unwrap();

		assert!(backup.join("Interface/AddOns/Foo/Foo.toc").is_file());
		assert!(backup.join("WTF/Account/config.wtf").is_file());
		assert!(!backup.join("Interface/AddOns/Foo/Foo.lua.bak").exists());
		assert!(!backup.join("Interface/AddOns/Foo/Foo.xml.old").exists());
		assert!(!backup.join("WTF/Account/config.wtf.bak").exists());

		fs::write(backup.join("Interface/AddOns/Foo/Stale.lua"), "-- gone\n").unwrap();
		fs::remove_file(client.join("Interface/AddOns/Foo/Foo.toc")).unwrap();
		fs::write(
			client.join("Interface/AddOns/Foo/Foo.toc"),
			"## Interface: 16002\n",
		)
		.unwrap();
		let (_engine, log2) =
			rsync_mirror(&client.join("Interface"), &backup.join("Interface")).unwrap();
		assert!(!backup.join("Interface/AddOns/Foo/Stale.lua").exists());
		assert!(
			log2.contains("Stale")
				|| log2.contains("deleting")
				|| log2.contains("- ")
				|| log2.contains("Foo.toc")
				|| log2.contains("完成")
		);
		assert_eq!(
			fs::read_to_string(backup.join("Interface/AddOns/Foo/Foo.toc")).unwrap(),
			"## Interface: 16002\n"
		);

		fs::write(
			backup.join("Interface/AddOns/Foo/Foo.toc"),
			"## Interface: restored\n",
		)
		.unwrap();
		rsync_mirror(&backup.join("Interface"), &client.join("Interface")).unwrap();
		assert_eq!(
			fs::read_to_string(client.join("Interface/AddOns/Foo/Foo.toc")).unwrap(),
			"## Interface: restored\n"
		);

		let _ = fs::remove_dir_all(root);
	}

	#[test]
	fn mirror_fallback_log_mentions_copy_and_delete() {
		let stamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let root = std::env::temp_dir().join(format!("wam-mirror-log-{stamp}"));
		let src = root.join("src");
		let dst = root.join("dst");
		let _ = fs::remove_dir_all(&root);
		fs::create_dir_all(src.join("a")).unwrap();
		fs::write(src.join("a/one.txt"), "1\n").unwrap();
		fs::create_dir_all(&dst).unwrap();
		fs::write(dst.join("gone.txt"), "x\n").unwrap();

		let log = mirror_dir_fallback(&src, &dst).unwrap();
		assert!(dst.join("a/one.txt").is_file());
		assert!(!dst.join("gone.txt").exists());
		assert!(log.contains("one.txt") && (log.contains('+') || log.contains("复制")));
		assert!(log.contains("gone.txt") && (log.contains('-') || log.contains("删除")));
		let _ = fs::remove_dir_all(root);
	}
}
