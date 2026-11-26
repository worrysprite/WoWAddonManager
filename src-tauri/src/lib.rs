mod app_log;
mod config;
mod curseforge;
mod icon_cache;
mod types;
mod wow;

use app_log::LogView;
use config::{SaveSettingsRequest, SettingsView};
use curseforge::catalog;
use types::{
	AddonDetail, AddonJobProgress, AddonScanProgress, DeleteRequest, FilterCatalog, InstallRequest,
	InstalledAddon, ManualInstallRequest, SearchParams, SearchResult, UpdateRequest, WowInstall,
};
use std::sync::Arc;
use wow::backup::SyncReport;
use wow::wtf::{
	WtfCopyAccountRequest, WtfCopyCharacterRequest, WtfDeleteAccountRequest,
	WtfDeleteCharacterRequest, WtfProfileTree,
};
use wow::{backup, detect, install, scan, wtf};

async fn run_blocking<T, F>(f: F) -> Result<T, String>
where
	T: Send + 'static,
	F: FnOnce() -> Result<T, String> + Send + 'static,
{
	tokio::task::spawn_blocking(f)
		.await
		.map_err(|e| format!("后台任务失败: {e}"))?
}

#[tauri::command]
async fn search_addons(params: SearchParams, force_refresh: Option<bool>) -> Result<SearchResult, String> {
	let force = force_refresh.unwrap_or(false);
	log::info!(
		"search_addons force={force} search={:?} page={:?}",
		params.search,
		params.page
	);
	let result = curseforge::search_addons(params, force).await;
	log::info!(
		"search_addons done source={} total={} items={}",
		result.source,
		result.total,
		result.items.len()
	);
	Ok(result)
}

#[tauri::command]
fn get_filter_catalog() -> FilterCatalog {
	catalog::filter_catalog()
}

#[tauri::command]
async fn get_addon_detail(mod_id: i64) -> Result<AddonDetail, String> {
	log::info!("get_addon_detail mod_id={mod_id}");
	curseforge::get_addon_detail(mod_id).await
}

#[tauri::command]
async fn detect_wow_installs() -> Result<Vec<WowInstall>, String> {
	run_blocking(|| {
		let cfg = config::load();
		Ok(detect::detect_with_extra_roots(&cfg.extra_wow_roots).installs)
	})
	.await
}

#[tauri::command]
async fn add_wow_path(path: String) -> Result<SettingsView, String> {
	run_blocking(move || {
		config::add_wow_root(&path)?;
		Ok(config::settings_view())
	})
	.await
}

#[tauri::command]
async fn remove_wow_path(path: String) -> Result<SettingsView, String> {
	run_blocking(move || {
		config::remove_wow_root(&path)?;
		Ok(config::settings_view())
	})
	.await
}

#[tauri::command]
async fn rescan_wow_installs() -> Result<SettingsView, String> {
	log::warn!("rescan_wow_installs (deep)");
	run_blocking(|| {
		detect::invalidate_cache();
		Ok(config::settings_view_deep())
	})
	.await
}

#[tauri::command]
async fn list_installed_addons(install_id: String) -> Result<Vec<InstalledAddon>, String> {
	log::info!("list_installed_addons install_id={install_id}");
	let install = resolve_install_async(install_id.clone()).await?;
	let list = scan::list_installed(&install, false).await?;
	log::info!(
		"list_installed_addons done install={} count={}",
		install.label,
		list.len()
	);
	Ok(list)
}

#[tauri::command]
async fn rescan_installed_addons(
	install_id: String,
	on_progress: tauri::ipc::Channel<AddonScanProgress>,
) -> Result<Vec<InstalledAddon>, String> {
	log::warn!("rescan_installed_addons install_id={install_id}");
	let install = resolve_install_async(install_id.clone()).await?;
	let progress = Arc::new(move |p: AddonScanProgress| {
		let _ = on_progress.send(p);
	});
	let list = scan::list_installed_with_progress(&install, true, Some(progress)).await?;
	log::warn!(
		"rescan_installed_addons done install={} count={}",
		install.label,
		list.len()
	);
	Ok(list)
}

/// Hourly CurseForge update check for one client install.
#[tauri::command]
async fn check_installed_updates(install_id: String) -> Result<Vec<InstalledAddon>, String> {
	log::info!("check_installed_updates install_id={install_id}");
	let install = resolve_install_async(install_id.clone()).await?;
	let list = scan::check_installed_updates(&install).await?;
	log::info!(
		"check_installed_updates done install={} count={}",
		install.label,
		list.len()
	);
	Ok(list)
}

