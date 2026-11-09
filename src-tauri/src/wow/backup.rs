use crate::config;
use crate::types::WowInstall;
use crate::wow::{detect, install};
use filetime::{set_file_mtime, FileTime};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

fn emit(on_progress: &mut impl FnMut(&str), line: &str) {
	if line.is_empty() {
		return;
	}
	log::debug!("sync> {line}");
	on_progress(line);
}

/// True when destination already matches source (size + mtime), like rsync's quick check.
fn file_unchanged(src: &Path, dst: &Path) -> bool {
	let Ok(src_meta) = fs::metadata(src) else {
		return false;
	};
	let Ok(dst_meta) = fs::metadata(dst) else {
		return false;
	};
	if !src_meta.is_file() || !dst_meta.is_file() {
		return false;
	}
	if src_meta.len() != dst_meta.len() {
		return false;
	}
	match (src_meta.modified(), dst_meta.modified()) {
		(Ok(a), Ok(b)) => a == b,
		_ => false,
	}
}

fn copy_file_preserving_mtime(src: &Path, dst: &Path) -> Result<(), String> {
	if let Some(parent) = dst.parent() {
		fs::create_dir_all(parent).map_err(|e| format!("无法创建目录 {}: {e}", parent.display()))?;
	}
	fs::copy(src, dst).map_err(|e| {
		format!(
			"同步文件失败 {} -> {}: {e}",
			src.display(),
			dst.display()
		)
	})?;
	if let Ok(meta) = fs::metadata(src) {
		let mtime = FileTime::from_last_modification_time(&meta);
		if let Err(err) = set_file_mtime(dst, mtime) {
			log::debug!("保留 mtime 失败 {}: {err}", dst.display());
		}
	}
	Ok(())
}

/// Local mirror sync: copy/update, skip unchanged, delete extras; skip `.bak` / `.old`.
fn mirror_dir(
	src: &Path,
	dst: &Path,
	on_progress: &mut impl FnMut(&str),
) -> Result<String, String> {
	if !src.is_dir() {
		return Err(format!("源目录不存在: {}", src.display()));
	}
	fs::create_dir_all(dst).map_err(|e| format!("无法创建目录 {}: {e}", dst.display()))?;

	log::info!(
		"内置镜像同步开始 src={} dst={}",
		src.display(),
		dst.display()
	);
	let mut kept: HashSet<PathBuf> = HashSet::new();
	let mut copied = 0usize;
	let mut skipped = 0usize;
	let mut deleted = 0usize;
	let mut log_lines: Vec<String> = Vec::new();
	let start = format!("[mirror] {} → {}", src.display(), dst.display());
	emit(on_progress, &start);
	log_lines.push(start);

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
			if file_unchanged(path, &target) {
				skipped += 1;
				log::trace!("跳过未变更 {}", rel.display());
				continue;
			}
			copy_file_preserving_mtime(path, &target)?;
			copied += 1;
			let line = format!("+ {}", rel.display());
			emit(on_progress, &line);
			log_lines.push(line);
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
		let line = format!("- {rel}");
		emit(on_progress, &line);
		log_lines.push(line);
		log::trace!("删除 {rel}");
	}

	let summary = format!("完成：复制 {copied}，跳过 {skipped}，删除 {deleted}");
	emit(on_progress, &summary);
	log_lines.push(summary);
	let log_text = log_lines.join("\n") + "\n";
	log::info!(
		"内置镜像同步完成 copied={copied} skipped={skipped} deleted={deleted} src={} dst={}",
		src.display(),
		dst.display()
	);
	Ok(log_text)
}

fn mirror_tree(
	src: &Path,
	dst: &Path,
	on_progress: &mut impl FnMut(&str),
) -> Result<(String, String), String> {
	let log = mirror_dir(src, dst, on_progress)?;
	Ok(("mirror".into(), log))
}

