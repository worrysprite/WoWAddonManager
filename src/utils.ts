export function formatDownloads(count?: number | null): string {
	if (count == null || Number.isNaN(count)) return "-";
	const abs = Math.abs(count);
	if (abs >= 1_000_000_000) return `${(count / 1_000_000_000).toFixed(1)}B`;
	if (abs >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
	if (abs >= 1_000) return `${(count / 1_000).toFixed(1)}K`;
	return `${count}`;
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
