import { useEffect, useRef, useState } from "react";
import { ask, open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import { useI18n } from "../i18n";
import type { InstalledAddon, SyncReport, WowInstall } from "../types";
import { AddonCard } from "./AddonCard";

type SyncDialogState = {
	kind: "backup" | "restore";
	running: boolean;
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
	const { t } = useI18n();
	const [addons, setAddons] = useState<InstalledAddon[]>([]);
	const [loading, setLoading] = useState(false);
	const [scanning, setScanning] = useState(false);
	const [busy, setBusy] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [status, setStatus] = useState<string | null>(null);
	const [syncDialog, setSyncDialog] = useState<SyncDialogState | null>(null);
	const syncOutputRef = useRef<HTMLPreElement | null>(null);
	const requestSeq = useRef(0);
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

	async function loadCached(forInstallId = install.id) {
		const seq = beginRequest(forInstallId);
		if (seq === null) return;
		setLoading(true);
		setError(null);
		try {
			const list = await api.listInstalledAddons(forInstallId);
			if (!isCurrent(forInstallId, seq)) return;
			setAddons(list);
		} catch (err) {
			if (!isCurrent(forInstallId, seq)) return;
			setError(String(err));
		} finally {
			if (isCurrent(forInstallId, seq)) {
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
		setStatus(null);
		try {
			const list = await api.rescanInstalledAddons(forInstallId);
			if (!isCurrent(forInstallId, seq)) return;
			setAddons(list);
			setStatus(t("installed.rescanDone", { count: list.length }));
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
			setStatus(null);
			setBusy(null);
			setScanning(false);
			setSyncDialog(null);
		}
		setLoading(false);
		loadCached(install.id);
	}, [install.id, refreshKey]);

	async function handleUpdate(folderName: string) {
		const forInstallId = install.id;
		setBusy(folderName);
		setStatus(null);
		try {
			await api.updateAddon(forInstallId, folderName);
			if (installIdRef.current !== forInstallId) return;
			await loadCached(forInstallId);
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
		} finally {
			if (installIdRef.current === forInstallId) {
				setBusy(null);
			}
		}
	}

	async function handleDelete(folderName: string) {
		const forInstallId = install.id;
		const title =
			addons.find((a) => a.folderName === folderName)?.title || folderName;
		// Native async dialog — window.confirm blocks the WebView and the list
		// appears empty behind the dialog until the user dismisses it.
		const confirmed = await ask(
			t("installed.deleteConfirm", { name: title }),
			{
				title: t("installed.deleteTitle"),
				kind: "warning",
				okLabel: t("installed.deleteOk"),
				cancelLabel: t("installed.deleteCancel"),
			},
		);
		if (!confirmed) return;
		if (installIdRef.current !== forInstallId) return;
		setBusy(folderName);
		setStatus(null);
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
				setBusy(null);
			}
		}
	}

	async function handleManualInstall() {
		const forInstallId = install.id;
		setError(null);
		setStatus(null);
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
			setBusy("__manual__");
			const installed = await api.installFromArchive(forInstallId, selected);
			if (installIdRef.current !== forInstallId) return;
			await loadCached(forInstallId);
			if (installIdRef.current !== forInstallId) return;
			const names = installed.map((item) => item.title).join("、");
			setStatus(t("installed.manualDone", { names: names || selected }));
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
		} finally {
			if (installIdRef.current === forInstallId) {
				setBusy(null);
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
	}, [syncDialog?.report?.log, syncDialog?.error, syncDialog?.running]);

	async function handleBackup() {
		const forInstallId = install.id;
		setError(null);
		setStatus(null);
		if (!hasBackupPath) {
			setStatus(t("installed.backupNeedPath"));
			onNeedBackupPath();
			return;
		}
		setBusy("__backup__");
		setSyncDialog({ kind: "backup", running: true, report: null, error: null });
		try {
			const report = await api.backupInstall(forInstallId);
			if (installIdRef.current !== forInstallId) return;
			setSyncDialog({ kind: "backup", running: false, report, error: null });
			setStatus(t("installed.backupDone", { path: report.path }));
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			const message = String(err);
			if (message.includes("未设置备份路径")) {
				setSyncDialog(null);
				setStatus(t("installed.backupNeedPath"));
				onNeedBackupPath();
				return;
			}
			setSyncDialog({
				kind: "backup",
				running: false,
				report: null,
				error: message,
			});
			setError(message);
		} finally {
			if (installIdRef.current === forInstallId) {
				setBusy(null);
			}
		}
	}

	async function handleRestore() {
		const forInstallId = install.id;
		setError(null);
		setStatus(null);
		if (!hasBackupPath) {
			setStatus(t("installed.backupNeedPath"));
			onNeedBackupPath();
			return;
		}
		const confirmed = await ask(
			t("installed.restoreConfirm", { label: install.label }),
			{
				title: t("installed.restoreTitle"),
				kind: "warning",
				okLabel: t("installed.restoreOk"),
				cancelLabel: t("installed.restoreCancel"),
			},
		);
		if (!confirmed) return;
		if (installIdRef.current !== forInstallId) return;
		setBusy("__restore__");
		setSyncDialog({ kind: "restore", running: true, report: null, error: null });
		try {
			const report = await api.restoreInstall(forInstallId);
			if (installIdRef.current !== forInstallId) return;
			const seq = beginRequest(forInstallId);
			if (seq === null) return;
			const list = await api.rescanInstalledAddons(forInstallId);
			if (!isCurrent(forInstallId, seq)) return;
			setAddons(list);
			setSyncDialog({ kind: "restore", running: false, report, error: null });
			setStatus(t("installed.restoreDone", { path: report.path }));
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			const message = String(err);
			if (message.includes("未设置备份路径")) {
				setSyncDialog(null);
				setStatus(t("installed.backupNeedPath"));
				onNeedBackupPath();
				return;
			}
			if (message.includes("未找到该客户端的备份目录")) {
				setSyncDialog({
					kind: "restore",
					running: false,
					report: null,
					error: t("installed.restoreMissing"),
				});
				setError(t("installed.restoreMissing"));
				return;
			}
			setSyncDialog({
				kind: "restore",
				running: false,
				report: null,
				error: message,
			});
			setError(message);
		} finally {
			if (installIdRef.current === forInstallId) {
				setBusy(null);
			}
		}
	}

	const blocked = loading || scanning || busy !== null;

	return (
		<section className="installed-layout">
			<div className="installed-toolbar">
				<div>
					<h2>{t("installed.title", { label: install.label })}</h2>
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
						{busy === "__backup__" ? t("installed.backingUp") : t("installed.backup")}
					</button>
					<button
						className="btn"
						onClick={handleRestore}
						disabled={blocked}
						title={t("installed.restore")}
					>
						{busy === "__restore__" ? t("installed.restoring") : t("installed.restore")}
					</button>
				</div>
			</div>

			{error ? <div className="banner">{error}</div> : null}
			{status ? <div className="banner ok-banner">{status}</div> : null}

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
								<span className="sync-modal-ok">
									{t("installed.syncDone", { engine: syncDialog.report.engine })}
								</span>
							) : null}
						</div>
						{syncDialog.report ? (
							<div className="muted sync-modal-meta">
								<div>
									{t("installed.syncEngine", { engine: syncDialog.report.engine })}
								</div>
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
								{syncDialog.running
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
				{(loading || scanning) && addons.length === 0 ? (
					<div className="empty">
						{scanning ? t("installed.rescanning") : t("installed.loadingList")}
					</div>
				) : null}
				{addons.map((addon) => (
					<AddonCard
						key={`${install.id}:${addon.folderName}`}
						mode="installed"
						addon={addon}
						busy={busy === addon.folderName}
						onUpdate={() => handleUpdate(addon.folderName)}
						onDelete={() => handleDelete(addon.folderName)}
					/>
				))}
				{!loading && !scanning && addons.length === 0 ? (
					<div className="empty">
						<strong>{t("installed.emptyTitle")}</strong>
						{t("installed.emptyHint")}
					</div>
				) : null}
			</div>
		</section>
	);
}