/// After disk scan: localize remote icons only (no update check).
#[tauri::command]
async fn localize_installed_icons(install_id: String) -> Result<Vec<InstalledAddon>, String> {
	log::info!("localize_installed_icons install_id={install_id}");
	let install = resolve_install_async(install_id.clone()).await?;
	let list = scan::localize_installed_icons(&install).await?;
	log::info!(
		"localize_installed_icons done install={} count={}",
		install.label,
		list.len()
	);
	Ok(list)
}

async fn resolve_install_async(install_id: String) -> Result<WowInstall, String> {
	run_blocking(move || {
		let cfg = config::load();
		let report = detect::detect_with_extra_roots(&cfg.extra_wow_roots);
		install::find_install_in(&report.installs, &install_id)
	})
	.await
}

#[tauri::command]
async fn install_addon(
	request: InstallRequest,
	on_progress: tauri::ipc::Channel<AddonJobProgress>,
) -> Result<InstalledAddon, String> {
	let progress = Arc::new(move |p: AddonJobProgress| {
		let _ = on_progress.send(p);
	});
	install::install_addon_with_progress(
		&request.install_id,
		request.addon,
		Some(progress),
	)
	.await
}

#[tauri::command]
async fn update_addon(
	request: UpdateRequest,
	on_progress: tauri::ipc::Channel<AddonJobProgress>,
) -> Result<InstalledAddon, String> {
	let progress = Arc::new(move |p: AddonJobProgress| {
		let _ = on_progress.send(p);
	});
	install::update_addon_with_progress(
		&request.install_id,
		&request.folder_name,
		Some(progress),
	)
	.await
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
async fn get_settings() -> Result<SettingsView, String> {
	log::info!("get_settings");
	run_blocking(|| Ok(config::settings_view())).await
}

#[tauri::command]
async fn save_settings(request: SaveSettingsRequest) -> Result<SettingsView, String> {
	log::warn!("save_settings");
	run_blocking(move || config::save_settings(request)).await
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

#[tauri::command]
fn get_logs() -> LogView {
	app_log::view()
}

#[tauri::command]
fn set_log_level(level: String) -> Result<String, String> {
	app_log::set_level(&level)
}

#[tauri::command]
fn clear_logs() {
	app_log::clear();
}

#[tauri::command]
fn save_logs(path: String) -> Result<(), String> {
	app_log::save_to_path(&path)
}

#[tauri::command]
async fn backup_install(
	install_id: String,
	on_progress: tauri::ipc::Channel<String>,
) -> Result<SyncReport, String> {
	log::info!("backup_install install_id={install_id}");
	run_blocking(move || {
		backup::backup_install(&install_id, |line| {
			let _ = on_progress.send(line.to_string());
		})
	})
	.await
}

#[tauri::command]
async fn restore_install(
	install_id: String,
	on_progress: tauri::ipc::Channel<String>,
) -> Result<SyncReport, String> {
	log::info!("restore_install install_id={install_id}");
	run_blocking(move || {
		backup::restore_install(&install_id, |line| {
			let _ = on_progress.send(line.to_string());
		})
	})
	.await
}

#[tauri::command]
async fn list_wtf_profiles(install_id: String) -> Result<WtfProfileTree, String> {
	log::info!("list_wtf_profiles install_id={install_id}");
	run_blocking(move || wtf::list_profiles(&install_id)).await
}

#[tauri::command]
async fn delete_wtf_account(request: WtfDeleteAccountRequest) -> Result<(), String> {
	log::warn!(
		"delete_wtf_account install_id={} account={}",
		request.install_id,
		request.account
	);
	run_blocking(move || wtf::delete_account(&request.install_id, &request.account)).await
}

#[tauri::command]
async fn delete_wtf_character(request: WtfDeleteCharacterRequest) -> Result<(), String> {
	log::warn!(
		"delete_wtf_character install_id={} account={} realm={} character={}",
		request.install_id,
		request.account,
		request.realm,
		request.character
	);
	run_blocking(move || {
		wtf::delete_character(
			&request.install_id,
			&request.account,
			&request.realm,
			&request.character,
		)
	})
	.await
}

#[tauri::command]
async fn copy_wtf_account(request: WtfCopyAccountRequest) -> Result<u32, String> {
	log::info!(
		"copy_wtf_account install_id={} from={} to={}",
		request.install_id,
		request.from.account,
		request.to.account
	);
	run_blocking(move || {
		wtf::copy_account(&request.install_id, &request.from, &request.to).map(|n| n as u32)
	})
	.await
}

#[tauri::command]
async fn copy_wtf_character(request: WtfCopyCharacterRequest) -> Result<u32, String> {
	log::info!(
		"copy_wtf_character install_id={} from={}/{}/{} to={}/{}/{}",
		request.install_id,
		request.from.account,
		request.from.realm,
		request.from.character,
		request.to.account,
		request.to.realm,
		request.to.character
	);
	run_blocking(move || {
		wtf::copy_character(&request.install_id, &request.from, &request.to).map(|n| n as u32)
	})
	.await
}

#[tauri::command]
fn open_dir(path: String) -> Result<(), String> {
	use std::path::PathBuf;
	use std::process::Command;

	let dir = PathBuf::from(&path);
	std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建目录: {e}"))?;
	log::info!("open_dir {}", dir.display());

	#[cfg(target_os = "windows")]
	{
		Command::new("explorer")
			.arg(dir.as_os_str())
			.spawn()
			.map_err(|e| format!("无法打开目录: {e}"))?;
		return Ok(());
	}

	#[cfg(target_os = "macos")]
	{
		Command::new("open")
			.arg(dir.as_os_str())
			.spawn()
			.map_err(|e| format!("无法打开目录: {e}"))?;
		return Ok(());
	}

	#[cfg(all(unix, not(target_os = "macos")))]
	{
		Command::new("xdg-open")
			.arg(dir.as_os_str())
			.spawn()
			.map_err(|e| format!("无法打开目录: {e}"))?;
		return Ok(());
	}

	#[allow(unreachable_code)]
	Err("当前平台不支持打开目录".into())
}

#[tauri::command]
fn get_app_version() -> String {
	env!("CARGO_PKG_VERSION").to_string()
}

/// Heuristic: NSIS installs usually live under Local AppData product folder / Program Files.
/// Do not match the executable file name alone — portable builds are also named
/// `WoW-Addon-Manager.exe` / similar and must stay on the manual-download path.
#[tauri::command]
fn get_update_install_kind() -> String {
	let Ok(exe) = std::env::current_exe() else {
		return "portable".into();
	};
	let path = exe.to_string_lossy().to_lowercase().replace('\\', "/");
	let parent = exe
		.parent()
		.map(|p| p.to_string_lossy().to_lowercase().replace('\\', "/"))
		.unwrap_or_default();
	let installed = parent.contains("program files")
		|| parent.contains("/appdata/local/wow addon manager")
		|| parent.contains("/appdata/roaming/wow addon manager")
		|| parent.contains("com.worrysprite.wowaddonmanager")
		|| path.contains("/appdata/local/wow addon manager/")
		|| path.contains("/appdata/roaming/wow addon manager/");
	if installed {
		"installed".into()
	} else {
		"portable".into()
	}
}

/// Default window size: 70% of monitor width × 80% of monitor height.
fn apply_default_window_size(window: &tauri::WebviewWindow) {
	let monitor = match window.current_monitor() {
		Ok(Some(monitor)) => monitor,
		_ => match window.primary_monitor() {
			Ok(Some(monitor)) => monitor,
			_ => return,
		},
	};
	let size = monitor.size();
	let width = ((size.width as f64) * 0.70).round().max(1.0) as u32;
	let height = ((size.height as f64) * 0.80).round().max(1.0) as u32;
	if let Err(err) = window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height })) {
		log::warn!("failed to set default window size: {err}");
		return;
	}
	if let Err(err) = window.center() {
		log::warn!("failed to center window: {err}");
	}
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	app_log::init();
	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.plugin(tauri_plugin_dialog::init())
		.plugin(tauri_plugin_process::init())
		.plugin(tauri_plugin_updater::Builder::new().build())
		.setup(|app| {
			use tauri::Manager;
			if let Some(window) = app.get_webview_window("main") {
				apply_default_window_size(&window);
			}
			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			search_addons,
			get_filter_catalog,
			get_addon_detail,
			detect_wow_installs,
			add_wow_path,
			remove_wow_path,
			rescan_wow_installs,
			list_installed_addons,
			rescan_installed_addons,
			check_installed_updates,
			localize_installed_icons,
			install_addon,
			install_from_archive,
			update_addon,
			delete_addon,
			get_settings,
			save_settings,
			get_api_help,
			get_logs,
			set_log_level,
			clear_logs,
			save_logs,
			backup_install,
			restore_install,
			list_wtf_profiles,
			delete_wtf_account,
			delete_wtf_character,
			copy_wtf_account,
			copy_wtf_character,
			open_dir,
			get_app_version,
			get_update_install_kind
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
