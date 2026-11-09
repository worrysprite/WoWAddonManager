use std::fs::{self, File};
use std::io::{copy, Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;

const IGNORED_TOP: &[&str] = &["__macosx", ".ds_store", "thumbs.db"];

pub fn extract_archive(archive_path: &Path, addons_path: &Path) -> Result<Vec<String>, String> {
	fs::create_dir_all(addons_path).map_err(|e| e.to_string())?;
	let ext = extension_of(archive_path);
	match ext.as_str() {
		"zip" => extract_zip_file(archive_path, addons_path),
		"7z" => extract_7z_file(archive_path, addons_path),
		"rar" => extract_via_external_7zip(archive_path, addons_path)
			.or_else(|err| Err(format!("无法解压 RAR：{err}。请安装 7-Zip，或改用 ZIP/7Z 压缩包。"))),
		"tar" => extract_via_external_7zip(archive_path, addons_path)
			.or_else(|_| Err("暂不支持该 tar 压缩包，请改用 ZIP/7Z。".into())),
		"gz" | "tgz" | "bz2" | "xz" => extract_via_external_7zip(archive_path, addons_path)
			.or_else(|_| Err(format!("暂不直接支持 .{ext}，请改用 ZIP/7Z，或安装 7-Zip 后重试。"))),
		other => Err(format!("不支持的压缩格式：.{other}（支持 zip / 7z / rar）")),
	}
}

pub fn extract_zip_bytes(bytes: &[u8], addons_path: &Path) -> Result<Vec<String>, String> {
	fs::create_dir_all(addons_path).map_err(|e| e.to_string())?;
	let staging = staging_dir(addons_path)?;
	let reader = Cursor::new(bytes);
	let mut archive = ZipArchive::new(reader).map_err(|e| e.to_string())?;
	for i in 0..archive.len() {
		let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
		let owned_name = file
			.enclosed_name()
			.map(|p| p.to_path_buf())
			.or_else(|| safe_relative_path(Some(Path::new(file.name()))));
		let Some(rel) = owned_name.and_then(|p| safe_relative_path(Some(&p))) else {
			continue;
		};
		write_archive_entry(&staging, &rel, file.name().ends_with('/'), &mut file)?;
	}
	finalize_extract(&staging, addons_path)
}

fn extract_zip_file(archive_path: &Path, addons_path: &Path) -> Result<Vec<String>, String> {
	let bytes = fs::read(archive_path).map_err(|e| e.to_string())?;
	extract_zip_bytes(&bytes, addons_path)
}

fn extract_7z_file(archive_path: &Path, addons_path: &Path) -> Result<Vec<String>, String> {
	let staging = staging_dir(addons_path)?;
	sevenz_rust::decompress_file(archive_path, &staging).map_err(|e| e.to_string())?;
	// sevenz-rust may create nested structure already under staging
	finalize_extract(&staging, addons_path).or_else(|_| {
		// Fallback to system 7-Zip if crate fails oddly
		let _ = fs::remove_dir_all(&staging);
		extract_via_external_7zip(archive_path, addons_path)
	})
}

fn extract_via_external_7zip(archive_path: &Path, addons_path: &Path) -> Result<Vec<String>, String> {
	let seven_zip = find_7zip_executable().ok_or_else(|| {
		"未找到 7-Zip（7z.exe）。RAR/部分格式需要系统安装 7-Zip。".to_string()
	})?;
	let staging = staging_dir(addons_path)?;
	let status = Command::new(seven_zip)
		.arg("x")
		.arg(archive_path)
		.arg(format!("-o{}", staging.display()))
		.arg("-y")
		.status()
		.map_err(|e| e.to_string())?;
	if !status.success() {
		let _ = fs::remove_dir_all(&staging);
		return Err("7-Zip 解压失败".into());
	}
	finalize_extract(&staging, addons_path)
}

fn find_7zip_executable() -> Option<PathBuf> {
	let candidates = [
		PathBuf::from(r"C:\Program Files\7-Zip\7z.exe"),
		PathBuf::from(r"C:\Program Files (x86)\7-Zip\7z.exe"),
	];
	for path in candidates {
		if path.exists() {
			return Some(path);
		}
	}
	which("7z").or_else(|| which("7za")).or_else(|| which("7z.exe"))
}

fn which(name: &str) -> Option<PathBuf> {
	let path = std::env::var_os("PATH")?;
	for dir in std::env::split_paths(&path) {
		let candidate = dir.join(name);
		if candidate.exists() {
			return Some(candidate);
		}
	}
	None
}

fn staging_dir(addons_path: &Path) -> Result<PathBuf, String> {
	let staging = addons_path.join(format!(
		".wam-extract-{}",
		chrono::Utc::now().timestamp_millis()
	));
	fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
	Ok(staging)
}

fn finalize_extract(staging: &Path, addons_path: &Path) -> Result<Vec<String>, String> {
	normalize_flat_toc_layout(staging)?;
	let folders = collect_addon_folders(staging)?;
	if folders.is_empty() {
		let _ = fs::remove_dir_all(staging);
		return Err("压缩包中未找到有效的插件目录（需包含 .toc 文件）".into());
	}

	for folder in &folders {
		let src = staging.join(folder);
		let dest = addons_path.join(folder);
		if dest.exists() {
			fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
		}
		// Prefer rename; fallback to copy.
		if fs::rename(&src, &dest).is_err() {
			copy_dir_recursive(&src, &dest)?;
		}
	}

	let _ = fs::remove_dir_all(staging);
	Ok(folders)
}

fn normalize_flat_toc_layout(staging: &Path) -> Result<(), String> {
	let mut root_tocs = Vec::new();
	let mut has_subdir = false;
	for entry in fs::read_dir(staging).map_err(|e| e.to_string())? {
		let entry = entry.map_err(|e| e.to_string())?;
		let path = entry.path();
		let name = entry.file_name().to_string_lossy().to_string();
		if should_ignore(&name) {
			continue;
		}
		if path.is_dir() {
			has_subdir = true;
		} else if name.to_lowercase().ends_with(".toc") {
			root_tocs.push(path);
		}
	}
	if has_subdir || root_tocs.is_empty() {
		return Ok(());
	}

	// Flat zip: wrap files into a folder named after the first toc stem.
	let folder_name = root_tocs[0]
		.file_stem()
		.and_then(|s| s.to_str())
		.unwrap_or("ManualAddon")
		.to_string();
	let target = staging.join(&folder_name);
	fs::create_dir_all(&target).map_err(|e| e.to_string())?;
	for entry in fs::read_dir(staging).map_err(|e| e.to_string())? {
		let entry = entry.map_err(|e| e.to_string())?;
		let path = entry.path();
		if path == target {
			continue;
		}
		let name = entry.file_name();
		if should_ignore(&name.to_string_lossy()) {
			continue;
		}
		fs::rename(&path, target.join(&name)).map_err(|e| e.to_string())?;
	}
	Ok(())
}

fn collect_addon_folders(staging: &Path) -> Result<Vec<String>, String> {
	let mut folders = Vec::new();
	for entry in fs::read_dir(staging).map_err(|e| e.to_string())? {
		let entry = entry.map_err(|e| e.to_string())?;
		let path = entry.path();
		let name = entry.file_name().to_string_lossy().to_string();
		if !path.is_dir() || should_ignore(&name) || name.starts_with('.') {
			continue;
		}
		if dir_has_toc(&path) {
			folders.push(name);
		}
	}
	folders.sort();
	Ok(folders)
}

fn dir_has_toc(dir: &Path) -> bool {
	fs::read_dir(dir)
		.ok()
		.into_iter()
		.flatten()
		.flatten()
		.any(|entry| {
			entry
				.path()
				.extension()
				.and_then(|s| s.to_str())
				.map(|s| s.eq_ignore_ascii_case("toc"))
				.unwrap_or(false)
		})
}

fn write_archive_entry<R: Read>(
	staging: &Path,
	rel: &Path,
	is_dir: bool,
	reader: &mut R,
) -> Result<(), String> {
	if rel.components().next().is_none() {
		return Ok(());
	}
	if let Some(Component::Normal(first)) = rel.components().next() {
		if should_ignore(&first.to_string_lossy()) {
			return Ok(());
		}
	}
	let out_path = staging.join(rel);
	if is_dir {
		fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
		return Ok(());
	}
	if let Some(parent) = out_path.parent() {
		fs::create_dir_all(parent).map_err(|e| e.to_string())?;
	}
	let mut outfile = File::create(&out_path).map_err(|e| e.to_string())?;
	copy(reader, &mut outfile).map_err(|e| e.to_string())?;
	Ok(())
}

fn safe_relative_path(path: Option<&Path>) -> Option<PathBuf> {
	let path = path?;
	let mut out = PathBuf::new();
	for component in path.components() {
		match component {
			Component::Normal(part) => out.push(part),
			Component::CurDir => {}
			_ => return None,
		}
	}
	if out.as_os_str().is_empty() {
		None
	} else {
		Some(out)
	}
}

fn should_ignore(name: &str) -> bool {
	let lower = name.to_lowercase();
	IGNORED_TOP.iter().any(|item| lower == *item) || lower.starts_with(".__")
}

fn extension_of(path: &Path) -> String {
	let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
	if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
		return "tgz".into();
	}
	if name.ends_with(".tar.bz2") {
		return "bz2".into();
	}
	path.extension()
		.and_then(|s| s.to_str())
		.unwrap_or("")
		.to_lowercase()
}

pub(crate) fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
	fs::create_dir_all(dest).map_err(|e| e.to_string())?;
	for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
		let entry = entry.map_err(|e| e.to_string())?;
		let from = entry.path();
		let to = dest.join(entry.file_name());
		if from.is_dir() {
			copy_dir_recursive(&from, &to)?;
		} else {
			fs::copy(&from, &to).map_err(|e| e.to_string())?;
		}
	}
	Ok(())
}

