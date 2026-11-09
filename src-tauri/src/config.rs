use crate::types::WowInstall;
use crate::wow::detect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
	pub extra_wow_roots: Vec<String>,
	#[serde(default)]
	pub curseforge_api_key: Option<String>,
	/// UI language preference: `system`, `zh-CN`, or `en`.
	#[serde(default = "default_locale_preference")]
	pub locale: String,
	/// Per-install backup directories keyed by install id.
	#[serde(default)]
	pub backup_paths: HashMap<String, String>,
	/// Legacy global backup path from older builds; ignored after load.
	#[serde(default, skip_serializing)]
	pub backup_path: Option<String>,
}

fn default_locale_preference() -> String {
	"system".into()
}

impl Default for AppConfig {
	fn default() -> Self {
		Self {
			extra_wow_roots: Vec::new(),
			curseforge_api_key: None,
			locale: default_locale_preference(),
			backup_paths: HashMap::new(),
			backup_path: None,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
	pub curseforge_api_key: String,
	pub has_api_key: bool,
	pub apply_url: String,
	pub docs_url: String,
	pub wow_roots: Vec<String>,
	pub detected_installs: Vec<WowInstall>,
	pub invalid_wow_roots: Vec<String>,
	pub has_wow_install: bool,
	pub locale: String,
	pub system_locale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallBackupPathRequest {
	pub install_id: String,
	/// Empty string clears the path for this install.
	pub backup_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsRequest {
	/// `None` leaves the stored key unchanged; `Some("")` clears it.
	#[serde(default)]
	pub curseforge_api_key: Option<String>,
	#[serde(default)]
	pub wow_roots: Option<Vec<String>>,
	#[serde(default)]
	pub locale: Option<String>,
	#[serde(default)]
	pub install_backup: Option<InstallBackupPathRequest>,
}

fn config_path() -> Result<PathBuf, String> {
	let dir = dirs::config_dir()
		.ok_or_else(|| "无法定位配置目录".to_string())?
		.join("wow-addon-manager");
	fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
	Ok(dir.join("config.json"))
}

pub fn load() -> AppConfig {
	let Ok(path) = config_path() else {
		return AppConfig::default();
	};
	match fs::read_to_string(path) {
		Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
		Err(_) => AppConfig::default(),
	}
}

pub fn save(config: &AppConfig) -> Result<(), String> {
	let path = config_path()?;
	let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
	fs::write(path, content).map_err(|e| e.to_string())
}

pub fn add_wow_root(path: &str) -> Result<AppConfig, String> {
	let trimmed = path.trim();
	if trimmed.is_empty() {
		return Err("路径不能为空".into());
	}
	// Validate before saving.
	detect::install_from_path(trimmed)?;
	let mut config = load();
	if !config
		.extra_wow_roots
		.iter()
		.any(|p| p.eq_ignore_ascii_case(trimmed))
	{
		config.extra_wow_roots.push(trimmed.to_string());
		save(&config)?;
	}
	detect::invalidate_cache();
	Ok(config)
}

pub fn remove_wow_root(path: &str) -> Result<AppConfig, String> {
	let mut config = load();
	config
		.extra_wow_roots
		.retain(|p| !p.eq_ignore_ascii_case(path));
	save(&config)?;
	detect::invalidate_cache();
	Ok(config)
}

pub fn api_key() -> Option<String> {
	load()
		.curseforge_api_key
		.map(|k| k.trim().to_string())
		.filter(|k| !k.is_empty())
}

pub fn settings_view() -> SettingsView {
	settings_view_with_depth(false)
}

pub fn settings_view_deep() -> SettingsView {
	settings_view_with_depth(true)
}

fn normalize_locale_preference(raw: &str) -> String {
	match raw.trim().to_ascii_lowercase().as_str() {
		"" | "system" | "auto" => "system".into(),
		"zh" | "zh-cn" | "zh_cn" | "zh-hans" => "zh-CN".into(),
		"en" | "en-us" | "en_us" | "en-gb" => "en".into(),
		other if other.starts_with("zh") => "zh-CN".into(),
		_ => "en".into(),
	}
}

pub fn detect_system_locale() -> String {
	std::env::var("LANG")
		.ok()
		.or_else(|| std::env::var("LC_ALL").ok())
		.or_else(|| std::env::var("LC_MESSAGES").ok())
		.map(|v| {
			let code = v.split('.').next().unwrap_or(&v);
			normalize_locale_preference(code)
		})
		.filter(|v| v != "system")
		.unwrap_or_else(|| {
			#[cfg(windows)]
			{
				use std::os::windows::ffi::OsStringExt;
				#[link(name = "kernel32")]
				extern "system" {
					fn GetUserDefaultLocaleName(lp_locale_name: *mut u16, cch_locale_name: i32) -> i32;
				}
				let mut buf = [0u16; 85];
				let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
				if len > 1 {
					let raw = std::ffi::OsString::from_wide(&buf[..(len as usize - 1)])
						.to_string_lossy()
						.to_string();
					return normalize_locale_preference(&raw);
				}
			}
			"en".into()
		})
}

fn apply_backup_paths(mut installs: Vec<WowInstall>, paths: &HashMap<String, String>) -> Vec<WowInstall> {
	for install in &mut installs {
		install.backup_path = paths
			.get(&install.id)
			.map(|p| p.trim().to_string())
			.filter(|p| !p.is_empty())
			.unwrap_or_default();
	}
	installs
}

/// Copy legacy global `backupPath` onto every known install once, then drop it.
fn migrate_legacy_backup_path(config: &mut AppConfig, install_ids: &[String]) -> bool {
	let legacy = config
		.backup_path
		.as_deref()
		.map(str::trim)
		.filter(|p| !p.is_empty())
		.map(|p| p.to_string());
	let Some(legacy) = legacy else {
		return false;
	};
	if config.backup_paths.is_empty() {
		for id in install_ids {
			if id.trim().is_empty() {
				continue;
			}
			config
				.backup_paths
				.entry(id.clone())
				.or_insert_with(|| legacy.clone());
		}
	}
	config.backup_path = None;
	true
}

fn load_migrated(install_ids: &[String]) -> AppConfig {
	let mut config = load();
	if migrate_legacy_backup_path(&mut config, install_ids) {
		let _ = save(&config);
	}
	config
}

fn settings_view_with_depth(deep: bool) -> SettingsView {
	let preliminary = load();
	let report = if deep {
		detect::detect_with_extra_roots_deep(&preliminary.extra_wow_roots)
	} else {
		detect::detect_with_extra_roots(&preliminary.extra_wow_roots)
	};
	let install_ids: Vec<String> = report.installs.iter().map(|i| i.id.clone()).collect();
	let config = load_migrated(&install_ids);
	let key = config
		.curseforge_api_key
		.clone()
		.unwrap_or_default()
		.trim()
		.to_string();
	SettingsView {
		has_api_key: !key.is_empty(),
		curseforge_api_key: key,
		apply_url: "https://forms.monday.com/forms/dce5ccb7afda9a1c21dab1a1aa1d84eb?r=use1"
			.into(),
		docs_url: "https://docs.curseforge.com/rest-api/".into(),
		wow_roots: config.extra_wow_roots,
		has_wow_install: !report.installs.is_empty(),
		detected_installs: apply_backup_paths(report.installs, &config.backup_paths),
		invalid_wow_roots: report.invalid_roots,
		locale: normalize_locale_preference(&config.locale),
		system_locale: detect_system_locale(),
	}
}

pub fn backup_path_for_install(install_id: &str) -> Option<String> {
	let config = load_migrated(&[install_id.to_string()]);
	config
		.backup_paths
		.get(install_id)
		.map(|p| p.trim().to_string())
		.filter(|p| !p.is_empty())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn migrates_legacy_global_backup_to_each_install() {
		let mut config = AppConfig {
			backup_path: Some("D:/WoWBackup".into()),
			backup_paths: HashMap::new(),
			..AppConfig::default()
		};
		let changed = migrate_legacy_backup_path(
			&mut config,
			&["wow-aaa".into(), "wow-bbb".into()],
		);
		assert!(changed);
		assert!(config.backup_path.is_none());
		assert_eq!(
			config.backup_paths.get("wow-aaa").map(String::as_str),
			Some("D:/WoWBackup")
		);
		assert_eq!(
			config.backup_paths.get("wow-bbb").map(String::as_str),
			Some("D:/WoWBackup")
		);
	}

	#[test]
	fn keeps_existing_per_install_paths_when_migrating() {
		let mut config = AppConfig {
			backup_path: Some("D:/Legacy".into()),
			backup_paths: HashMap::from([("wow-aaa".into(), "E:/Keep".into())]),
			..AppConfig::default()
		};
		assert!(migrate_legacy_backup_path(
			&mut config,
			&["wow-aaa".into(), "wow-bbb".into()]
		));
		assert!(config.backup_path.is_none());
		assert_eq!(
			config.backup_paths.get("wow-aaa").map(String::as_str),
			Some("E:/Keep")
		);
		assert!(!config.backup_paths.contains_key("wow-bbb"));
	}
}

pub fn save_settings(request: SaveSettingsRequest) -> Result<SettingsView, String> {
	let mut config = load();
	let mut api_key_changed = false;
	if let Some(raw_key) = request.curseforge_api_key {
		let trimmed = raw_key.trim().to_string();
		let next = if trimmed.is_empty() {
			None
		} else {
			Some(trimmed)
		};
		api_key_changed = config.curseforge_api_key != next;
		config.curseforge_api_key = next;
	}
	if let Some(roots) = request.wow_roots {
		let mut cleaned = Vec::new();
		for root in roots {
			let root = root.trim().to_string();
			if root.is_empty() {
				continue;
			}
			if cleaned.iter().any(|p: &String| p.eq_ignore_ascii_case(&root)) {
				continue;
			}
			cleaned.push(root);
		}
		config.extra_wow_roots = cleaned;
	}
	if let Some(locale) = request.locale {
		config.locale = normalize_locale_preference(&locale);
	}
	if let Some(patch) = request.install_backup {
		let install_id = patch.install_id.trim().to_string();
		if install_id.is_empty() {
			return Err("缺少客户端 id".into());
		}
		let trimmed = patch.backup_path.trim().to_string();
		if trimmed.is_empty() {
			config.backup_paths.remove(&install_id);
		} else {
			config.backup_paths.insert(install_id, trimmed);
		}
	}
	// Drop deprecated global field so it is not written back.
	config.backup_path = None;
	save(&config)?;
	detect::invalidate_cache();
	if api_key_changed {
		crate::curseforge::browse_cache::invalidate();
	}
	Ok(settings_view())
}
