import { useEffect, useRef, useState } from "react";
import { ask, open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import { useToast } from "../hooks/useToast";
import { useI18n } from "../i18n";
import { translateInstallLabel } from "../i18n/catalogLabels";
import type {
	AddonJobProgress,
	InstalledAddon,
	SyncReport,
	WowInstall,
} from "../types";
import { AddonCard } from "./AddonCard";
import { AddonDetailModal } from "./AddonDetailModal";

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
	/** Per-addon / per-op busy keys (folderName, or __manual__/__backup__/__restore__). */
	const [busyIds, setBusyIds] = useState<Record<string, true>>({});
	const [updateProgressById, setUpdateProgressById] = useState<
		Record<string, AddonJobProgress>
	>({});
	const [error, setError] = useState<string | null>(null);
	const [syncDialog, setSyncDialog] = useState<SyncDialogState | null>(null);
	const [detail, setDetail] = useState<{ modId: number; title: string } | null>(
		null,
	);
	const syncOutputRef = useRef<HTMLPreElement | null>(null);
	const syncLiveLogRef = useRef("");
	const syncFlushTimerRef = useRef<number | null>(null);
	const requestSeq = useRef(0);
	const busyIdsRef = useRef<Record<string, true>>({});
	const installIdRef = useRef(install.id);
	const prevInstallIdRef = useRef(install.id);
	installIdRef.current = install.id;

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
		if (!silent) {
			setLoading(true);
			setError(null);
		}
		try {
			const list = await api.listInstalledAddons(forInstallId);
			if (!isCurrent(forInstallId, seq)) return;
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
		setError(null);
		try {
			const list = await api.rescanInstalledAddons(forInstallId);
			if (!isCurrent(forInstallId, seq)) return;
			setAddons(list);
			showToast(t("installed.rescanDone", { count: list.length }));
		} catch (err) {
			if (!isCurrent(forInstallId, seq)) return;
			setError(String(err));
		} finally {
			if (isCurrent(forInstallId, seq)) {
				setScanning(false);
			}
		}
	}

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
			setScanning(false);
			setSyncDialog(null);
		}
		setLoading(false);
		loadCached(install.id);
	}, [install.id, refreshKey]);

	async function handleUpdate(folderName: string) {
		const forInstallId = install.id;
		if (!markBusy(folderName)) return;
		setAddonProgress(folderName, { stage: "download", percent: 0 });
		setError(null);
		try {
			await api.updateAddon(forInstallId, folderName, (progress) => {
				if (installIdRef.current !== forInstallId) return;
				setAddonProgress(folderName, progress);
			});
			if (installIdRef.current !== forInstallId) return;
			// Silent so other in-flight update progress UIs stay visible.
			await loadCached(forInstallId, { silent: true });
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
		} finally {
			if (installIdRef.current === forInstallId) {
				clearBusy(folderName);
				clearAddonProgress(folderName);
			}
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
			const list = await api.rescanInstalledAddons(forInstallId);
			if (!isCurrent(forInstallId, seq)) return;
			setAddons(list);
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
			{toastNode}
		</section>
	);
}
