use crate::types::WowInstall;
use crate::wow::detect;
use serde::{Deserialize, Serialize};
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
pub struct SaveSettingsRequest {
	/// `None` leaves the stored key unchanged; `Some("")` clears it.
	#[serde(default)]
	pub curseforge_api_key: Option<String>,
	#[serde(default)]
	pub wow_roots: Option<Vec<String>>,
	#[serde(default)]
	pub locale: Option<String>,
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

fn settings_view_with_depth(deep: bool) -> SettingsView {
	let config = load();
	let key = config
		.curseforge_api_key
		.clone()
		.unwrap_or_default()
		.trim()
		.to_string();
	let report = if deep {
		detect::detect_with_extra_roots_deep(&config.extra_wow_roots)
	} else {
		detect::detect_with_extra_roots(&config.extra_wow_roots)
	};
	SettingsView {
		has_api_key: !key.is_empty(),
		curseforge_api_key: key,
		apply_url: "https://forms.monday.com/forms/dce5ccb7afda9a1c21dab1a1aa1d84eb?r=use1"
			.into(),
		docs_url: "https://docs.curseforge.com/rest-api/".into(),
		wow_roots: config.extra_wow_roots,
		has_wow_install: !report.installs.is_empty(),
		detected_installs: report.installs,
		invalid_wow_roots: report.invalid_roots,
		locale: normalize_locale_preference(&config.locale),
		system_locale: detect_system_locale(),
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
	save(&config)?;
	detect::invalidate_cache();
	if api_key_changed {
		crate::curseforge::browse_cache::invalidate();
	}
	Ok(settings_view())
}
