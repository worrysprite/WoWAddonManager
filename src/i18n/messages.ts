export type LocaleCode = "en" | "zh-CN";
export type LocalePreference = "system" | LocaleCode;

export type MessageKey =
	| "app.brand"
	| "nav.browse"
	| "nav.settings"
	| "banner.noApiKey"
	| "banner.goSettings"
	| "banner.fillKey"
	| "banner.applyForm"
	| "banner.docs"
	| "banner.apiEnabled"
	| "banner.noWow"
	| "banner.addWowHint"
	| "banner.invalidWow"
	| "banner.fixOrRemove"
	| "banner.rescanningClients"
	| "banner.rescanClientsFound"
	| "banner.rescanClientsNone"
	| "banner.addonInstalled"
	| "browse.searchPlaceholder"
	| "browse.searchAria"
	| "browse.loading"
	| "browse.loadingList"
	| "browse.noMatchTitle"
	| "browse.noMatchHint"
	| "browse.prevPage"
	| "browse.nextPage"
	| "browse.pageInfo"
	| "browse.results"
	| "browse.attributionBefore"
	| "browse.attributionAfter"
	| "filters.title"
	| "filters.clearAll"
	| "filters.categories"
	| "filters.flavor"
	| "card.by"
	| "card.unknownAuthor"
	| "card.noDescription"
	| "card.needWowPath"
	| "card.noCompatibleInstall"
	| "card.installTo"
	| "card.versionInstalled"
	| "card.updateToVersion"
	| "card.openDetail"
	| "detail.title"
	| "detail.close"
	| "detail.loading"
	| "detail.screenshots"
	| "detail.description"
	| "detail.noDescription"
	| "apiDisabled.title"
	| "apiDisabled.body"
	| "apiDisabled.hint"
	| "apiDisabled.openCurseForge"
	| "apiDisabled.close"
	| "card.installedAddon"
	| "card.installedAt"
	| "card.update"
	| "card.delete"
	| "card.updatingDownload"
	| "card.updatingDownloadPct"
	| "card.updatingExtract"
	| "card.updatingInstall"
	| "card.suiteFolders"
	| "installed.title"
	| "installed.updateAll"
	| "installed.updateAllRemaining"
	| "installed.manual"
	| "installed.rescan"
	| "installed.openInterface"
	| "installed.scanning"
	| "installed.rescanning"
	| "installed.scanFound"
	| "installed.loadingList"
	| "installed.emptyTitle"
	| "installed.emptyHint"
	| "installed.rescanDone"
	| "installed.manualDone"
	| "installed.manualInstalling"
	| "installed.manualPartialFail"
	| "installed.deleteConfirm"
	| "installed.deleteSuiteConfirm"
	| "installed.deleteTitle"
	| "installed.deleteOk"
	| "installed.deleteCancel"
	| "installed.pickArchive"
	| "installed.archiveFilter"
	| "installed.clientMissing"
	| "installed.backup"
	| "installed.backingUp"
	| "installed.backupDone"
	| "installed.backupNeedPath"
	| "installed.restore"
	| "installed.restoring"
	| "installed.restoreDone"
	| "installed.restoreConfirm"
	| "installed.restoreTitle"
	| "installed.restoreOk"
	| "installed.restoreCancel"
	| "installed.restoreMissing"
	| "installed.syncBackupTitle"
	| "installed.syncRestoreTitle"
	| "installed.syncRunning"
	| "installed.syncDone"
	| "installed.syncFailed"
	| "installed.syncItems"
	| "installed.syncClose"
	| "installed.syncOutput"
	| "installed.subtabs"
	| "installed.tabAddons"
	| "installed.tabWtf"
	| "wtf.hint"
	| "wtf.refresh"
	| "wtf.refreshing"
	| "wtf.openFolder"
	| "wtf.loading"
	| "wtf.emptyTitle"
	| "wtf.emptyHint"
	| "wtf.accountMeta"
	| "wtf.noCharacters"
	| "wtf.copy"
	| "wtf.delete"
	| "wtf.deleting"
	| "wtf.deleteOk"
	| "wtf.deleteCancel"
	| "wtf.deleteAccountTitle"
	| "wtf.deleteAccountConfirm"
	| "wtf.deleteAccountDone"
	| "wtf.deleteCharacterTitle"
	| "wtf.deleteCharacterConfirm"
	| "wtf.deleteCharacterDone"
	| "wtf.copyTitle"
	| "wtf.copyOk"
	| "wtf.copyCancel"
	| "wtf.copying"
	| "wtf.copyDest"
	| "wtf.copyDestPlaceholder"
	| "wtf.copyAccountFrom"
	| "wtf.copyCharacterFrom"
	| "wtf.copyItems"
	| "wtf.copyItemSavedVariables"
	| "wtf.copyItemAddOns"
	| "wtf.copyItemLayout"
	| "wtf.copyItemConfigCache"
	| "wtf.copyItemChatCache"
	| "wtf.copyItemBindings"
	| "wtf.copyItemMacros"
	| "wtf.copyItemEditMode"
	| "wtf.copyAccountConfirm"
	| "wtf.copyCharacterConfirm"
	| "wtf.copyAccountDone"
	| "wtf.copyCharacterDone"
	| "settings.back"
	| "settings.title"
	| "settings.subtitle"
	| "settings.language"
	| "settings.languageHint"
	| "settings.languageSystem"
	| "settings.languageZh"
	| "settings.languageEn"
	| "settings.languageSaved"
	| "settings.apiTitle"
	| "settings.apiKey"
	| "settings.apiPlaceholder"
	| "settings.apiHint"
	| "settings.apiEnabled"
	| "settings.apiDisabled"
	| "settings.applyKey"
	| "settings.apiDocs"
	| "settings.saveApiKey"
	| "settings.saving"
	| "settings.clearInput"
	| "settings.apiSaved"
	| "settings.apiCleared"
	| "settings.wowTitle"
	| "settings.wowHint"
	| "settings.addWowPath"
	| "settings.rescan"
	| "settings.pickWowDir"
	| "settings.pathAdded"
	| "settings.pathRemoved"
	| "settings.detectedClients"
	| "settings.versionUnknown"
	| "settings.noClients"
	| "settings.manualPaths"
	| "settings.remove"
	| "settings.invalidPaths"
	| "settings.rescanDoneFound"
	| "settings.rescanDoneNone"
	| "settings.backupPath"
	| "settings.backupPathPlaceholder"
	| "settings.pickBackupDir"
	| "settings.chooseBackupPath"
	| "settings.clearBackupPath"
	| "settings.backupPathSaved"
	| "settings.backupPathCleared"
	| "settings.backupPathUnset"
	| "settings.logsTitle"
	| "settings.logsHint"
	| "settings.openLogs"
	| "settings.closeLogs"
	| "settings.saveLogs"
	| "settings.clearLogs"
	| "settings.logLevel"
	| "settings.noLogs"
	| "settings.logLevelSet"
	| "settings.logsCleared"
	| "settings.logsSaved"
	| "settings.saveLogTitle"
	| "update.title"
	| "update.current"
	| "update.checking"
	| "update.upToDate"
	| "update.downloading"
	| "update.ready"
	| "update.readyAction"
	| "update.availableManual"
	| "update.openDownload"
	| "update.openWebsite"
	| "update.checkNow"
	| "update.installRestart"
	| "update.installing"
	| "update.portableHint"
	| "update.error"
	| "update.notes";

