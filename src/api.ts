import { invoke } from "@tauri-apps/api/core";
import type {
	AddonSummary,
	ApiHelp,
	FilterCatalog,
	InstalledAddon,
	LogView,
	SearchParams,
	SearchResult,
	SettingsView,
	WowInstall,
} from "./types";

export function searchAddons(params: SearchParams, forceRefresh = false) {
	return invoke<SearchResult>("search_addons", { params, forceRefresh });
}

export function getFilterCatalog() {
	return invoke<FilterCatalog>("get_filter_catalog");
}

export function detectWowInstalls() {
	return invoke<WowInstall[]>("detect_wow_installs");
}

export function addWowPath(path: string) {
	return invoke<SettingsView>("add_wow_path", { path });
}

export function removeWowPath(path: string) {
	return invoke<SettingsView>("remove_wow_path", { path });
}

export function rescanWowInstalls() {
	return invoke<SettingsView>("rescan_wow_installs");
}

export function listInstalledAddons(installId: string) {
	return invoke<InstalledAddon[]>("list_installed_addons", { installId });
}

export function rescanInstalledAddons(installId: string) {
	return invoke<InstalledAddon[]>("rescan_installed_addons", { installId });
}

export function installAddon(installId: string, addon: AddonSummary) {
	return invoke<InstalledAddon>("install_addon", {
		request: { installId, addon },
	});
}

export function updateAddon(installId: string, folderName: string) {
	return invoke<InstalledAddon>("update_addon", {
		request: { installId, folderName },
	});
}

export function deleteAddon(installId: string, folderName: string) {
	return invoke<void>("delete_addon", {
		request: { installId, folderName },
	});
}

export function installFromArchive(installId: string, archivePath: string) {
	return invoke<InstalledAddon[]>("install_from_archive", {
		request: { installId, archivePath },
	});
}

export function getApiHelp() {
	return invoke<ApiHelp>("get_api_help");
}

export function getSettings() {
	return invoke<SettingsView>("get_settings");
}

export function saveSettings(options: {
	curseforgeApiKey?: string;
	wowRoots?: string[];
	locale?: string;
	backupPath?: string;
}) {
	return invoke<SettingsView>("save_settings", {
		request: {
			curseforgeApiKey: options.curseforgeApiKey,
			wowRoots: options.wowRoots,
			locale: options.locale,
			backupPath: options.backupPath,
		},
	});
}

export function backupInstall(installId: string) {
	return invoke<string>("backup_install", { installId });
}

export function getLogs() {
	return invoke<LogView>("get_logs");
}

export function setLogLevel(level: string) {
	return invoke<string>("set_log_level", { level });
}

export function clearLogs() {
	return invoke<void>("clear_logs");
}

export function saveLogs(path: string) {
	return invoke<void>("save_logs", { path });
}

export function openDir(path: string) {
	return invoke<void>("open_dir", { path });
}
