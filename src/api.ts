import { invoke } from "@tauri-apps/api/core";
import type {
	AddonSummary,
	ApiHelp,
	FilterCatalog,
	InstalledAddon,
	SearchParams,
	SearchResult,
	SettingsView,
	WowInstall,
} from "./types";

export function searchAddons(params: SearchParams) {
	return invoke<SearchResult>("search_addons", { params });
}

export function getFilterCatalog() {
	return invoke<FilterCatalog>("get_filter_catalog");
}

export function detectWowInstalls() {
	return invoke<WowInstall[]>("detect_wow_installs");
}

export function addWowPath(path: string) {
	return invoke<WowInstall[]>("add_wow_path", { path });
}

export function listInstalledAddons(installId: string) {
	return invoke<InstalledAddon[]>("list_installed_addons", { installId });
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

export function getApiHelp() {
	return invoke<ApiHelp>("get_api_help");
}

export function getSettings() {
	return invoke<SettingsView>("get_settings");
}

export function saveSettings(curseforgeApiKey: string) {
	return invoke<SettingsView>("save_settings", {
		request: { curseforgeApiKey },
	});
}