export type Messages = Record<MessageKey, string>;

export const en: Messages = {
	"app.brand": "WoW Addon Manager",
	"nav.browse": "Browse All",
	"nav.settings": "Settings",
	"banner.noApiKey": "No API Key configured. Using web GET. Go to",
	"banner.goSettings": "Settings",
	"banner.fillKey": "to enter a Key and switch to the official API.",
	"banner.applyForm": "Application form",
	"banner.docs": "Docs",
	"banner.apiEnabled": "CurseForge official API is enabled.",
	"banner.noWow": "No World of Warcraft install found. Go to",
	"banner.addWowHint":
		'and add a "World of Warcraft" root folder, or open Settings to scan clients.',
	"banner.invalidWow": "Some manually added install paths are invalid:",
	"banner.fixOrRemove": "Please fix or remove them in",
	"banner.rescanningClients": "Scanning WoW clients…",
	"banner.rescanClientsFound": "Client scan complete. Found {count} client(s).",
	"banner.rescanClientsNone": "Client scan complete. No clients found.",
	"banner.addonInstalled": "Addon installed",
	"browse.searchPlaceholder": "Search for World of Warcraft addons...",
	"browse.searchAria": "Search",
	"browse.loading": "Loading…",
	"browse.loadingList": "Searching…",
	"browse.noMatchTitle": "No matching addons",
	"browse.noMatchHint": "Try another keyword or adjust filters",
	"browse.prevPage": "Previous",
	"browse.nextPage": "Next",
	"browse.pageInfo": "Page {page} / {totalPages}",
	"browse.results": "{total} results",
	"browse.attributionBefore": "Addons come from ",
	"browse.attributionAfter": ". Please respect addon authors' copyright.",
	"filters.title": "Filters",
	"filters.clearAll": "Clear all",
	"filters.categories": "Categories",
	"filters.flavor": "Flavor",
	"card.by": "By {author}",
	"card.unknownAuthor": "Unknown",
	"card.noDescription": "No description",
	"card.needWowPath": "Add a WoW install path first",
	"card.noCompatibleInstall": "No compatible local client for this addon",
	"card.installTo": "Install to {label}",
	"card.versionInstalled": "{version} installed",
	"card.updateToVersion": "Update to {version}",
	"card.openDetail": "View addon details",
	"detail.title": "Addon details",
	"detail.close": "Close",
	"detail.loading": "Loading details…",
	"detail.screenshots": "Screenshots",
	"detail.description": "Description",
	"detail.noDescription": "No detailed description available.",
	"apiDisabled.title": "Cannot download via API",
	"apiDisabled.body":
		"These authors have disabled API distribution, so this app cannot download the files automatically.",
	"apiDisabled.hint": "Open the CurseForge page to download manually, then use Manual install if needed.",
	"apiDisabled.openCurseForge": "Open CurseForge",
	"apiDisabled.close": "Close",
	"card.installedAddon": "Installed addon",
	"card.installedAt": "Installed {date}",
	"card.update": "Update",
	"card.delete": "Delete",
	"card.updatingDownload": "Downloading…",
	"card.updatingDownloadPct": "Downloading {percent}%",
	"card.updatingExtract": "Extracting…",
	"card.updatingInstall": "Installing…",
	"card.suiteFolders": "{count} folders in package",
	"installed.title": "{label} installed addons",
	"installed.updateAll": "Update All",
	"installed.updateAllRemaining": "{n} remaining",
	"installed.manual": "Manual install",
	"installed.rescan": "Scan Addons",
	"installed.openInterface": "Open Interface Folder",
	"installed.scanning": "Scanning…",
	"installed.rescanning": "Scanning addon folder…",
	"installed.scanFound": "Found {name}, {count} addon(s) so far",
	"installed.loadingList": "Loading addon list…",
	"installed.emptyTitle": "No installed addons yet",
	"installed.emptyHint":
		'Install from Browse All, or click "Manual install" and choose one or more archives.',
	"installed.rescanDone": "Addon scan complete. {count} addon(s).",
	"installed.manualDone": "Manually installed: {names}",
	"installed.manualInstalling": "Installing {current}/{total}…",
	"installed.manualPartialFail":
		"Failed {failed} of {total}: {details}",
	"installed.deleteConfirm": "Delete addon {name}?",
	"installed.deleteSuiteConfirm":
		"Delete {name}? This removes the whole package ({count} folders).",
	"installed.deleteTitle": "Delete addon",
	"installed.deleteOk": "Delete",
	"installed.deleteCancel": "Cancel",
	"installed.pickArchive": "Select addon archive(s)",
	"installed.archiveFilter": "Addon Archive",
	"installed.clientMissing": "Client not found",
	"installed.backup": "Backup",
	"installed.backingUp": "Backing up…",
	"installed.backupDone": "Backup synced: {path}",
	"installed.backupNeedPath": "Set a backup folder in Settings first.",
	"installed.restore": "Restore",
	"installed.restoring": "Restoring…",
	"installed.restoreDone": "Restore synced from: {path}",
	"installed.restoreConfirm":
		"Restore Interface and WTF for {label} from the backup folder? Current files will be overwritten to match the backup.",
	"installed.restoreTitle": "Restore from backup",
	"installed.restoreOk": "Restore",
	"installed.restoreCancel": "Cancel",
	"installed.restoreMissing": "No backup found for this client.",
	"installed.syncBackupTitle": "Backup progress",
	"installed.syncRestoreTitle": "Restore progress",
	"installed.syncRunning": "Waiting for sync output…",
	"installed.syncDone": "Completed",
	"installed.syncFailed": "Sync failed",
	"installed.syncItems": "Folders: {items}",
	"installed.syncClose": "Close",
	"installed.syncOutput": "Output",
	"installed.subtabs": "Installed page sections",
	"installed.tabAddons": "Addons",
	"installed.tabWtf": "Config",
	"wtf.hint":
		"Manage WTF account and character profiles for this client. Close the game before deleting or copying.",
	"wtf.refresh": "Refresh",
	"wtf.refreshing": "Refreshing…",
	"wtf.openFolder": "Open WTF folder",
	"wtf.loading": "Loading WTF profiles…",
	"wtf.emptyTitle": "No WTF accounts found",
	"wtf.emptyHint":
		"Log into the game once, or check that this client’s WTF/Account folder exists.",
	"wtf.accountMeta": "{count} character(s)",
	"wtf.noCharacters": "No character folders under this account.",
	"wtf.copy": "Copy",
	"wtf.delete": "Delete",
	"wtf.deleting": "Deleting…",
	"wtf.deleteOk": "Delete",
	"wtf.deleteCancel": "Cancel",
	"wtf.deleteAccountTitle": "Delete account config",
	"wtf.deleteAccountConfirm":
		"Delete the entire WTF config for account {name}? This cannot be undone.",
	"wtf.deleteAccountDone": "Deleted account config: {name}",
	"wtf.deleteCharacterTitle": "Delete character config",
	"wtf.deleteCharacterConfirm":
		"Delete WTF config for character {name}? This cannot be undone.",
	"wtf.deleteCharacterDone": "Deleted character config: {name}",
	"wtf.copyTitle": "Copy config",
	"wtf.copyOk": "Copy",
	"wtf.copyCancel": "Cancel",
	"wtf.copying": "Copying…",
	"wtf.copyDest": "Destination",
	"wtf.copyDestPlaceholder": "Select destination…",
	"wtf.copyAccountFrom": "Copy from account {name}",
	"wtf.copyCharacterFrom": "Copy from character {name}",
	"wtf.copyItems": "What to copy",
	"wtf.copyItemSavedVariables": "SavedVariables",
	"wtf.copyItemAddOns": "Enabled addons (AddOns.txt)",
	"wtf.copyItemLayout": "Layout cache",
	"wtf.copyItemConfigCache": "Config cache",
	"wtf.copyItemChatCache": "Chat cache",
	"wtf.copyItemBindings": "Key bindings",
	"wtf.copyItemMacros": "Account macros",
	"wtf.copyItemEditMode": "Edit Mode cache",
	"wtf.copyAccountConfirm":
		"Overwrite selected account-level config on {to} with data from {from}?",
	"wtf.copyCharacterConfirm":
		"Overwrite selected character config on {to} with data from {from}?",
	"wtf.copyAccountDone": "Copied account config {from} → {to}",
	"wtf.copyCharacterDone": "Copied character config {from} → {to}",
	"settings.back": "← Back",
	"settings.title": "Settings",
	"settings.subtitle": "API Key, language, WoW paths, and logs",
	"settings.language": "Language",
	"settings.languageHint": "Default follows the system language. You can override it here.",
	"settings.languageSystem": "System default",
	"settings.languageZh": "简体中文",
	"settings.languageEn": "English",
	"settings.languageSaved": "Language updated.",
	"settings.apiTitle": "CurseForge API",
	"settings.apiKey": "CurseForge API Key",
	"settings.apiPlaceholder": "Paste x-api-key",
	"settings.apiHint":
		"With a key, search and download use the official API (`api.curseforge.com`). Clear and save to fall back to web GET.",
	"settings.apiEnabled": " Official API is currently enabled.",
	"settings.apiDisabled": " No key configured.",
	"settings.applyKey": "Apply for API Key",
	"settings.apiDocs": "API docs",
	"settings.saveApiKey": "Save API Key",
	"settings.saving": "Saving…",
	"settings.clearInput": "Clear input",
	"settings.apiSaved": "API Key saved. Browse and download will use the official CurseForge API.",
	"settings.apiCleared": "API Key cleared. Falling back to web GET.",
	"settings.wowTitle": "World of Warcraft installs",
	"settings.wowHint":
		"Startup scans the registry, Battle.net Agent, and common drive paths. If nothing is found, add a World of Warcraft root or a `_retail_` / `_classic_` folder here.",
	"settings.addWowPath": "Add install path",
	"settings.rescan": "Scan Clients",
	"settings.pickWowDir": "Select World of Warcraft install folder",
	"settings.pathAdded": "Added install path: {path}",
	"settings.pathRemoved": "Removed: {path}",
	"settings.detectedClients": "Detected clients",
	"settings.versionUnknown": "Unknown version",
	"settings.noClients": "No World of Warcraft clients detected.",
	"settings.manualPaths": "Manually added paths",
	"settings.remove": "Remove",
	"settings.invalidPaths": "These manual paths are invalid or have no recognizable client:",
	"settings.rescanDoneFound": "Rescan complete. Detected {count} client(s).",
	"settings.rescanDoneNone": "Rescan complete. Still no World of Warcraft clients found.",
	"settings.backupPath": "Backup",
	"settings.backupPathPlaceholder": "Not set",
	"settings.pickBackupDir": "Select backup folder for {label}",
	"settings.chooseBackupPath": "Set backup folder",
	"settings.clearBackupPath": "Clear",
	"settings.backupPathSaved": "Backup folder for {label} saved: {path}",
	"settings.backupPathCleared": "Backup folder for {label} cleared",
	"settings.backupPathUnset":
		"Set a backup folder for this client under Detected clients before backing up.",
	"settings.logsTitle": "Logs",
	"settings.logsHint":
		"Logs are always recorded from startup (default INFO). Show this panel only to view, change level, or save.",
	"settings.openLogs": "Show logs",
	"settings.closeLogs": "Hide logs",
	"settings.saveLogs": "Save log file",
	"settings.clearLogs": "Clear",
	"settings.logLevel": "Log level",
	"settings.noLogs": "(No logs yet)",
	"settings.logLevelSet": "Log level set to {level}",
	"settings.logsCleared": "Logs cleared",
	"settings.logsSaved": "Logs saved: {path}",
	"settings.saveLogTitle": "Save log file",
	"update.title": "App updates",
	"update.current": "Current version: {version}",
	"update.checking": "Checking for updates…",
	"update.upToDate": "You are on the latest version.",
	"update.downloading": "Downloading update {version}…",
	"update.ready": "Update {version} is ready.",
	"update.readyAction": "Restart to install",
	"update.availableManual": "Version {version} is available.",
	"update.openDownload": "Open download page",
	"update.openWebsite": "Open website",
	"update.checkNow": "Check for updates",
	"update.installRestart": "Install and restart",
	"update.installing": "Installing update…",
	"update.portableHint":
		"Portable builds cannot auto-install. Download the new installer or portable exe from GitHub Releases.",
	"update.error": "Update check failed: {error}",
	"update.notes": "Release notes",
};