fn sync_pair(
	src_root: &Path,
	dst_root: &Path,
	label: &str,
	on_progress: &mut impl FnMut(&str),
) -> Result<(Vec<&'static str>, String, String), String> {
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
	let mut log_chunks = Vec::new();

	if has_interface {
		log::info!("同步 Interface …");
		emit(on_progress, "===== Interface =====");
		let (_engine, chunk) =
			mirror_tree(&interface_src, &dst_root.join("Interface"), on_progress)?;
		log_chunks.push(format!("===== Interface =====\n{chunk}"));
		synced.push("Interface");
	}
	if has_wtf {
		log::info!("同步 WTF …");
		emit(on_progress, "===== WTF =====");
		let (_engine, chunk) = mirror_tree(&wtf_src, &dst_root.join("WTF"), on_progress)?;
		log_chunks.push(format!("===== WTF =====\n{chunk}"));
		synced.push("WTF");
	}

	Ok((synced, "mirror".into(), log_chunks.join("\n")))
}

/// Sync live `Interface` / `WTF` into this client's configured backup folder.
pub fn backup_install(
	install_id: &str,
	mut on_progress: impl FnMut(&str),
) -> Result<SyncReport, String> {
	log::info!("开始备份 install_id={install_id}");
	let install = resolve_install(install_id)?;
	let install_path = PathBuf::from(&install.install_path);
	let dest_root = backup_root_for(install_id)?;
	emit(
		&mut on_progress,
		&format!("开始备份 {} → {}", install.label, dest_root.display()),
	);
	log::info!(
		"备份路径 install={} src={} dest={}",
		install.label,
		install_path.display(),
		dest_root.display()
	);
	let (synced, engine, log_text) =
		match sync_pair(&install_path, &dest_root, "客户端", &mut on_progress) {
			Ok(v) => v,
			Err(err) => {
				log::error!("备份失败 install={} err={err}", install.label);
				emit(&mut on_progress, &format!("错误: {err}"));
				return Err(err);
			}
		};
	emit(
		&mut on_progress,
		&format!("备份完成（{}）: {}", engine, synced.join(" + ")),
	);
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
pub fn restore_install(
	install_id: &str,
	mut on_progress: impl FnMut(&str),
) -> Result<SyncReport, String> {
	log::info!("开始恢复 install_id={install_id}");
	let install = resolve_install(install_id)?;
	let install_path = PathBuf::from(&install.install_path);
	let src_root = backup_root_for(install_id)?;
	if !src_root.join("Interface").is_dir() && !src_root.join("WTF").is_dir() {
		let err = format!("未找到该客户端的备份目录：{}", src_root.display());
		log::error!("{err}");
		emit(&mut on_progress, &format!("错误: {err}"));
		return Err(err);
	}
	emit(
		&mut on_progress,
		&format!("开始恢复 {} ← {}", install.label, src_root.display()),
	);
	log::info!(
		"恢复路径 install={} from={} to={}",
		install.label,
		src_root.display(),
		install_path.display()
	);
	let (synced, engine, log_text) =
		match sync_pair(&src_root, &install_path, "备份", &mut on_progress) {
			Ok(v) => v,
			Err(err) => {
				log::error!("恢复失败 install={} err={err}", install.label);
				emit(&mut on_progress, &format!("错误: {err}"));
				return Err(err);
			}
		};
	emit(
		&mut on_progress,
		&format!("恢复完成（{}）: {}", engine, synced.join(" + ")),
	);
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
	use std::time::{Duration, SystemTime, UNIX_EPOCH};

	fn noop(_: &str) {}

	fn stamp() -> u128 {
		SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	}

	#[test]
	fn mirror_syncs_and_skips_bak_old() {
		let root = std::env::temp_dir().join(format!("wam-backup-{}", stamp()));
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

		let mut lines = Vec::new();
		let (engine, log) = mirror_tree(
			&client.join("Interface"),
			&backup.join("Interface"),
			&mut |line| lines.push(line.to_string()),
		)
		.unwrap();
		assert_eq!(engine, "mirror");
		assert!(log.contains("Foo.toc") || log.contains('+'));
		assert!(!lines.is_empty(), "progress callback should receive lines");
		mirror_tree(&client.join("WTF"), &backup.join("WTF"), &mut noop).unwrap();

		assert!(backup.join("Interface/AddOns/Foo/Foo.toc").is_file());
		assert!(backup.join("WTF/Account/config.wtf").is_file());
		assert!(!backup.join("Interface/AddOns/Foo/Foo.lua.bak").exists());
		assert!(!backup.join("Interface/AddOns/Foo/Foo.xml.old").exists());
		assert!(!backup.join("WTF/Account/config.wtf.bak").exists());

		fs::write(backup.join("Interface/AddOns/Foo/Stale.lua"), "-- gone\n").unwrap();
		let toc = client.join("Interface/AddOns/Foo/Foo.toc");
		// Different length so size check catches the change even on coarse mtime FS.
		fs::write(&toc, "## Interface: 16002\n## Updated\n").unwrap();
		let (_engine, log2) = mirror_tree(
			&client.join("Interface"),
			&backup.join("Interface"),
			&mut noop,
		)
		.unwrap();
		assert!(!backup.join("Interface/AddOns/Foo/Stale.lua").exists());
		assert!(log2.contains("Stale") || log2.contains('-') || log2.contains("Foo.toc"));
		assert_eq!(
			fs::read_to_string(backup.join("Interface/AddOns/Foo/Foo.toc")).unwrap(),
			"## Interface: 16002\n## Updated\n"
		);

		let backup_toc = backup.join("Interface/AddOns/Foo/Foo.toc");
		fs::write(&backup_toc, "## Interface: restored\n## From backup\n").unwrap();
		mirror_tree(
			&backup.join("Interface"),
			&client.join("Interface"),
			&mut noop,
		)
		.unwrap();
		assert_eq!(
			fs::read_to_string(client.join("Interface/AddOns/Foo/Foo.toc")).unwrap(),
			"## Interface: restored\n## From backup\n"
		);

		let _ = fs::remove_dir_all(root);
	}

	#[test]
	fn mirror_streams_copy_and_delete() {
		let root = std::env::temp_dir().join(format!("wam-mirror-log-{}", stamp()));
		let src = root.join("src");
		let dst = root.join("dst");
		let _ = fs::remove_dir_all(&root);
		fs::create_dir_all(src.join("a")).unwrap();
		fs::write(src.join("a/one.txt"), "1\n").unwrap();
		fs::create_dir_all(&dst).unwrap();
		fs::write(dst.join("gone.txt"), "x\n").unwrap();

		let mut streamed = Vec::new();
		let log = mirror_dir(&src, &dst, &mut |line| streamed.push(line.to_string())).unwrap();
		assert!(dst.join("a/one.txt").is_file());
		assert!(!dst.join("gone.txt").exists());
		assert!(log.contains("one.txt"));
		assert!(log.contains("gone.txt"));
		assert!(
			streamed.iter().any(|l| l.contains("one.txt")),
			"live stream should include copied file: {streamed:?}"
		);
		assert!(
			streamed.iter().any(|l| l.contains("gone.txt")),
			"live stream should include deleted file: {streamed:?}"
		);
		let _ = fs::remove_dir_all(root);
	}

	#[test]
	fn mirror_skips_unchanged_on_second_pass() {
		let root = std::env::temp_dir().join(format!("wam-mirror-skip-{}", stamp()));
		let src = root.join("src");
		let dst = root.join("dst");
		let _ = fs::remove_dir_all(&root);
		fs::create_dir_all(src.join("a")).unwrap();
		fs::write(src.join("a/keep.txt"), "same\n").unwrap();

		mirror_dir(&src, &dst, &mut noop).unwrap();
		// Ensure mtime comparison is stable across filesystems with coarse resolution.
		std::thread::sleep(Duration::from_millis(20));
		let mut lines = Vec::new();
		let log = mirror_dir(&src, &dst, &mut |line| lines.push(line.to_string())).unwrap();
		assert!(
			log.contains("跳过 1") || log.contains("复制 0"),
			"second pass should skip unchanged file: {log}"
		);
		assert!(
			!lines.iter().any(|l| l.starts_with("+ a/keep.txt") || l.starts_with("+ a\\keep.txt")),
			"unchanged file should not be re-emitted as copy: {lines:?}"
		);

		fs::write(src.join("a/keep.txt"), "changed-content\n").unwrap();
		let log2 = mirror_dir(&src, &dst, &mut noop).unwrap();
		assert!(
			log2.contains("keep.txt") && log2.contains('+'),
			"changed file should be copied: {log2}"
		);
		assert_eq!(
			fs::read_to_string(dst.join("a/keep.txt")).unwrap(),
			"changed-content\n"
		);
		let _ = fs::remove_dir_all(root);
	}
}
