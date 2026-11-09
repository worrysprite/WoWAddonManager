use crate::config;
use crate::types::WowInstall;
use crate::wow::{detect, install};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

fn resolve_install(install_id: &str) -> Result<WowInstall, String> {
	let cfg = config::load();
	let report = detect::detect_with_extra_roots(&cfg.extra_wow_roots);
	install::find_install_in(&report.installs, install_id)
}

fn sanitize_folder_name(raw: &str) -> String {
	let cleaned: String = raw
		.chars()
		.map(|c| match c {
			'/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
			c if c.is_control() => '_',
			c => c,
		})
		.collect();
	let trimmed = cleaned.trim().trim_matches('.');
	if trimmed.is_empty() {
		"wow-client".into()
	} else {
		trimmed.to_string()
	}
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

fn backup_folder_name(install: &WowInstall) -> String {
	// Keep label readable; append install id (`wow-<hash>`) so two clients with
	// the same flavor label never share a backup directory.
	format!(
		"{}__{}",
		sanitize_folder_name(&install.label),
		sanitize_folder_name(&install.id)
	)
}

fn backup_root_for(install: &WowInstall) -> Result<PathBuf, String> {
	let cfg = config::load();
	let backup_root = cfg
		.backup_path
		.as_deref()
		.map(str::trim)
		.filter(|p| !p.is_empty())
		.ok_or_else(|| "未设置备份路径".to_string())?;
	Ok(PathBuf::from(backup_root).join(backup_folder_name(install)))
}

fn ensure_trailing_slash(path: &Path) -> String {
	let mut s = path.display().to_string();
	if !s.ends_with('/') && !s.ends_with('\\') {
		s.push('/');
	}
	s
}

fn rsync_available() -> bool {
	Command::new("rsync")
		.arg("--version")
		.output()
		.map(|out| out.status.success())
		.unwrap_or(false)
}

/// Mirror `src` into `dst`, skipping `.bak` / `.old` (like rsync --exclude).
fn rsync_mirror(src: &Path, dst: &Path) -> Result<(), String> {
	if !src.is_dir() {
		return Err(format!("源目录不存在: {}", src.display()));
	}
	fs::create_dir_all(dst).map_err(|e| format!("无法创建目录 {}: {e}", dst.display()))?;

	if rsync_available() {
		let output = Command::new("rsync")
			.args([
				"-a",
				"--delete",
				"--exclude=*.bak",
				"--exclude=*.BAK",
				"--exclude=*.old",
				"--exclude=*.OLD",
			])
			.arg(ensure_trailing_slash(src))
			.arg(ensure_trailing_slash(dst))
			.output()
			.map_err(|e| format!("启动 rsync 失败: {e}"))?;
		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			let stdout = String::from_utf8_lossy(&output.stdout);
			let detail = if stderr.trim().is_empty() {
				stdout.trim()
			} else {
				stderr.trim()
			};
			return Err(format!("rsync 失败 ({}): {detail}", output.status));
		}
		return Ok(());
	}

	mirror_dir_fallback(src, dst)
}

/// Rust fallback when system rsync is unavailable: copy/update then delete extras.
fn mirror_dir_fallback(src: &Path, dst: &Path) -> Result<(), String> {
	let mut kept: HashSet<PathBuf> = HashSet::new();

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
		}
	}

	// Delete destination entries that are no longer in source (keep .bak/.old).
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
		if path.is_dir() {
			fs::remove_dir_all(&path)
				.map_err(|e| format!("同步删除目录失败 {}: {e}", path.display()))?;
		} else if path.is_file() {
			fs::remove_file(&path)
				.map_err(|e| format!("同步删除文件失败 {}: {e}", path.display()))?;
		}
	}
	Ok(())
}

fn sync_pair(src_root: &Path, dst_root: &Path, label: &str) -> Result<Vec<&'static str>, String> {
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
	if has_interface {
		rsync_mirror(&interface_src, &dst_root.join("Interface"))?;
		synced.push("Interface");
	}
	if has_wtf {
		rsync_mirror(&wtf_src, &dst_root.join("WTF"))?;
		synced.push("WTF");
	}
	Ok(synced)
}

