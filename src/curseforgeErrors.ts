/** Matches Rust `API_DISTRIBUTION_DISABLED_CODE` in curseforge/api.rs */
export const API_DISTRIBUTION_DISABLED_CODE = "CF_API_DISTRIBUTION_DISABLED";

export function isApiDistributionDisabledError(err: unknown): boolean {
	return String(err).includes(API_DISTRIBUTION_DISABLED_CODE);
}

/** Prefer stored website URL; else build from slug or numeric id. */
export function resolveCurseforgeAddonUrl(opts: {
	websiteUrl?: string | null;
	slug?: string | null;
	id?: number | null;
}): string | null {
	const website = opts.websiteUrl?.trim();
	if (website && /^https?:\/\//i.test(website)) {
		return website;
	}
	const slug = opts.slug?.trim();
	if (slug) {
		return `https://www.curseforge.com/wow/addons/${encodeURIComponent(slug)}`;
	}
	if (opts.id != null && Number.isFinite(opts.id) && opts.id > 0) {
		return `https://www.curseforge.com/wow/addons/${opts.id}`;
	}
	return null;
}
