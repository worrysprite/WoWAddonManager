//! WTF account / character profile list, delete, and copy.
//!
//! Copy scope (see internal note + PR):
//! - Character → character: SavedVariables/, layout-local.txt, layout-cache.txt,
//!   config-cache.wtf, chat-cache.txt, AddOns.txt.
//!   Skips bindings-cache.wtf and macros-cache.txt so destination keybinds/macros stay.
//! - Account → account: SavedVariables/, SavedVariables.lua, config-cache.wtf,
//!   macros-cache.txt, bindings-cache.wtf, edit-mode-cache-account.txt.
//!   Does not copy realm/character subfolders.

use crate::config;
use crate::types::WowInstall;
use crate::wow::{detect, install};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Known non-realm directory names directly under an account folder.
const ACCOUNT_SKIP_DIRS: &[&str] = &["SavedVariables"];

/// Files/dirs copied for character → character (relative to char folder).
const CHAR_COPY_ITEMS: &[&str] = &[
	"SavedVariables",
	"layout-local.txt",
	"layout-cache.txt",
	"config-cache.wtf",
	"chat-cache.txt",
	"AddOns.txt",
];

/// Files/dirs copied for account → account (relative to account folder).
const ACCOUNT_COPY_ITEMS: &[&str] = &[
	"SavedVariables",
	"SavedVariables.lua",
	"config-cache.wtf",
	"macros-cache.txt",
	"bindings-cache.wtf",
	"edit-mode-cache-account.txt",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WtfCharacter {
	pub realm: String,
	pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WtfAccount {
	pub name: String,
	pub characters: Vec<WtfCharacter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WtfProfileTree {
	pub wtf_path: String,
	pub accounts: Vec<WtfAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WtfAccountRef {
	pub account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WtfCharacterRef {
	pub account: String,
	pub realm: String,
	pub character: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WtfDeleteAccountRequest {
	pub install_id: String,
	pub account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WtfDeleteCharacterRequest {
	pub install_id: String,
	pub account: String,
	pub realm: String,
	pub character: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WtfCopyAccountRequest {
	pub install_id: String,
	pub from: WtfAccountRef,
	pub to: WtfAccountRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WtfCopyCharacterRequest {
	pub install_id: String,
	pub from: WtfCharacterRef,
	pub to: WtfCharacterRef,
}

fn resolve_install(install_id: &str) -> Result<WowInstall, String> {
	let cfg = config::load();
	let report = detect::detect_with_extra_roots(&cfg.extra_wow_roots);
	install::find_install_in(&report.installs, install_id)
}

fn wtf_root_for(install: &WowInstall) -> PathBuf {
	PathBuf::from(&install.install_path).join("WTF")
}

fn account_root(wtf: &Path) -> PathBuf {
	wtf.join("Account")
}

/// Reject empty names and path separators / `..` so callers cannot escape WTF.
fn validate_segment(kind: &str, value: &str) -> Result<(), String> {
	let trimmed = value.trim();
	if trimmed.is_empty() {
		return Err(format!("{kind}名称不能为空"));
	}
	if trimmed != value {
		return Err(format!("{kind}名称无效"));
	}
	if value.contains('/')
		|| value.contains('\\')
		|| value.contains('\0')
		|| value == "."
		|| value == ".."
		|| value.contains("..")
	{
		return Err(format!("非法{kind}名称: {value}"));
	}
	Ok(())
}

fn ensure_under(child: &Path, ancestor: &Path) -> Result<(), String> {
	let child_c = child
		.canonicalize()
		.map_err(|e| format!("无法解析路径 {}: {e}", child.display()))?;
	let ancestor_c = ancestor
		.canonicalize()
		.map_err(|e| format!("无法解析路径 {}: {e}", ancestor.display()))?;
	if !child_c.starts_with(&ancestor_c) {
		return Err(format!(
			"路径超出允许范围: {} 不在 {}",
			child.display(),
			ancestor.display()
		));
	}
	Ok(())
}

fn is_skipped_account_dir(name: &str) -> bool {
	ACCOUNT_SKIP_DIRS
		.iter()
		.any(|s| s.eq_ignore_ascii_case(name))
}

fn looks_like_character_dir(path: &Path) -> bool {
	if !path.is_dir() {
		return false;
	}
	path.join("SavedVariables").is_dir()
		|| path.join("layout-local.txt").is_file()
		|| path.join("config-cache.wtf").is_file()
		|| path.join("AddOns.txt").is_file()
		|| path.join("chat-cache.txt").is_file()
}

fn list_dir_names(path: &Path) -> Result<Vec<String>, String> {
	if !path.is_dir() {
		return Ok(Vec::new());
	}
	let mut names = Vec::new();
	for entry in fs::read_dir(path).map_err(|e| format!("读取目录失败 {}: {e}", path.display()))? {
		let entry = entry.map_err(|e| format!("读取目录项失败: {e}"))?;
		let meta = entry
			.file_type()
			.map_err(|e| format!("读取文件类型失败: {e}"))?;
		if !meta.is_dir() {
			continue;
		}
		let name = entry.file_name().to_string_lossy().to_string();
		if name.starts_with('.') {
			continue;
		}
		names.push(name);
	}
	names.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
	Ok(names)
}

pub fn list_profiles(install_id: &str) -> Result<WtfProfileTree, String> {
	let install = resolve_install(install_id)?;
	let wtf = wtf_root_for(&install);
	let accounts_root = account_root(&wtf);
	let mut accounts = Vec::new();

	if accounts_root.is_dir() {
		for account_name in list_dir_names(&accounts_root)? {
			let account_path = accounts_root.join(&account_name);
			let mut characters = Vec::new();
			for realm_name in list_dir_names(&account_path)? {
				if is_skipped_account_dir(&realm_name) {
					continue;
				}
				let realm_path = account_path.join(&realm_name);
				for char_name in list_dir_names(&realm_path)? {
					if is_skipped_account_dir(&char_name) {
						continue;
					}
					let char_path = realm_path.join(&char_name);
					if looks_like_character_dir(&char_path) {
						characters.push(WtfCharacter {
							realm: realm_name.clone(),
							name: char_name,
						});
					}
				}
			}
			accounts.push(WtfAccount {
				name: account_name,
				characters,
			});
		}
	}

	Ok(WtfProfileTree {
		wtf_path: wtf.display().to_string(),
		accounts,
	})
}

pub fn delete_account(install_id: &str, account: &str) -> Result<(), String> {
	validate_segment("账号", account)?;
	let install = resolve_install(install_id)?;
	let wtf = wtf_root_for(&install);
	let accounts_root = account_root(&wtf);
	if !accounts_root.is_dir() {
		return Err(format!("未找到 WTF/Account 目录: {}", accounts_root.display()));
	}
	let target = accounts_root.join(account);
	if !target.is_dir() {
		return Err(format!("账号配置不存在: {account}"));
	}
	ensure_under(&target, &accounts_root)?;
	fs::remove_dir_all(&target)
		.map_err(|e| format!("删除账号配置失败 {account}: {e}"))?;
	log::info!("deleted WTF account {}", target.display());
	Ok(())
}

pub fn delete_character(
	install_id: &str,
	account: &str,
	realm: &str,
	character: &str,
) -> Result<(), String> {
	validate_segment("账号", account)?;
	validate_segment("服务器", realm)?;
	validate_segment("角色", character)?;
	let install = resolve_install(install_id)?;
	let wtf = wtf_root_for(&install);
	let accounts_root = account_root(&wtf);
	if !accounts_root.is_dir() {
		return Err(format!("未找到 WTF/Account 目录: {}", accounts_root.display()));
	}
	let target = accounts_root.join(account).join(realm).join(character);
	if !target.is_dir() {
		return Err(format!("角色配置不存在: {account}/{realm}/{character}"));
	}
	ensure_under(&target, &accounts_root)?;
	fs::remove_dir_all(&target)
		.map_err(|e| format!("删除角色配置失败 {account}/{realm}/{character}: {e}"))?;
	// Remove empty realm folder if nothing left.
	let realm_path = accounts_root.join(account).join(realm);
	if realm_path.is_dir() && list_dir_names(&realm_path)?.is_empty() {
		let _ = fs::remove_dir(&realm_path);
	}
	log::info!("deleted WTF character {}", target.display());
	Ok(())
}

fn should_skip_copy_file(path: &Path) -> bool {
	path.extension()
		.and_then(|ext| ext.to_str())
		.map(|ext| {
			let ext = ext.to_ascii_lowercase();
			ext == "bak" || ext == "old"
		})
		.unwrap_or(false)
}

fn copy_file(src: &Path, dst: &Path) -> Result<(), String> {
	if let Some(parent) = dst.parent() {
		fs::create_dir_all(parent).map_err(|e| format!("无法创建目录 {}: {e}", parent.display()))?;
	}
	fs::copy(src, dst).map_err(|e| {
		format!(
			"复制文件失败 {} → {}: {e}",
			src.display(),
			dst.display()
		)
	})?;
	Ok(())
}

/// Replace `dst` with a copy of `src` directory (only non-.bak/.old files).
fn copy_dir_overwrite(src: &Path, dst: &Path) -> Result<(), String> {
	if !src.is_dir() {
		return Err(format!("源目录不存在: {}", src.display()));
	}
	if dst.exists() {
		fs::remove_dir_all(dst).map_err(|e| format!("清理目标目录失败 {}: {e}", dst.display()))?;
	}
	fs::create_dir_all(dst).map_err(|e| format!("无法创建目录 {}: {e}", dst.display()))?;
	for entry in fs::read_dir(src).map_err(|e| format!("读取目录失败 {}: {e}", src.display()))? {
		let entry = entry.map_err(|e| format!("读取目录项失败: {e}"))?;
		let src_child = entry.path();
		let name = entry.file_name();
		let dst_child = dst.join(&name);
		let meta = entry
			.file_type()
			.map_err(|e| format!("读取文件类型失败: {e}"))?;
		if meta.is_dir() {
			copy_dir_overwrite(&src_child, &dst_child)?;
		} else if meta.is_file() {
			if should_skip_copy_file(&src_child) {
				continue;
			}
			copy_file(&src_child, &dst_child)?;
		}
	}
	Ok(())
}

fn copy_named_items(src_root: &Path, dst_root: &Path, items: &[&str]) -> Result<usize, String> {
	fs::create_dir_all(dst_root)
		.map_err(|e| format!("无法创建目标目录 {}: {e}", dst_root.display()))?;
	let mut copied = 0usize;
	for name in items {
		let src = src_root.join(name);
		if !src.exists() {
			continue;
		}
		let dst = dst_root.join(name);
		if src.is_dir() {
			copy_dir_overwrite(&src, &dst)?;
			copied += 1;
		} else if src.is_file() {
			if should_skip_copy_file(&src) {
				continue;
			}
			if dst.exists() {
				fs::remove_file(&dst)
					.map_err(|e| format!("清理目标文件失败 {}: {e}", dst.display()))?;
			}
			copy_file(&src, &dst)?;
			copied += 1;
		}
	}
	Ok(copied)
}

pub fn copy_account(install_id: &str, from: &WtfAccountRef, to: &WtfAccountRef) -> Result<usize, String> {
	validate_segment("账号", &from.account)?;
	validate_segment("账号", &to.account)?;
	if from.account.eq_ignore_ascii_case(&to.account) {
		return Err("源账号与目标账号相同".into());
	}
	let install = resolve_install(install_id)?;
	let wtf = wtf_root_for(&install);
	let accounts_root = account_root(&wtf);
	if !accounts_root.is_dir() {
		return Err(format!("未找到 WTF/Account 目录: {}", accounts_root.display()));
	}
	let src = accounts_root.join(&from.account);
	let dst = accounts_root.join(&to.account);
	if !src.is_dir() {
		return Err(format!("源账号不存在: {}", from.account));
	}
	if !dst.is_dir() {
		return Err(format!("目标账号不存在: {}", to.account));
	}
	ensure_under(&src, &accounts_root)?;
	ensure_under(&dst, &accounts_root)?;
	let n = copy_named_items(&src, &dst, ACCOUNT_COPY_ITEMS)?;
	if n == 0 {
		return Err("源账号没有可复制的配置项".into());
	}
	log::info!(
		"copied WTF account {} → {} ({n} items)",
		from.account,
		to.account
	);
	Ok(n)
}

pub fn copy_character(
	install_id: &str,
	from: &WtfCharacterRef,
	to: &WtfCharacterRef,
) -> Result<usize, String> {
	validate_segment("账号", &from.account)?;
	validate_segment("服务器", &from.realm)?;
	validate_segment("角色", &from.character)?;
	validate_segment("账号", &to.account)?;
	validate_segment("服务器", &to.realm)?;
	validate_segment("角色", &to.character)?;
	if from.account.eq_ignore_ascii_case(&to.account)
		&& from.realm.eq_ignore_ascii_case(&to.realm)
		&& from.character.eq_ignore_ascii_case(&to.character)
	{
		return Err("源角色与目标角色相同".into());
	}
	let install = resolve_install(install_id)?;
	let wtf = wtf_root_for(&install);
	let accounts_root = account_root(&wtf);
	if !accounts_root.is_dir() {
		return Err(format!("未找到 WTF/Account 目录: {}", accounts_root.display()));
	}
	let src = accounts_root
		.join(&from.account)
		.join(&from.realm)
		.join(&from.character);
	let dst = accounts_root
		.join(&to.account)
		.join(&to.realm)
		.join(&to.character);
	if !src.is_dir() {
		return Err(format!(
			"源角色不存在: {}/{}/{}",
			from.account, from.realm, from.character
		));
	}
	if !dst.is_dir() {
		return Err(format!(
			"目标角色不存在: {}/{}/{}",
			to.account, to.realm, to.character
		));
	}
	ensure_under(&src, &accounts_root)?;
	ensure_under(&dst, &accounts_root)?;
	let n = copy_named_items(&src, &dst, CHAR_COPY_ITEMS)?;
	if n == 0 {
		return Err("源角色没有可复制的配置项".into());
	}
	log::info!(
		"copied WTF character {}/{}/{} → {}/{}/{} ({n} items)",
		from.account,
		from.realm,
		from.character,
		to.account,
		to.realm,
		to.character
	);
	Ok(n)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Write;

	fn write_file(path: &Path, body: &str) {
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).unwrap();
		}
		let mut f = fs::File::create(path).unwrap();
		f.write_all(body.as_bytes()).unwrap();
	}

	#[test]
	fn validate_segment_rejects_traversal() {
		assert!(validate_segment("账号", "").is_err());
		assert!(validate_segment("账号", "../x").is_err());
		assert!(validate_segment("账号", "a/b").is_err());
		assert!(validate_segment("账号", "a\\b").is_err());
		assert!(validate_segment("账号", "Main").is_ok());
	}

	#[test]
	fn copy_named_items_character_scope_skips_bindings() {
		let root = std::env::temp_dir().join(format!(
			"wow-wtf-char-{}",
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		let src = root.join("src");
		let dst = root.join("dst");
		write_file(&src.join("SavedVariables/Foo.lua"), "Foo = {}\n");
		write_file(&src.join("SavedVariables/Foo.lua.bak"), "old\n");
		write_file(&src.join("layout-local.txt"), "layout\n");
		write_file(&src.join("bindings-cache.wtf"), "BINDINGS\n");
		write_file(&src.join("macros-cache.txt"), "MACRO\n");
		write_file(&dst.join("bindings-cache.wtf"), "KEEP\n");
		write_file(&dst.join("macros-cache.txt"), "KEEP_MACRO\n");

		let n = copy_named_items(&src, &dst, CHAR_COPY_ITEMS).unwrap();
		assert!(n >= 2);
		assert_eq!(
			fs::read_to_string(dst.join("SavedVariables/Foo.lua")).unwrap(),
			"Foo = {}\n"
		);
		assert!(!dst.join("SavedVariables/Foo.lua.bak").exists());
		assert_eq!(
			fs::read_to_string(dst.join("layout-local.txt")).unwrap(),
			"layout\n"
		);
		assert_eq!(
			fs::read_to_string(dst.join("bindings-cache.wtf")).unwrap(),
			"KEEP\n"
		);
		assert_eq!(
			fs::read_to_string(dst.join("macros-cache.txt")).unwrap(),
			"KEEP_MACRO\n"
		);
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn copy_named_items_account_scope_no_realms() {
		let root = std::env::temp_dir().join(format!(
			"wow-wtf-acc-{}",
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		let src = root.join("src");
		let dst = root.join("dst");
		write_file(&src.join("SavedVariables/Bar.lua"), "Bar = {}\n");
		write_file(&src.join("SavedVariables.lua"), "SV = {}\n");
		write_file(&src.join("config-cache.wtf"), "SET x \"1\"\n");
		write_file(
			&src.join("SomeRealm/SomeChar/layout-local.txt"),
			"should-not-copy\n",
		);
		fs::create_dir_all(&dst).unwrap();

		let n = copy_named_items(&src, &dst, ACCOUNT_COPY_ITEMS).unwrap();
		assert!(n >= 3);
		assert!(dst.join("SavedVariables/Bar.lua").is_file());
		assert!(dst.join("SavedVariables.lua").is_file());
		assert!(dst.join("config-cache.wtf").is_file());
		assert!(!dst.join("SomeRealm").exists());
		let _ = fs::remove_dir_all(&root);
	}

	#[test]
	fn list_profiles_from_disk_layout() {
		let root = std::env::temp_dir().join(format!(
			"wow-wtf-list-{}",
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		let account = root.join("WTF/Account/123456");
		write_file(
			&account.join("SavedVariables/Addon.lua"),
			"Addon = {}\n",
		);
		write_file(
			&account.join("RealmA/CharOne/layout-local.txt"),
			"layout\n",
		);
		write_file(
			&account.join("RealmA/CharTwo/SavedVariables/X.lua"),
			"X = {}\n",
		);
		// Empty dir under realm should not count as character.
		fs::create_dir_all(account.join("RealmA/EmptyFolder")).unwrap();

		let accounts_root = root.join("WTF/Account");
		let mut accounts = Vec::new();
		for account_name in list_dir_names(&accounts_root).unwrap() {
			let account_path = accounts_root.join(&account_name);
			let mut characters = Vec::new();
			for realm_name in list_dir_names(&account_path).unwrap() {
				if is_skipped_account_dir(&realm_name) {
					continue;
				}
				let realm_path = account_path.join(&realm_name);
				for char_name in list_dir_names(&realm_path).unwrap() {
					let char_path = realm_path.join(&char_name);
					if looks_like_character_dir(&char_path) {
						characters.push(WtfCharacter {
							realm: realm_name.clone(),
							name: char_name,
						});
					}
				}
			}
			accounts.push(WtfAccount {
				name: account_name,
				characters,
			});
		}
		assert_eq!(accounts.len(), 1);
		assert_eq!(accounts[0].name, "123456");
		assert_eq!(accounts[0].characters.len(), 2);
		let _ = fs::remove_dir_all(&root);
	}
}
