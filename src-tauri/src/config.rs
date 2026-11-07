use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
	pub extra_wow_roots: Vec<String>,
	#[serde(default)]
	pub curseforge_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
	pub curseforge_api_key: String,
	pub has_api_key: bool,
	pub apply_url: String,
	pub docs_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsRequest {
	pub curseforge_api_key: String,
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
	let mut config = load();
	if !config.extra_wow_roots.iter().any(|p| p == path) {
		config.extra_wow_roots.push(path.to_string());
		save(&config)?;
	}
	Ok(config)
}

pub fn api_key() -> Option<String> {
	load()
		.curseforge_api_key
		.map(|k| k.trim().to_string())
		.filter(|k| !k.is_empty())
}

pub fn settings_view() -> SettingsView {
	let key = api_key().unwrap_or_default();
	SettingsView {
		has_api_key: !key.is_empty(),
		curseforge_api_key: key,
		apply_url: "https://forms.monday.com/forms/dce5ccb7afda9a1c21dab1a1aa1d84eb?r=use1"
			.into(),
		docs_url: "https://docs.curseforge.com/rest-api/".into(),
	}
}

pub fn save_settings(request: SaveSettingsRequest) -> Result<SettingsView, String> {
	let mut config = load();
	let trimmed = request.curseforge_api_key.trim().to_string();
	config.curseforge_api_key = if trimmed.is_empty() {
		None
	} else {
		Some(trimmed)
	};
	save(&config)?;
	Ok(settings_view())
}
