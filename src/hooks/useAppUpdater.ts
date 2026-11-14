import { useEffect, useState } from "react";
import {
	getUpdaterState,
	runUpdateCheck,
	setupInstallOnClose,
	subscribeUpdater,
	type UpdaterState,
} from "../updater";

const FIRST_CHECK_DELAY_MS = 3_000;
const HOURLY_CHECK_MS = 60 * 60 * 1_000;

function isDownloadedReady(status: UpdaterState["status"]): boolean {
	return status === "ready";
}

/** Subscribe to shared updater state; schedule silent checks when autoCheck. */
export function useAppUpdater(options?: { autoCheck?: boolean }) {
	const autoCheck = options?.autoCheck ?? true;
	const [state, setState] = useState<UpdaterState>(() => getUpdaterState());

	useEffect(() => subscribeUpdater(setState), []);

	useEffect(() => {
		if (!autoCheck) return;

		let cancelled = false;
		let startTimer: ReturnType<typeof setTimeout> | null = null;
		let hourlyTimer: ReturnType<typeof setInterval> | null = null;

		const clearTimers = () => {
			if (startTimer !== null) {
				clearTimeout(startTimer);
				startTimer = null;
			}
			if (hourlyTimer !== null) {
				clearInterval(hourlyTimer);
				hourlyTimer = null;
			}
		};

		const tick = () => {
			if (cancelled || isDownloadedReady(getUpdaterState().status)) {
				clearTimers();
				return;
			}
			runUpdateCheck({ autoDownload: true })
				.catch(() => undefined)
				.finally(() => {
					if (cancelled || isDownloadedReady(getUpdaterState().status)) {
						clearTimers();
					}
				});
		};

		startTimer = setTimeout(() => {
			startTimer = null;
			if (cancelled || isDownloadedReady(getUpdaterState().status)) return;
			tick();
			if (cancelled || isDownloadedReady(getUpdaterState().status)) return;
			hourlyTimer = setInterval(tick, HOURLY_CHECK_MS);
		}, FIRST_CHECK_DELAY_MS);

		const unsub = subscribeUpdater((next) => {
			if (isDownloadedReady(next.status)) {
				clearTimers();
			}
		});

		return () => {
			cancelled = true;
			clearTimers();
			unsub();
		};
	}, [autoCheck]);

	useEffect(() => {
		if (!autoCheck) return;
		let disposed = false;
		let unlisten: (() => void) | null = null;
		setupInstallOnClose()
			.then((fn) => {
				if (disposed) {
					fn();
					return;
				}
				unlisten = fn;
			})
			.catch(() => undefined);
		return () => {
			disposed = true;
			unlisten?.();
		};
	}, [autoCheck]);

	return state;
}
