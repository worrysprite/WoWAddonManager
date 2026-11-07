import type { AddonSummary, WowInstall } from "./types";

export function formatDownloads(count?: number | null): string {
	if (count == null || Number.isNaN(count)) return "-";
	const abs = Math.abs(count);
	if (abs >= 1_000_000_000) return `${(count / 1_000_000_000).toFixed(1)}B`;
	if (abs >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
	if (abs >= 1_000) return `${(count / 1_000).toFixed(1)}K`;
	return `${count}`;
}

/** Normalize flavor labels / keys for comparison (`MoP Classic` ≈ `mop-classic`). */
export function normalizeFlavorToken(value: string): string {
	return value.toLowerCase().replace(/[^a-z0-9]+/g, "");
}

const FLAVOR_KEY_ALIASES: Record<string, string[]> = {
	retail: ["retail"],
	ptr: ["retail", "ptr"],
	xptr: ["retail", "xptr"],
	classic: ["classic"],
	"mop-classic": ["mopclassic"],
	"classic-beta": ["mopclassic", "classicbeta"],
	"classic-ptr": ["mopclassic", "classicptr"],
	forever: ["forever"],
	"classic-tbc": ["classictbc"],
	cataclysm: ["cataclysmclassic", "cataclysm"],
	wotlk: ["wotlkclassic", "wotlk"],
};

/**
 * Whether a browse addon lists support for this local WoW install.
 * If the addon has no flavor metadata (web scrape / incomplete), keep all buttons.
 */
export function addonSupportsInstall(addon: AddonSummary, install: WowInstall): boolean {
	if (!addon.flavors.length) {
		return true;
	}

	const addonTokens = new Set(addon.flavors.map(normalizeFlavorToken).filter(Boolean));
	const candidates = new Set<string>();

	for (const raw of [install.flavor, install.label, install.flavorKey]) {
		const token = normalizeFlavorToken(raw);
		if (token) candidates.add(token);
	}
	for (const alias of FLAVOR_KEY_ALIASES[install.flavorKey] ?? []) {
		candidates.add(alias);
	}

	for (const candidate of candidates) {
		if (addonTokens.has(candidate)) return true;
		for (const token of addonTokens) {
			if (token.includes(candidate) || candidate.includes(token)) return true;
		}
	}
	return false;
}

export function compatibleInstalls(
	addon: AddonSummary,
	installs: WowInstall[],
): WowInstall[] {
	return installs.filter((install) => addonSupportsInstall(addon, install));
}

export function formatDate(value?: string | null): string {
	if (!value) return "-";
	const date = new Date(value);
	if (!Number.isNaN(date.getTime())) {
		return date.toLocaleDateString("en-US", {
			month: "short",
			day: "numeric",
			year: "numeric",
		});
	}
	// Already human readable (e.g. 2026-11-07)
	if (/^\d{4}-\d{2}-\d{2}/.test(value)) {
		const d = new Date(`${value.slice(0, 10)}T00:00:00Z`);
		if (!Number.isNaN(d.getTime())) {
			return d.toLocaleDateString("en-US", {
				month: "short",
				day: "numeric",
				year: "numeric",
			});
		}
	}
	return value;
}

export function primaryCategory(categories: string[]): string {
	return categories[0] ?? "Addon";
}

export function extraCategoryCount(categories: string[]): number {
	return Math.max(0, categories.length - 1);
}
