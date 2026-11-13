import { useEffect, useState } from "react";
import {
	getUpdaterState,
	runUpdateCheck,
	subscribeUpdater,
	type UpdaterState,
} from "../updater";

/** Subscribe to shared updater state; kick off a silent check once on mount. */
export function useAppUpdater(options?: { autoCheck?: boolean }) {
	const autoCheck = options?.autoCheck ?? true;
	const [state, setState] = useState<UpdaterState>(() => getUpdaterState());

	useEffect(() => subscribeUpdater(setState), []);

	useEffect(() => {
		if (!autoCheck) return;
		runUpdateCheck({ autoDownload: true }).catch(() => undefined);
	}, [autoCheck]);

	return state;
}