/// Sync live `Interface` / `WTF` into the configured backup folder for this client.
pub fn backup_install(install_id: &str) -> Result<String, String> {
	let install = resolve_install(install_id)?;
	let install_path = PathBuf::from(&install.install_path);
	let dest_root = backup_root_for(&install)?;
	let synced = sync_pair(&install_path, &dest_root, "客户端")?;
	log::warn!(
		"备份同步完成 install={} dest={} items={} via={}",
		install.label,
		dest_root.display(),
		synced.join("+"),
		if rsync_available() {
			"rsync"
		} else {
			"mirror"
		}
	);
	Ok(dest_root.display().to_string())
}

/// Sync backup `Interface` / `WTF` back into the live client install.
pub fn restore_install(install_id: &str) -> Result<String, String> {
	let install = resolve_install(install_id)?;
	let install_path = PathBuf::from(&install.install_path);
	let backup_root = backup_root_for(&install)?;
	if !backup_root.is_dir() {
		return Err(format!(
			"未找到该客户端的备份目录：{}",
			backup_root.display()
		));
	}
	let synced = sync_pair(&backup_root, &install_path, "备份")?;
	log::warn!(
		"恢复同步完成 install={} from={} items={} via={}",
		install.label,
		backup_root.display(),
		synced.join("+"),
		if rsync_available() {
			"rsync"
		} else {
			"mirror"
		}
	);
	Ok(backup_root.display().to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::{SystemTime, UNIX_EPOCH};

	#[test]
	fn sanitize_replaces_invalid_chars() {
		assert_eq!(sanitize_folder_name("Retail: _retail_"), "Retail_ _retail_");
		assert_eq!(sanitize_folder_name("   "), "wow-client");
	}

	#[test]
	fn backup_folder_includes_install_id() {
		let a = WowInstall {
			id: "c__games__wow__retail".into(),
			label: "Retail".into(),
			flavor: "Retail".into(),
			flavor_key: "retail".into(),
			install_path: r"C:\Games\World of Warcraft\_retail_".into(),
			addons_path: r"C:\Games\World of Warcraft\_retail_\Interface\AddOns".into(),
			game_version: None,
			game_version_type_id: None,
		};
		let b = WowInstall {
			id: "d__other__wow__retail".into(),
			label: "Retail".into(),
			..a.clone()
		};
		assert_ne!(backup_folder_name(&a), backup_folder_name(&b));
		assert!(backup_folder_name(&a).starts_with("Retail__"));
	}

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

		rsync_mirror(&client.join("Interface"), &backup.join("Interface")).unwrap();
		rsync_mirror(&client.join("WTF"), &backup.join("WTF")).unwrap();

		assert!(backup.join("Interface/AddOns/Foo/Foo.toc").is_file());
		assert!(backup.join("WTF/Account/config.wtf").is_file());
		assert!(!backup.join("Interface/AddOns/Foo/Foo.lua.bak").exists());
		assert!(!backup.join("Interface/AddOns/Foo/Foo.xml.old").exists());
		assert!(!backup.join("WTF/Account/config.wtf.bak").exists());

		// Stale file in backup should be removed on next sync.
		fs::write(backup.join("Interface/AddOns/Foo/Stale.lua"), "-- gone\n").unwrap();
		fs::remove_file(client.join("Interface/AddOns/Foo/Foo.toc")).unwrap();
		fs::write(
			client.join("Interface/AddOns/Foo/Foo.toc"),
			"## Interface: 16002\n",
		)
		.unwrap();
		rsync_mirror(&client.join("Interface"), &backup.join("Interface")).unwrap();
		assert!(!backup.join("Interface/AddOns/Foo/Stale.lua").exists());
		assert_eq!(
			fs::read_to_string(backup.join("Interface/AddOns/Foo/Foo.toc")).unwrap(),
			"## Interface: 16002\n"
		);

		// Restore direction.
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
}
