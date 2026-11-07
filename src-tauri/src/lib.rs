mod config;
mod curseforge;
mod types;
mod wow;

use config::{SaveSettingsRequest, SettingsView};
use curseforge::catalog;
use types::{
	DeleteRequest, FilterCatalog, InstallRequest, InstalledAddon, ManualInstallRequest,
	SearchParams, SearchResult, UpdateRequest, WowInstall,
};
use wow::{detect, install, scan};

#[tauri::command]
async fn search_addons(params: SearchParams) -> Result<SearchResult, String> {
	Ok(curseforge::search_addons(params).await)
}

#[tauri::command]
fn get_filter_catalog() -> FilterCatalog {
	catalog::filter_catalog()
}

#[tauri::command]
fn detect_wow_installs() -> Result<Vec<WowInstall>, String> {
	let cfg = config::load();
	let report = detect::detect_with_extra_roots(&cfg.extra_wow_roots);
	Ok(report.installs)
}

#[tauri::command]
fn add_wow_path(path: String) -> Result<SettingsView, String> {
	config::add_wow_root(&path)?;
	Ok(config::settings_view())
}

#[tauri::command]
fn remove_wow_path(path: String) -> Result<SettingsView, String> {
	config::remove_wow_root(&path)?;
	Ok(config::settings_view())
}

#[tauri::command]
fn rescan_wow_installs() -> Result<SettingsView, String> {
	Ok(config::settings_view())
}

#[tauri::command]
async fn list_installed_addons(install_id: String) -> Result<Vec<InstalledAddon>, String> {
	let installs = detect_wow_installs()?;
	let install = install::find_install_in(&installs, &install_id)?;
	scan::list_installed(&install).await
}

#[tauri::command]
async fn install_addon(request: InstallRequest) -> Result<InstalledAddon, String> {
	install::install_addon(&request.install_id, request.addon).await
}

#[tauri::command]
async fn update_addon(request: UpdateRequest) -> Result<InstalledAddon, String> {
	install::update_addon(&request.install_id, &request.folder_name).await
}

#[tauri::command]
async fn delete_addon(request: DeleteRequest) -> Result<(), String> {
	install::delete_addon(&request.install_id, &request.folder_name).await
}

#[tauri::command]
async fn install_from_archive(request: ManualInstallRequest) -> Result<Vec<InstalledAddon>, String> {
	install::install_from_archive(&request.install_id, &request.archive_path).await
}

#[tauri::command]
fn get_settings() -> SettingsView {
	config::settings_view()
}

#[tauri::command]
fn save_settings(request: SaveSettingsRequest) -> Result<SettingsView, String> {
	config::save_settings(request)
}

#[tauri::command]
fn get_api_help() -> serde_json::Value {
	serde_json::json!({
		"docs": "https://docs.curseforge.com/rest-api/",
		"apply": "https://forms.monday.com/forms/dce5ccb7afda9a1c21dab1a1aa1d84eb?r=use1",
		"console": "https://console.curseforge.com/",
		"note": "第三方开发者请通过申请表获取 x-api-key，不要用 Studios Console 随意注册。"
	})
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.plugin(tauri_plugin_dialog::init())
		.invoke_handler(tauri::generate_handler![
			search_addons,
			get_filter_catalog,
			detect_wow_installs,
			add_wow_path,
			remove_wow_path,
			rescan_wow_installs,
			list_installed_addons,
			install_addon,
			install_from_archive,
			update_addon,
			delete_addon,
			get_settings,
			save_settings,
			get_api_help
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
