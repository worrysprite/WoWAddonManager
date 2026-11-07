use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
	pub extra_wow_roots: Vec<String>,
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
