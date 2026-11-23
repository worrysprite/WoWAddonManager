import { useEffect, useRef, useState } from "react";
import { ask, open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import {
	isApiDistributionDisabledError,
	resolveCurseforgeAddonUrl,
} from "../curseforgeErrors";
import { useToast } from "../hooks/useToast";
import { useI18n } from "../i18n";
import { translateInstallLabel } from "../i18n/catalogLabels";
import {
	enrichAfterScan,
	getInstalledUpdateEpoch,
	getLastInstalledUpdateResult,
	noteInstalledDiskScan,
	subscribeInstalledUpdates,
} from "../installedUpdates";
import type {
	AddonJobProgress,
	AddonScanProgress,
	InstalledAddon,
	SyncReport,
	WowInstall,
} from "../types";
import { AddonCard } from "./AddonCard";
import { AddonDetailModal } from "./AddonDetailModal";
import { ApiDistributionDisabledDialog } from "./ApiDistributionDisabledDialog";

type SyncDialogState = {
	kind: "backup" | "restore";
	running: boolean;
	liveLog: string;
	report: SyncReport | null;
	error: string | null;
};

type Props = {
	install: WowInstall;
	refreshKey: number;
	onNeedBackupPath: () => void;
};

const UPDATE_ALL_CONCURRENCY = 5;

export function InstalledTab({
	install,
	refreshKey,
	onNeedBackupPath,
}: Props) {
	const hasBackupPath = Boolean(install.backupPath?.trim());
	const { t, locale } = useI18n();
	const { showToast, toastNode } = useToast();
	const installLabel = translateInstallLabel(install.flavorKey, install.label, locale);
	const [addons, setAddons] = useState<InstalledAddon[]>([]);
	const [loading, setLoading] = useState(false);
	const [scanning, setScanning] = useState(false);
	const [scanProgress, setScanProgress] = useState<AddonScanProgress | null>(
		null,
	);
	/** Per-addon / per-op busy keys (folderName, or __manual__/__backup__/__restore__). */
	const [busyIds, setBusyIds] = useState<Record<string, true>>({});
	const [updateProgressById, setUpdateProgressById] = useState<
		Record<string, AddonJobProgress>
	>({});
	/** Remaining Update All work (in-flight + queued). 0 = idle. */
	const [updateAllRemaining, setUpdateAllRemaining] = useState(0);
	const [error, setError] = useState<string | null>(null);
	const [syncDialog, setSyncDialog] = useState<SyncDialogState | null>(null);
	const [detail, setDetail] = useState<{ modId: number; title: string } | null>(
		null,
	);
	const [apiDisabledDialog, setApiDisabledDialog] = useState<{
		addonName: string;
		curseforgeUrl: string;
	} | null>(null);
	const syncOutputRef = useRef<HTMLPreElement | null>(null);
	const syncLiveLogRef = useRef("");
	const syncFlushTimerRef = useRef<number | null>(null);
	const requestSeq = useRef(0);
	const busyIdsRef = useRef<Record<string, true>>({});
	const updateAllActiveRef = useRef(false);
	const installIdRef = useRef(install.id);
	const prevInstallIdRef = useRef(install.id);
	const addonsRef = useRef<InstalledAddon[]>([]);
	installIdRef.current = install.id;
	addonsRef.current = addons;

	function beginRequest(forInstallId: string): number | null {
		if (installIdRef.current !== forInstallId) {
			return null;
		}
		return ++requestSeq.current;
	}

	function isCurrent(forInstallId: string, seq: number): boolean {
		return installIdRef.current === forInstallId && requestSeq.current === seq;
	}

	function markBusy(id: string) {
		if (busyIdsRef.current[id]) return false;
		busyIdsRef.current = { ...busyIdsRef.current, [id]: true };
		setBusyIds(busyIdsRef.current);
		return true;
	}

	function clearBusy(id: string) {
		if (!busyIdsRef.current[id]) return;
		const next = { ...busyIdsRef.current };
		delete next[id];
		busyIdsRef.current = next;
		setBusyIds(next);
	}

	function setAddonProgress(id: string, progress: AddonJobProgress) {
		setUpdateProgressById((prev) => ({ ...prev, [id]: progress }));
	}

	function clearAddonProgress(id: string) {
		setUpdateProgressById((prev) => {
			if (!(id in prev)) return prev;
			const next = { ...prev };
			delete next[id];
			return next;
		});
	}

	const anyBusy = Object.keys(busyIds).length > 0;

	async function loadCached(
		forInstallId = install.id,
		opts?: { silent?: boolean },
	) {
		const seq = beginRequest(forInstallId);
		if (seq === null) return;
		const silent = Boolean(opts?.silent);
		const epochAtStart = getInstalledUpdateEpoch();
		if (!silent) {
			setLoading(true);
			setError(null);
		}
		try {
			const list = await api.listInstalledAddons(forInstallId);
			if (!isCurrent(forInstallId, seq)) return;
			// Launch/hourly CF check may have finished while we awaited; do not
			// clobber fresher hasUpdate badges with a stale listInstalled result.
			if (getInstalledUpdateEpoch() !== epochAtStart) {
				const newer = getLastInstalledUpdateResult(forInstallId);
				if (newer) {
					setAddons(newer);
					return;
				}
			}
			setAddons(list);
		} catch (err) {
			if (!isCurrent(forInstallId, seq)) return;
			setError(String(err));
		} finally {
			if (!silent && isCurrent(forInstallId, seq)) {
				setLoading(false);
			}
		}
	}

	async function handleRescan() {
		const forInstallId = install.id;
		const seq = beginRequest(forInstallId);
		if (seq === null) return;
		setScanning(true);
		setScanProgress(null);
		setError(null);
		try {
			const list = await api.rescanInstalledAddons(forInstallId, (progress) => {
				if (!isCurrent(forInstallId, seq)) return;
				setScanProgress(progress);
			});
			if (!isCurrent(forInstallId, seq)) return;
			setAddons(list);
			showToast(t("installed.rescanDone", { count: list.length }));
			setScanning(false);
			setScanProgress(null);
			// Background: one CF update check + icons; hourly clock resets from now.
			void enrichAfterScan(forInstallId);
		} catch (err) {
			if (!isCurrent(forInstallId, seq)) return;
			setError(String(err));
			setScanning(false);
			setScanProgress(null);
		}
	}

	useEffect(() => {
		return subscribeInstalledUpdates((installId, list) => {
			if (installId !== installIdRef.current) return;
			setAddons(list);
		});
	}, []);

	useEffect(() => {
		const switched = prevInstallIdRef.current !== install.id;
		prevInstallIdRef.current = install.id;
		requestSeq.current += 1;
		// Only wipe the list when switching client tabs. Clearing on every
		// refresh (or while a modal is open) makes the list flash empty.
		if (switched) {
			setAddons([]);
			setError(null);
			busyIdsRef.current = {};
			setBusyIds({});
			setUpdateProgressById({});
			updateAllActiveRef.current = false;
			setUpdateAllRemaining(0);
			setScanning(false);
			setScanProgress(null);
			setSyncDialog(null);
			setApiDisabledDialog(null);
		}
		setLoading(false);
		// If launch/hourly check already finished for this client, show it now
		// (InstalledTab may mount after the ~3s check with no live listener).
		const already = getLastInstalledUpdateResult(install.id);
		if (already) {
			setAddons(already);
		}
		loadCached(install.id);
	}, [install.id, refreshKey]);

	function openApiDisabledDialogForFolder(folderName: string) {
		const addon = addonsRef.current.find((a) => a.folderName === folderName);
		const curseforgeUrl = resolveCurseforgeAddonUrl({
			websiteUrl: addon?.websiteUrl,
			slug: addon?.curseforgeSlug,
			id: addon?.curseforgeId,
		});
		if (!curseforgeUrl) return false;
		setApiDisabledDialog({
			addonName: addon?.title || folderName,
			curseforgeUrl,
		});
		return true;
	}

	/** Run one addon/suite update; returns false if already busy or skipped. */
	async function runUpdateJob(forInstallId: string, folderName: string): Promise<boolean> {
		if (!markBusy(folderName)) return false;
		setAddonProgress(folderName, { stage: "download", percent: 0 });
		try {
			await api.updateAddon(forInstallId, folderName, (progress) => {
				if (installIdRef.current !== forInstallId) return;
				setAddonProgress(folderName, progress);
			});
			if (installIdRef.current !== forInstallId) return true;
			// Silent so other in-flight update progress UIs stay visible.
			await loadCached(forInstallId, { silent: true });
		} catch (err) {
			if (installIdRef.current !== forInstallId) return true;
			if (
				isApiDistributionDisabledError(err) &&
				openApiDisabledDialogForFolder(folderName)
			) {
				setError(null);
			} else {
				setError(String(err));
			}
		} finally {
			if (installIdRef.current === forInstallId) {
				clearBusy(folderName);
				clearAddonProgress(folderName);
			}
		}
		return true;
	}

	async function handleUpdate(folderName: string) {
		setError(null);
		await runUpdateJob(install.id, folderName);
	}

	async function handleUpdateAll() {
		if (updateAllActiveRef.current || updateAllRemaining > 0) return;
		const forInstallId = install.id;
		const targets = addons
			.filter((a) => a.hasUpdate && !busyIdsRef.current[a.folderName])
			.map((a) => a.folderName);
		if (targets.length === 0) return;

		updateAllActiveRef.current = true;
		setError(null);
		const queue = [...targets];
		let remaining = queue.length;
		setUpdateAllRemaining(remaining);

		async function worker() {
			while (queue.length > 0) {
				if (installIdRef.current !== forInstallId) return;
				const folderName = queue.shift();
				if (!folderName) return;
				await runUpdateJob(forInstallId, folderName);
				remaining -= 1;
				if (installIdRef.current === forInstallId) {
					setUpdateAllRemaining(Math.max(0, remaining));
				}
			}
		}

		const workerCount = Math.min(UPDATE_ALL_CONCURRENCY, queue.length);
		await Promise.all(Array.from({ length: workerCount }, () => worker()));
		if (installIdRef.current === forInstallId) {
			updateAllActiveRef.current = false;
			setUpdateAllRemaining(0);
		}
	}

	async function handleDelete(folderName: string) {
		const forInstallId = install.id;
		const target = addons.find((a) => a.folderName === folderName);
		const title = target?.title || folderName;
		const suiteCount = target?.relatedFolders?.length ?? 1;
		// Native async dialog — window.confirm blocks the WebView and the list
		// appears empty behind the dialog until the user dismisses it.
		const confirmed = await ask(
			suiteCount > 1
				? t("installed.deleteSuiteConfirm", {
						name: title,
						count: String(suiteCount),
					})
				: t("installed.deleteConfirm", { name: title }),
			{
				title: t("installed.deleteTitle"),
				kind: "warning",
				okLabel: t("installed.deleteOk"),
				cancelLabel: t("installed.deleteCancel"),
			},
		);
		if (!confirmed) return;
		if (installIdRef.current !== forInstallId) return;
		if (!markBusy(folderName)) return;
		try {
			await api.deleteAddon(forInstallId, folderName);
			if (installIdRef.current !== forInstallId) return;
			setAddons((list) => list.filter((a) => a.folderName !== folderName));
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
			await loadCached(forInstallId);
		} finally {
			if (installIdRef.current === forInstallId) {
				clearBusy(folderName);
			}
		}
	}

	async function handleManualInstall() {
		const forInstallId = install.id;
		setError(null);
		try {
			const selected = await open({
				multiple: false,
				title: t("installed.pickArchive"),
				filters: [
					{
						name: t("installed.archiveFilter"),
						extensions: ["zip", "7z", "rar", "tar", "gz", "tgz"],
					},
				],
			});
			if (!selected || Array.isArray(selected)) return;
			if (installIdRef.current !== forInstallId) return;
			markBusy("__manual__");
			const installed = await api.installFromArchive(forInstallId, selected);
			if (installIdRef.current !== forInstallId) return;
			await loadCached(forInstallId);
			if (installIdRef.current !== forInstallId) return;
			const names = installed.map((item) => item.title).join("、");
			showToast(t("installed.manualDone", { names: names || selected }));
			// Recompute hasUpdate (manual installs have no file_id; version compare).
			void enrichAfterScan(forInstallId);
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
		} finally {
			if (installIdRef.current === forInstallId) {
				clearBusy("__manual__");
			}
		}
	}

	function interfaceDirFromAddonsPath(addonsPath: string): string {
		const trimmed = addonsPath.replace(/[\\/]+$/, "");
		return trimmed.replace(/[\\/]+AddOns$/i, "");
	}

	async function handleOpenInterface() {
		setError(null);
		try {
			const interfaceDir = interfaceDirFromAddonsPath(install.addonsPath);
			await api.openDir(interfaceDir);
		} catch (err) {
			setError(String(err));
		}
	}

	useEffect(() => {
		if (!syncDialog || !syncOutputRef.current) return;
		syncOutputRef.current.scrollTop = syncOutputRef.current.scrollHeight;
	}, [syncDialog?.liveLog, syncDialog?.report?.log, syncDialog?.error, syncDialog?.running]);

	useEffect(() => {
		return () => {
			if (syncFlushTimerRef.current !== null) {
				window.clearTimeout(syncFlushTimerRef.current);
				syncFlushTimerRef.current = null;
			}
		};
	}, []);

	function beginSyncDialog(kind: "backup" | "restore") {
		if (syncFlushTimerRef.current !== null) {
			window.clearTimeout(syncFlushTimerRef.current);
			syncFlushTimerRef.current = null;
		}
		syncLiveLogRef.current = "";
		setSyncDialog({
			kind,
			running: true,
			liveLog: "",
			report: null,
			error: null,
		});
	}

	function appendSyncLine(line: string) {
		syncLiveLogRef.current += `${line}\n`;
		if (syncFlushTimerRef.current !== null) return;
		syncFlushTimerRef.current = window.setTimeout(() => {
			syncFlushTimerRef.current = null;
			const liveLog = syncLiveLogRef.current;
			setSyncDialog((prev) => (prev ? { ...prev, liveLog } : prev));
		}, 50);
	}

	function finishSyncDialog(
		kind: "backup" | "restore",
		patch: Partial<SyncDialogState>,
	) {
		if (syncFlushTimerRef.current !== null) {
			window.clearTimeout(syncFlushTimerRef.current);
			syncFlushTimerRef.current = null;
		}
		setSyncDialog({
			kind,
			running: false,
			liveLog: syncLiveLogRef.current,
			report: null,
			error: null,
			...patch,
		});
	}

	async function handleBackup() {
		const forInstallId = install.id;
		setError(null);
		if (!hasBackupPath) {
			showToast(t("installed.backupNeedPath"));
			onNeedBackupPath();
			return;
		}
		markBusy("__backup__");
		beginSyncDialog("backup");
		try {
			const report = await api.backupInstall(forInstallId, appendSyncLine);
			if (installIdRef.current !== forInstallId) return;
			finishSyncDialog("backup", { report, error: null });
			showToast(t("installed.backupDone", { path: report.path }));
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			const message = String(err);
			if (message.includes("未设置备份路径")) {
				setSyncDialog(null);
				showToast(t("installed.backupNeedPath"));
				onNeedBackupPath();
				return;
			}
			finishSyncDialog("backup", { report: null, error: message });
			setError(message);
		} finally {
			if (installIdRef.current === forInstallId) {
				clearBusy("__backup__");
			}
		}
	}

	async function handleRestore() {
		const forInstallId = install.id;
		setError(null);
		if (!hasBackupPath) {
			showToast(t("installed.backupNeedPath"));
			onNeedBackupPath();
			return;
		}
		const confirmed = await ask(
			t("installed.restoreConfirm", { label: installLabel }),
			{
				title: t("installed.restoreTitle"),
				kind: "warning",
				okLabel: t("installed.restoreOk"),
				cancelLabel: t("installed.restoreCancel"),
			},
		);
		if (!confirmed) return;
		if (installIdRef.current !== forInstallId) return;
		markBusy("__restore__");
		beginSyncDialog("restore");
		let report: SyncReport | null = null;
		try {
			report = await api.restoreInstall(forInstallId, appendSyncLine);
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			const message = String(err);
			if (message.includes("未设置备份路径")) {
				setSyncDialog(null);
				showToast(t("installed.backupNeedPath"));
				onNeedBackupPath();
				return;
			}
			if (message.includes("未找到该客户端的备份目录")) {
				finishSyncDialog("restore", {
					report: null,
					error: t("installed.restoreMissing"),
				});
				setError(t("installed.restoreMissing"));
				return;
			}
			finishSyncDialog("restore", { report: null, error: message });
			setError(message);
			return;
		} finally {
			if (installIdRef.current === forInstallId && report === null) {
				clearBusy("__restore__");
			}
		}
		if (installIdRef.current !== forInstallId) return;
		finishSyncDialog("restore", { report, error: null });
		showToast(t("installed.restoreDone", { path: report.path }));
		try {
			const seq = beginRequest(forInstallId);
			if (seq === null) return;
			// Force disk+manifest re-read (backup may have swapped wow-addon-manager.json).
			const list = await api.rescanInstalledAddons(forInstallId);
			if (!isCurrent(forInstallId, seq)) return;
			// Publish before setState so remount / Browse / stale CF emits see the
			// post-restore list; bumps check generation to drop in-flight emits.
			noteInstalledDiskScan(forInstallId, list);
			setAddons(list);
			void enrichAfterScan(forInstallId);
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			// Restore already succeeded; keep success dialog and surface rescan separately.
			setError(String(err));
		} finally {
			if (installIdRef.current === forInstallId) {
				clearBusy("__restore__");
			}
		}
	}

	const blocked = loading || scanning || anyBusy;
	const updatableCount = addons.filter((a) => a.hasUpdate).length;
	const updateAllRunning = updateAllRemaining > 0;
	const showUpdateAll = updatableCount > 0 || updateAllRunning;

	return (
		<section className="installed-layout">
			<div className="installed-toolbar">
				<div>
					<h2>{t("installed.title", { label: installLabel })}</h2>
					<div className="muted">
						{install.installPath}
						{install.gameVersion ? ` · ${install.gameVersion}` : ""}
					</div>
				</div>
				<div className="toolbar-actions">
					{showUpdateAll ? (
						<button
							className="btn primary"
							onClick={handleUpdateAll}
							disabled={loading || scanning || updateAllRunning}
							title={t("installed.updateAll")}
						>
							{updateAllRunning
								? t("installed.updateAllRemaining", {
										n: String(updateAllRemaining),
									})
								: t("installed.updateAll")}
						</button>
					) : null}
					<button
						className="btn primary"
						onClick={handleManualInstall}
						disabled={blocked}
					>
						{t("installed.manual")}
					</button>
					<button
						className="btn"
						onClick={handleRescan}
						disabled={blocked}
						title={t("installed.rescan")}
					>
						{scanning ? t("installed.scanning") : t("installed.rescan")}
					</button>
					<button
						className="btn"
						onClick={handleOpenInterface}
						disabled={blocked}
						title={t("installed.openInterface")}
					>
						{t("installed.openInterface")}
					</button>
					<button
						className="btn"
						onClick={handleBackup}
						disabled={blocked}
						title={t("installed.backup")}
					>
						{busyIds.__backup__ ? t("installed.backingUp") : t("installed.backup")}
					</button>
					<button
						className="btn"
						onClick={handleRestore}
						disabled={blocked}
						title={t("installed.restore")}
					>
						{busyIds.__restore__ ? t("installed.restoring") : t("installed.restore")}
					</button>
				</div>
			</div>

			{error ? <div className="banner">{error}</div> : null}

			{syncDialog ? (
				<div className="modal-backdrop" role="presentation">
					<div
						className="modal-card sync-modal"
						role="dialog"
						aria-modal="true"
						aria-labelledby="sync-modal-title"
					>
						<div className="sync-modal-header">
							<h3 id="sync-modal-title">
								{syncDialog.kind === "backup"
									? t("installed.syncBackupTitle")
									: t("installed.syncRestoreTitle")}
							</h3>
							{syncDialog.running ? (
								<span className="muted">{t("installed.syncRunning")}</span>
							) : syncDialog.error ? (
								<span className="sync-modal-failed">{t("installed.syncFailed")}</span>
							) : syncDialog.report ? (
								<span className="sync-modal-ok">{t("installed.syncDone")}</span>
							) : null}
						</div>
						{syncDialog.report ? (
							<div className="muted sync-modal-meta">
								<div>
									{t("installed.syncItems", {
										items: syncDialog.report.items.join(" / ") || "—",
									})}
								</div>
								<div className="sync-modal-path">{syncDialog.report.path}</div>
							</div>
						) : null}
						<label className="settings-field">
							<span>{t("installed.syncOutput")}</span>
							<pre className="sync-output" ref={syncOutputRef}>
								{syncDialog.liveLog.trim()
									? syncDialog.liveLog
									: syncDialog.running
										? t("installed.syncRunning")
										: syncDialog.error ||
											syncDialog.report?.log ||
											"—"}
							</pre>
						</label>
						<div className="card-actions">
							<button
								className="btn primary"
								type="button"
								disabled={syncDialog.running}
								onClick={() => setSyncDialog(null)}
							>
								{t("installed.syncClose")}
							</button>
						</div>
					</div>
				</div>
			) : null}

			<div className="list-pane">
				{loading || scanning ? (
					<div className="list-loading" role="status" aria-live="polite">
						<div className="list-loading-spinner" aria-hidden="true" />
						<span>
							{scanning ? t("installed.rescanning") : t("installed.loadingList")}
						</span>
						{scanning && scanProgress ? (
							<span className="list-loading-progress">
								{t("installed.scanFound", {
									name: scanProgress.name,
									count: scanProgress.count,
								})}
							</span>
						) : null}
					</div>
				) : (
					<>
						{addons.map((addon) => (
							<AddonCard
								key={`${install.id}:${addon.folderName}`}
								mode="installed"
								addon={addon}
								busy={Boolean(busyIds[addon.folderName])}
								updateProgress={
									updateProgressById[addon.folderName] ?? null
								}
								onUpdate={() => handleUpdate(addon.folderName)}
								onDelete={() => handleDelete(addon.folderName)}
								onOpenDetail={(modId, title) => setDetail({ modId, title })}
							/>
						))}
						{addons.length === 0 ? (
							<div className="empty">
								<strong>{t("installed.emptyTitle")}</strong>
								{t("installed.emptyHint")}
							</div>
						) : null}
					</>
				)}
			</div>
			{detail ? (
				<AddonDetailModal
					modId={detail.modId}
					fallbackTitle={detail.title}
					onClose={() => setDetail(null)}
				/>
			) : null}
			{apiDisabledDialog ? (
				<ApiDistributionDisabledDialog
					addonName={apiDisabledDialog.addonName}
					curseforgeUrl={apiDisabledDialog.curseforgeUrl}
					onClose={() => setApiDisabledDialog(null)}
				/>
			) : null}
			{toastNode}
		</section>
	);
}
