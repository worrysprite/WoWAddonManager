/** Resolve a CurseForge mod id from browse/installed card fields. */
export function resolveAddonModId(options: {
	id?: number | null;
	curseforgeId?: number | null;
}): number | null {
	if (typeof options.id === "number" && options.id > 0) {
		return options.id;
	}
	if (typeof options.curseforgeId === "number" && options.curseforgeId > 0) {
		return options.curseforgeId;
	}
	return null;
}

/** Lightweight HTML cleanup before rendering CurseForge description. */
export function sanitizeAddonHtml(html: string): string {
	return html
		.replace(/<script[\s\S]*?>[\s\S]*?<\/script>/gi, "")
		.replace(/<iframe[\s\S]*?>[\s\S]*?<\/iframe>/gi, "")
		.replace(/<object[\s\S]*?>[\s\S]*?<\/object>/gi, "")
		.replace(/<embed[\s\S]*?>/gi, "")
		.replace(/\son[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)/gi, "")
		.replace(/javascript:/gi, "");
}
