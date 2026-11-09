use crate::config;
use crate::types::WowInstall;
use crate::wow::{detect, install};
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};
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

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
	if !src.is_dir() {
		return Err(format!("源目录不存在: {}", src.display()));
	}
	fs::create_dir_all(dst).map_err(|e| format!("无法创建备份目录 {}: {e}", dst.display()))?;

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
					"复制文件失败 {} -> {}: {e}",
					path.display(),
					target.display()
				)
			})?;
		}
	}
	Ok(())
}

/// Copy `Interface` and `WTF` under the client install into a timestamped folder
/// inside the configured backup path.
pub fn backup_install(install_id: &str) -> Result<String, String> {
	let cfg = config::load();
	let backup_root = cfg
		.backup_path
		.as_deref()
		.map(str::trim)
		.filter(|p| !p.is_empty())
		.ok_or_else(|| "未设置备份路径".to_string())?;

	let install = resolve_install(install_id)?;
	let install_path = PathBuf::from(&install.install_path);
	let interface_src = install_path.join("Interface");
	let wtf_src = install_path.join("WTF");

	let has_interface = interface_src.is_dir();
	let has_wtf = wtf_src.is_dir();
	if !has_interface && !has_wtf {
		return Err(format!(
			"客户端目录下未找到 Interface 或 WTF：{}",
			install_path.display()
		));
	}

	let stamp = Local::now().format("%Y%m%d_%H%M%S");
	let folder_name = format!("{}_{stamp}", sanitize_folder_name(&install.label));
	let dest_root = PathBuf::from(backup_root).join(folder_name);
	if dest_root.exists() {
		return Err(format!("备份目录已存在：{}", dest_root.display()));
	}
	fs::create_dir_all(&dest_root)
		.map_err(|e| format!("无法创建备份目录 {}: {e}", dest_root.display()))?;

	let mut copied = Vec::new();
	if has_interface {
		copy_dir_recursive(&interface_src, &dest_root.join("Interface"))?;
		copied.push("Interface");
	}
	if has_wtf {
		copy_dir_recursive(&wtf_src, &dest_root.join("WTF"))?;
		copied.push("WTF");
	}

	log::warn!(
		"备份完成 install={} dest={} items={}",
		install.label,
		dest_root.display(),
		copied.join("+")
	);

	Ok(dest_root.display().to_string())
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
	fn copies_interface_and_wtf() {
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

		copy_dir_recursive(&client.join("Interface"), &backup.join("Interface")).unwrap();
		copy_dir_recursive(&client.join("WTF"), &backup.join("WTF")).unwrap();

		assert!(backup.join("Interface/AddOns/Foo/Foo.toc").is_file());
		assert!(backup.join("WTF/Account/config.wtf").is_file());
		assert!(!backup.join("Interface/AddOns/Foo/Foo.lua.bak").exists());
		assert!(!backup.join("Interface/AddOns/Foo/Foo.xml.old").exists());
		assert!(!backup.join("WTF/Account/config.wtf.bak").exists());
		let _ = fs::remove_dir_all(root);
	}
}