export const zhCN: Messages = {
	"app.brand": "WoW Addon Manager",
	"nav.browse": "浏览全部",
	"nav.settings": "设置",
	"banner.noApiKey": "尚未配置 API Key，当前使用网页 GET。请到",
	"banner.goSettings": "设置",
	"banner.fillKey": "填写 Key 后改用官方接口。",
	"banner.applyForm": "申请表",
	"banner.docs": "文档",
	"banner.apiEnabled": "已启用 CurseForge 官方 API。",
	"banner.noWow": "未检测到魔兽世界安装目录。请到",
	"banner.addWowHint": "手动添加「World of Warcraft」根目录，或到设置里扫描客户端。",
	"banner.invalidWow": "部分手动设置的安装目录无效：",
	"banner.fixOrRemove": "请到",
	"banner.rescanningClients": "正在扫描魔兽客户端…",
	"banner.rescanClientsFound": "客户端扫描完成，发现 {count} 个客户端",
	"banner.rescanClientsNone": "客户端扫描完成，未发现客户端",
	"banner.addonInstalled": "插件已安装",
	"browse.searchPlaceholder": "搜索魔兽世界插件…",
	"browse.searchAria": "搜索",
	"browse.loading": "加载中…",
	"browse.loadingList": "正在搜索…",
	"browse.noMatchTitle": "没有匹配的插件",
	"browse.noMatchHint": "试试调整关键词或过滤器",
	"browse.prevPage": "上一页",
	"browse.nextPage": "下一页",
	"browse.pageInfo": "第 {page} / {totalPages} 页",
	"browse.results": "{total} 个结果",
	"browse.attributionBefore": "插件来自于 ",
	"browse.attributionAfter": "，请尊重插件作者版权。",
	"filters.title": "筛选",
	"filters.clearAll": "清除全部",
	"filters.categories": "分类",
	"filters.flavor": "游戏版本类型",
	"card.by": "作者 {author}",
	"card.unknownAuthor": "未知",
	"card.noDescription": "暂无简介",
	"card.needWowPath": "请先添加魔兽安装目录",
	"card.noCompatibleInstall": "当前已检测到的客户端均不支持该插件",
	"card.installTo": "安装到 {label}",
	"card.versionInstalled": "{version}已安装",
	"card.updateToVersion": "更新到{version}",
	"card.openDetail": "查看插件详情",
	"detail.title": "插件详情",
	"detail.close": "关闭",
	"detail.loading": "正在加载详情…",
	"detail.screenshots": "截图",
	"detail.description": "详细介绍",
	"detail.noDescription": "暂无详细介绍。",
	"apiDisabled.title": "无法通过 API 下载",
	"apiDisabled.body":
		"下列插件的作者禁用了 API 分发，本应用无法自动下载这些文件。",
	"apiDisabled.hint": "请打开 CurseForge 页面手动下载，如需本地安装可使用「手动安装」。",
	"apiDisabled.openCurseForge": "打开 CurseForge",
	"apiDisabled.close": "关闭",
	"card.installedAddon": "已安装插件",
	"card.installedAt": "安装于 {date}",
	"card.update": "更新",
	"card.delete": "删除",
	"card.updatingDownload": "下载中…",
	"card.updatingDownloadPct": "下载中 {percent}%",
	"card.updatingExtract": "解压中…",
	"card.updatingInstall": "安装中…",
	"card.suiteFolders": "含 {count} 个插件目录",
	"installed.title": "{label} 已安装插件",
	"installed.updateAll": "全部更新",
	"installed.updateAllRemaining": "剩余{n}个",
	"installed.manual": "手动安装",
	"installed.rescan": "扫描插件",
	"installed.openInterface": "打开插件目录",
	"installed.scanning": "扫描中…",
	"installed.rescanning": "正在扫描插件目录…",
	"installed.scanFound": "已找到{name}插件，共{count}个",
	"installed.loadingList": "正在读取插件列表…",
	"installed.emptyTitle": "还没有已安装插件",
	"installed.emptyHint": "可从「浏览全部」安装，或点击右上角「手动安装」选择一个或多个压缩包。",
	"installed.rescanDone": "插件扫描完成，共 {count} 个插件",
	"installed.manualDone": "已手动安装：{names}",
	"installed.manualInstalling": "正在安装 {current}/{total}…",
	"installed.manualPartialFail":
		"有 {failed}/{total} 个安装失败：{details}",
	"installed.deleteConfirm": "确定删除插件 {name}？",
	"installed.deleteSuiteConfirm":
		"确定删除 {name}？将一并删除整个合集（共 {count} 个插件目录）。",
	"installed.deleteTitle": "删除插件",
	"installed.deleteOk": "删除",
	"installed.deleteCancel": "取消",
	"installed.pickArchive": "选择插件压缩包（可多选）",
	"installed.archiveFilter": "插件压缩包",
	"installed.clientMissing": "未找到该客户端",
	"installed.backup": "备份",
	"installed.backingUp": "备份中…",
	"installed.backupDone": "备份已同步：{path}",
	"installed.backupNeedPath": "请先在设置中指定备份路径",
	"installed.restore": "恢复",
	"installed.restoring": "恢复中…",
	"installed.restoreDone": "已从备份恢复：{path}",
	"installed.restoreConfirm":
		"确定将备份中的 Interface / WTF 恢复到 {label}？当前目录会同步为与备份一致。",
	"installed.restoreTitle": "从备份恢复",
	"installed.restoreOk": "恢复",
	"installed.restoreCancel": "取消",
	"installed.restoreMissing": "未找到该客户端的备份",
	"installed.syncBackupTitle": "备份进度",
	"installed.syncRestoreTitle": "恢复进度",
	"installed.syncRunning": "等待同步输出…",
	"installed.syncDone": "已完成",
	"installed.syncFailed": "同步失败",
	"installed.syncItems": "目录：{items}",
	"installed.syncClose": "关闭",
	"installed.syncOutput": "输出",
	"installed.subtabs": "已安装页分区",
	"installed.tabAddons": "插件",
	"installed.tabWtf": "配置管理",
	"wtf.hint":
		"管理此客户端 WTF 下的账号与角色配置。删除或复制前请先完全退出游戏。",
	"wtf.refresh": "刷新",
	"wtf.refreshing": "刷新中…",
	"wtf.openFolder": "打开 WTF 目录",
	"wtf.loading": "正在读取 WTF 配置…",
	"wtf.emptyTitle": "未找到 WTF 账号",
	"wtf.emptyHint": "请先进入游戏登录一次，或确认该客户端存在 WTF/Account 目录。",
	"wtf.accountMeta": "{count} 个角色",
	"wtf.noCharacters": "该账号下暂无角色配置目录。",
	"wtf.copy": "复制",
	"wtf.delete": "删除",
	"wtf.deleting": "删除中…",
	"wtf.deleteOk": "删除",
	"wtf.deleteCancel": "取消",
	"wtf.deleteAccountTitle": "删除账号配置",
	"wtf.deleteAccountConfirm":
		"确定删除账号 {name} 的全部 WTF 配置？此操作不可恢复。",
	"wtf.deleteAccountDone": "已删除账号配置：{name}",
	"wtf.deleteCharacterTitle": "删除角色配置",
	"wtf.deleteCharacterConfirm":
		"确定删除角色 {name} 的 WTF 配置？此操作不可恢复。",
	"wtf.deleteCharacterDone": "已删除角色配置：{name}",
	"wtf.copyTitle": "复制配置",
	"wtf.copyOk": "复制",
	"wtf.copyCancel": "取消",
	"wtf.copying": "复制中…",
	"wtf.copyDest": "目标",
	"wtf.copyDestPlaceholder": "选择目标…",
	"wtf.copyAccountFrom": "从账号 {name} 复制",
	"wtf.copyCharacterFrom": "从角色 {name} 复制",
	"wtf.copyItems": "要复制的内容",
	"wtf.copyItemSavedVariables": "保存的变量 (SavedVariables)",
	"wtf.copyItemAddOns": "启用的插件 (AddOns.txt)",
	"wtf.copyItemLayout": "界面布局缓存",
	"wtf.copyItemConfigCache": "配置缓存 (config-cache)",
	"wtf.copyItemChatCache": "聊天缓存",
	"wtf.copyItemBindings": "按键绑定",
	"wtf.copyItemMacros": "账号宏",
	"wtf.copyItemEditMode": "编辑模式缓存",
	"wtf.copyAccountConfirm":
		"确定用 {from} 的所选账号级配置覆盖 {to} 中对应项？",
	"wtf.copyCharacterConfirm":
		"确定用 {from} 的所选角色配置覆盖 {to} 中对应项？",
	"wtf.copyAccountDone": "已复制账号配置 {from} → {to}",
	"wtf.copyCharacterDone": "已复制角色配置 {from} → {to}",
	"settings.back": "← 返回",
	"settings.title": "设置",
	"settings.subtitle": "配置语言、API Key、魔兽世界安装目录与日志",
	"settings.language": "语言",
	"settings.languageHint": "默认跟随系统语言，也可在此手动切换。",
	"settings.languageSystem": "跟随系统",
	"settings.languageZh": "简体中文",
	"settings.languageEn": "English",
	"settings.languageSaved": "语言已更新。",
	"settings.apiTitle": "CurseForge API",
	"settings.apiKey": "CurseForge API Key",
	"settings.apiPlaceholder": "粘贴 x-api-key",
	"settings.apiHint":
		"填写后，搜索与下载走官方 API（`api.curseforge.com`）。清空并保存则恢复网页 GET。",
	"settings.apiEnabled": " 当前已启用官方 API。",
	"settings.apiDisabled": " 当前未配置 Key。",
	"settings.applyKey": "申请 API Key",
	"settings.apiDocs": "API 文档",
	"settings.saveApiKey": "保存 API Key",
	"settings.saving": "保存中…",
	"settings.clearInput": "清空输入",
	"settings.apiSaved": "已保存 API Key。浏览与下载将使用 CurseForge 官方 API。",
	"settings.apiCleared": "已清空 API Key。将回退到网页 GET。",
	"settings.wowTitle": "魔兽世界安装目录",
	"settings.wowHint":
		"启动时会自动扫描注册表、Battle.net Agent、常见盘符路径。若未找到，可在此手动添加「World of Warcraft」根目录，或其中的 `_retail_` / `_classic_` 等版本目录。",
	"settings.addWowPath": "添加安装目录",
	"settings.rescan": "扫描客户端",
	"settings.pickWowDir": "选择 World of Warcraft 安装目录",
	"settings.pathAdded": "已添加安装目录：{path}",
	"settings.pathRemoved": "已移除：{path}",
	"settings.detectedClients": "已检测到的客户端",
	"settings.versionUnknown": "版本未知",
	"settings.noClients": "尚未检测到魔兽世界客户端。",
	"settings.manualPaths": "手动添加的路径",
	"settings.remove": "移除",
	"settings.invalidPaths": "以下手动路径无效或不包含可识别的客户端：",
	"settings.rescanDoneFound": "客户端扫描完成，检测到 {count} 个客户端。",
	"settings.rescanDoneNone": "客户端扫描完成，仍未检测到魔兽世界客户端。",
	"settings.backupPath": "备份目录",
	"settings.backupPathPlaceholder": "未设置",
	"settings.pickBackupDir": "为 {label} 选择备份目录",
	"settings.chooseBackupPath": "设置备份目录",
	"settings.clearBackupPath": "清除",
	"settings.backupPathSaved": "已为 {label} 保存备份目录：{path}",
	"settings.backupPathCleared": "已清除 {label} 的备份目录",
	"settings.backupPathUnset": "请先在「已检测到的客户端」中为该版本设置备份目录。",
	"settings.logsTitle": "日志",
	"settings.logsHint":
		"日志从启动起始终记录（默认 INFO）。此处仅用于查看、调整级别或保存文件。",
	"settings.openLogs": "查看日志",
	"settings.closeLogs": "收起日志",
	"settings.saveLogs": "保存日志文件",
	"settings.clearLogs": "清空",
	"settings.logLevel": "日志级别",
	"settings.noLogs": "（暂无日志）",
	"settings.logLevelSet": "日志级别已设为 {level}",
	"settings.logsCleared": "日志已清空",
	"settings.logsSaved": "日志已保存：{path}",
	"settings.saveLogTitle": "保存日志文件",
	"update.title": "应用更新",
	"update.current": "当前版本：{version}",
	"update.checking": "正在检查更新…",
	"update.upToDate": "已是最新版本。",
	"update.downloading": "正在下载更新 {version}…",
	"update.ready": "更新 {version} 已就绪。",
	"update.readyAction": "重启以安装",
	"update.availableManual": "发现新版本 {version}。",
	"update.openDownload": "打开下载页",
	"update.openWebsite": "打开官网",
	"update.checkNow": "检查更新",
	"update.installRestart": "安装并重启",
	"update.installing": "正在安装更新…",
	"update.portableHint":
		"便携版无法自动安装。请到 GitHub Releases 下载新的安装包或便携 exe。",
	"update.error": "检查更新失败：{error}",
	"update.notes": "更新说明",
};

