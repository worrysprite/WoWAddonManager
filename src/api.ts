import { Channel, invoke } from "@tauri-apps/api/core";
import type {
	AddonDetail,
	AddonJobProgress,
	AddonScanProgress,
	AddonSummary,
	ApiHelp,
	FilterCatalog,
	InstalledAddon,
	LogView,
	SearchParams,
	SearchResult,
	SettingsView,
	SyncReport,
	WtfAccountRef,
	WtfCharacterRef,
	WtfProfileTree,
	WowInstall,
} from "./types";

export function searchAddons(params: SearchParams, forceRefresh = false) {
	return invoke<SearchResult>("search_addons", { params, forceRefresh });
}

export function getAddonDetail(modId: number) {
	return invoke<AddonDetail>("get_addon_detail", { modId });
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

export function rescanInstalledAddons(
	installId: string,
	onProgress?: (progress: AddonScanProgress) => void,
) {
	const channel = new Channel<AddonScanProgress>();
	channel.onmessage = (progress) => {
		onProgress?.(progress);
	};
	return invoke<InstalledAddon[]>("rescan_installed_addons", {
		installId,
		onProgress: channel,
	});
}

/** Hourly CurseForge update check for one client. */
export function checkInstalledUpdates(installId: string) {
	return invoke<InstalledAddon[]>("check_installed_updates", { installId });
}

/** After disk scan: localize remote icons only (no update check). */
export function localizeInstalledIcons(installId: string) {
	return invoke<InstalledAddon[]>("localize_installed_icons", { installId });
}

export function installAddon(
	installId: string,
	addon: AddonSummary,
	onProgress?: (progress: AddonJobProgress) => void,
) {
	const channel = new Channel<AddonJobProgress>();
	channel.onmessage = (progress) => {
		onProgress?.(progress);
	};
	return invoke<InstalledAddon>("install_addon", {
		request: { installId, addon },
		onProgress: channel,
	});
}

export function updateAddon(
	installId: string,
	folderName: string,
	onProgress?: (progress: AddonJobProgress) => void,
) {
	const channel = new Channel<AddonJobProgress>();
	channel.onmessage = (progress) => {
		onProgress?.(progress);
	};
	return invoke<InstalledAddon>("update_addon", {
		request: { installId, folderName },
		onProgress: channel,
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
	installBackup?: { installId: string; backupPath: string };
}) {
	return invoke<SettingsView>("save_settings", {
		request: {
			curseforgeApiKey: options.curseforgeApiKey,
			wowRoots: options.wowRoots,
			locale: options.locale,
			installBackup: options.installBackup,
		},
	});
}

export function backupInstall(
	installId: string,
	onProgress?: (line: string) => void,
) {
	const channel = new Channel<string>();
	channel.onmessage = (line) => {
		onProgress?.(line);
	};
	return invoke<SyncReport>("backup_install", {
		installId,
		onProgress: channel,
	});
}

export function restoreInstall(
	installId: string,
	onProgress?: (line: string) => void,
) {
	const channel = new Channel<string>();
	channel.onmessage = (line) => {
		onProgress?.(line);
	};
	return invoke<SyncReport>("restore_install", {
		installId,
		onProgress: channel,
	});
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

export function listWtfProfiles(installId: string) {
	return invoke<WtfProfileTree>("list_wtf_profiles", { installId });
}

export function deleteWtfAccount(installId: string, account: string) {
	return invoke<void>("delete_wtf_account", {
		request: { installId, account },
	});
}

export function deleteWtfCharacter(
	installId: string,
	account: string,
	realm: string,
	character: string,
) {
	return invoke<void>("delete_wtf_character", {
		request: { installId, account, realm, character },
	});
}

export function copyWtfAccount(
	installId: string,
	from: WtfAccountRef,
	to: WtfAccountRef,
	items: string[],
) {
	return invoke<number>("copy_wtf_account", {
		request: { installId, from, to, items },
	});
}

export function copyWtfCharacter(
	installId: string,
	from: WtfCharacterRef,
	to: WtfCharacterRef,
	items: string[],
) {
	return invoke<number>("copy_wtf_character", {
		request: { installId, from, to, items },
	});
}

export function getAppVersion() {
	return invoke<string>("get_app_version");
}

export function getUpdateInstallKind() {
	return invoke<string>("get_update_install_kind");
}
