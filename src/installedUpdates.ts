import * as api from "./api";
import type { InstalledAddon } from "./types";

/** Match app auto-update: first tick shortly after schedule start, then hourly. */
const FIRST_CHECK_DELAY_MS = 3_000;
const HOURLY_CHECK_MS = 60 * 60 * 1_000;

type Listener = (installId: string, list: InstalledAddon[]) => void;

const listeners = new Set<Listener>();
let getInstallIds: (() => string[]) | null = null;
let startTimer: ReturnType<typeof setTimeout> | null = null;
let hourlyTimer: ReturnType<typeof setInterval> | null = null;
let running = false;
let generation = 0;

export function subscribeInstalledUpdates(listener: Listener): () => void {
	listeners.add(listener);
	return () => {
		listeners.delete(listener);
	};
}

function emit(installId: string, list: InstalledAddon[]) {
	for (const listener of listeners) {
		listener(installId, list);
	}
}

function clearTimers() {
	if (startTimer !== null) {
		clearTimeout(startTimer);
		startTimer = null;
	}
	if (hourlyTimer !== null) {
		clearInterval(hourlyTimer);
		hourlyTimer = null;
	}
}

async function checkAll(gen: number) {
	if (running) return;
	const ids = getInstallIds?.() ?? [];
	if (ids.length === 0) return;
	running = true;
	try {
		for (const installId of ids) {
			if (gen !== generation) return;
			try {
				const list = await api.checkInstalledUpdates(installId);
				if (gen !== generation) return;
				emit(installId, list);
			} catch {
				// Best-effort; keep prior badges.
			}
		}
	} finally {
		running = false;
	}
}

/**
 * Start / restart the CurseForge update schedule, then once per hour.
 * - Launch (`runSoon: true`): first check after ~3s.
 * - After scan (`runSoon: false`): wait a full hour before the next check.
 */
export function startInstalledUpdateSchedule(
	nextGetInstallIds: () => string[],
	options?: { runSoon?: boolean },
) {
	getInstallIds = nextGetInstallIds;
	generation += 1;
	const gen = generation;
	clearTimers();
	const delay =
		options?.runSoon === false ? HOURLY_CHECK_MS : FIRST_CHECK_DELAY_MS;

	startTimer = setTimeout(() => {
		startTimer = null;
		if (gen !== generation) return;
		void checkAll(gen).finally(() => {
			if (gen !== generation) return;
			hourlyTimer = setInterval(() => {
				void checkAll(generation);
			}, HOURLY_CHECK_MS);
		});
	}, delay);
}

/** After disk scan: restart the hourly clock (next CF check in one hour). */
export function noteInstalledScanCompleted() {
	if (!getInstallIds) return;
	startInstalledUpdateSchedule(getInstallIds, { runSoon: false });
}

export function stopInstalledUpdateSchedule() {
	generation += 1;
	clearTimers();
	getInstallIds = null;
}

/** Background icon cache after disk scan (no CurseForge update check). */
export async function localizeIconsInBackground(installId: string) {
	try {
		const list = await api.localizeInstalledIcons(installId);
		emit(installId, list);
	} catch {
		// Best-effort.
	}
}
