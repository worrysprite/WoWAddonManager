import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { openUrl } from "@tauri-apps/plugin-opener";
import { invoke } from "@tauri-apps/api/core";

export const RELEASES_LATEST_URL =
	"https://github.com/worrysprite/WoWAddonManager/releases/latest";

export type UpdateInstallKind = "installed" | "portable";

export type UpdaterStatus =
	| "idle"
	| "checking"
	| "upToDate"
	| "downloading"
	| "ready"
	| "availableManual"
	| "installing"
	| "error";

export type UpdaterState = {
	status: UpdaterStatus;
	installKind: UpdateInstallKind;
	currentVersion: string;
	remoteVersion: string | null;
	notes: string | null;
	downloadedBytes: number;
	totalBytes: number | null;
	error: string | null;
};

type Listener = (state: UpdaterState) => void;

const INITIAL: UpdaterState = {
	status: "idle",
	installKind: "installed",
	currentVersion: "",
	remoteVersion: null,
	notes: null,
	downloadedBytes: 0,
	totalBytes: null,
	error: null,
};

let state: UpdaterState = { ...INITIAL };
let pendingUpdate: Update | null = null;
let inFlight: Promise<void> | null = null;
const listeners = new Set<Listener>();

function emit() {
	for (const listener of listeners) {
		listener(state);
	}
}

function setState(patch: Partial<UpdaterState>) {
	state = { ...state, ...patch };
	emit();
}

export function getUpdaterState(): UpdaterState {
	return state;
}

export function subscribeUpdater(listener: Listener): () => void {
	listeners.add(listener);
	listener(state);
	return () => {
		listeners.delete(listener);
	};
}

export async function openLatestReleasePage(): Promise<void> {
	await openUrl(RELEASES_LATEST_URL);
}

async function loadMeta(): Promise<void> {
	const [currentVersion, installKind] = await Promise.all([
		invoke<string>("get_app_version"),
		invoke<string>("get_update_install_kind"),
	]);
	setState({
		currentVersion,
		installKind: installKind === "portable" ? "portable" : "installed",
	});
}

async function discardPending(): Promise<void> {
	if (pendingUpdate) {
		try {
			await pendingUpdate.close();
		} catch {
			/* ignore */
		}
		pendingUpdate = null;
	}
}

/**
 * Check for updates. When `autoDownload` and install kind is installed,
 * silently download then mark ready for restart.
 */
export async function runUpdateCheck(options?: {
	autoDownload?: boolean;
}): Promise<void> {
	const autoDownload = options?.autoDownload ?? true;
	if (inFlight) {
		return inFlight;
	}

	inFlight = (async () => {
		// Keep an already-downloaded / announced update; do not discard on re-check.
		if (
			pendingUpdate &&
			(state.status === "ready" || state.status === "availableManual")
		) {
			return;
		}

		setState({
			status: "checking",
			error: null,
			downloadedBytes: 0,
			totalBytes: null,
		});
		try {
			await loadMeta();
			await discardPending();
			const update = await check();
			if (!update) {
				setState({
					status: "upToDate",
					remoteVersion: null,
					notes: null,
				});
				return;
			}

			pendingUpdate = update;
			setState({
				remoteVersion: update.version,
				notes: update.body ?? null,
				currentVersion: update.currentVersion || state.currentVersion,
			});

			if (state.installKind === "portable") {
				setState({ status: "availableManual" });
				return;
			}

			if (!autoDownload) {
				setState({ status: "availableManual" });
				return;
			}

			setState({ status: "downloading", downloadedBytes: 0, totalBytes: null });
			let downloaded = 0;
			await update.download((event) => {
				if (event.event === "Started") {
					downloaded = 0;
					setState({
						downloadedBytes: 0,
						totalBytes: event.data.contentLength ?? null,
					});
				} else if (event.event === "Progress") {
					downloaded += event.data.chunkLength;
					setState({ downloadedBytes: downloaded });
				}
			});
			setState({ status: "ready" });
		} catch (err) {
			await discardPending();
			setState({
				status: "error",
				error: String(err),
			});
		} finally {
			inFlight = null;
		}
	})();

	return inFlight;
}

/** Install a previously downloaded update (Windows exits; installer relaunches). */
export async function installDownloadedUpdate(): Promise<void> {
	if (!pendingUpdate) {
		throw new Error("No downloaded update ready to install");
	}
	setState({ status: "installing", error: null });
	try {
		await pendingUpdate.install({ restartAfterInstall: true });
		// Non-Windows: relaunch into the new build.
		try {
			await relaunch();
		} catch {
			/* Windows already exited on install */
		}
	} catch (err) {
		setState({
			status: "error",
			error: String(err),
		});
		throw err;
	}
}

/** Download (if needed) then install — used by Settings "Check for updates". */
export async function downloadAndInstallNow(): Promise<void> {
	if (state.installKind === "portable") {
		await openLatestReleasePage();
		return;
	}
	if (state.status !== "ready" || !pendingUpdate) {
		await runUpdateCheck({ autoDownload: true });
	}
	if (state.status === "ready") {
		await installDownloadedUpdate();
		return;
	}
	if (state.status === "availableManual") {
		await openLatestReleasePage();
	}
}