export const catalogs: Record<LocaleCode, Messages> = {
	en,
	"zh-CN": zhCN,
};

export function normalizeLocale(raw: string | null | undefined): LocaleCode {
	const value = (raw || "").trim().toLowerCase().replace(/_/g, "-");
	if (value.startsWith("zh")) {
		return "zh-CN";
	}
	return "en";
}

export function detectSystemLocale(): LocaleCode {
	if (typeof navigator !== "undefined") {
		const candidates = [...(navigator.languages ?? []), navigator.language].filter(Boolean);
		if (candidates[0]) {
			return normalizeLocale(candidates[0]);
		}
	}
	return "en";
}

export function resolveLocale(
	preference: LocalePreference | string | null | undefined,
	systemLocale?: string | null,
): LocaleCode {
	const pref = (preference || "system").trim();
	if (pref === "system" || pref === "" || pref === "auto") {
		return systemLocale ? normalizeLocale(systemLocale) : detectSystemLocale();
	}
	return normalizeLocale(pref);
}

export function translate(
	locale: LocaleCode,
	key: MessageKey,
	vars?: Record<string, string | number>,
): string {
	const template = catalogs[locale][key] ?? catalogs.en[key] ?? key;
	if (!vars) return template;
	return template.replace(/\{(\w+)\}/g, (_, name: string) =>
		vars[name] === undefined || vars[name] === null ? "" : String(vars[name]),
	);
}
