export interface AddonSummary {
	id: number;
	slug: string;
	name: string;
	author: string;
	summary: string;
	thumbnailUrl: string;
	downloadCount: number;
	updatedAt: string;
	latestGameVersion: string;
	categories: string[];
	flavors: string[];
	websiteUrl: string;
	latestFileId?: number | null;
}

export interface SearchResult {
	items: AddonSummary[];
	page: number;
	pageSize: number;
	total: number;
	source: string;
	message?: string | null;
}

export interface SearchParams {
	search?: string;
	page?: number;
	pageSize?: number;
	categories?: string[];
	gameVersionTypeIds?: number[];
	sortBy?: string;
}

export interface FilterOption {
	id: string;
	label: string;
	value: string;
}

export interface FlavorOption {
	id: string;
	label: string;
	gameVersionTypeId: number;
}

export interface FilterCatalog {
	categories: FilterOption[];
	flavors: FlavorOption[];
	gameVersions: FilterOption[];
}

export interface WowInstall {
	id: string;
	label: string;
	flavor: string;
	flavorKey: string;
	installPath: string;
	addonsPath: string;
	gameVersion?: string | null;
	gameVersionTypeId?: number | null;
	backupPath?: string;
}

export interface InstalledAddon {
	folderName: string;
	title: string;
	author?: string | null;
	notes?: string | null;
	interfaceVersion?: string | null;
	version?: string | null;
	curseforgeId?: number | null;
	curseforgeSlug?: string | null;
	fileId?: number | null;
	thumbnailUrl?: string | null;
	categories: string[];
	installedAt?: string | null;
	updatedAt?: string | null;
	downloadCount?: number | null;
	latestGameVersion?: string | null;
	flavors: string[];
	hasUpdate: boolean;
	remoteVersion?: string | null;
	remoteFileId?: number | null;
	websiteUrl?: string | null;
	/** All AddOns folders in this CurseForge/manual package (including primary). */
	relatedFolders?: string[];
	isPrimary?: boolean;
	archiveName?: string | null;
}

export interface AddonJobProgress {
	stage: "download" | "extract" | "install" | string;
	percent?: number | null;
	received?: number | null;
	total?: number | null;
}

export interface ApiHelp {
	docs: string;
	apply: string;
	console: string;
	note: string;
}

export interface SettingsView {
	curseforgeApiKey: string;
	hasApiKey: boolean;
	applyUrl: string;
	docsUrl: string;
	wowRoots: string[];
	detectedInstalls: WowInstall[];
	invalidWowRoots: string[];
	hasWowInstall: boolean;
	locale: string;
	systemLocale: string;
}

export interface LogEntry {
	timestamp: string;
	level: string;
	target: string;
	message: string;
}

export interface LogView {
	level: string;
	entries: LogEntry[];
}

export interface SyncReport {
	path: string;
	engine: string;
	items: string[];
	log: string;
	success: boolean;
}
