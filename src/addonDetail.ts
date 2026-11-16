import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { openUrl } from "@tauri-apps/plugin-opener";

const DETAIL_LABEL = "addon-detail";

/** Build a CurseForge addon page URL from an optional website URL or slug. */
export function resolveAddonWebsiteUrl(options: {
	websiteUrl?: string | null;
	slug?: string | null;
}): string | null {
	const direct = options.websiteUrl?.trim();
	if (direct && /^https?:\/\//i.test(direct)) {
		return direct;
	}
	const slug = options.slug?.trim();
	if (slug) {
		return `https://www.curseforge.com/wow/addons/${encodeURIComponent(slug)}`;
	}
	return null;
}

/**
 * Open the CurseForge addon page in an app webview window (popup).
 * Falls back to the system browser if the window cannot be created.
 */
export async function openAddonDetailPage(
	url: string,
	title?: string,
): Promise<void> {
	const target = url.trim();
	if (!target) return;

	try {
		const existing = await WebviewWindow.getByLabel(DETAIL_LABEL);
		if (existing) {
			await existing.close();
		}
	} catch {
		/* no existing window */
	}

	try {
		await new Promise<void>((resolve, reject) => {
			const win = new WebviewWindow(DETAIL_LABEL, {
				url: target,
				title: title?.trim() || "CurseForge",
				width: 1100,
				height: 800,
				minWidth: 720,
				minHeight: 520,
				center: true,
				focus: true,
				resizable: true,
			});
			win.once("tauri://created", () => resolve());
			win.once("tauri://error", (event) => {
				reject(event.payload ?? new Error("Failed to open addon detail window"));
			});
		});
	} catch {
		await openUrl(target);
	}
}
