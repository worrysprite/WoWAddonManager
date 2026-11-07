import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import { useI18n } from "../i18n";
import type { InstalledAddon, WowInstall } from "../types";
import { AddonCard } from "./AddonCard";

type Props = {
	install: WowInstall;
	refreshKey: number;
};

export function InstalledTab({ install, refreshKey }: Props) {
	const { t } = useI18n();
	const [addons, setAddons] = useState<InstalledAddon[]>([]);
	const [loading, setLoading] = useState(false);
	const [scanning, setScanning] = useState(false);
	const [busy, setBusy] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [status, setStatus] = useState<string | null>(null);
	const requestSeq = useRef(0);
	const installIdRef = useRef(install.id);
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
		requestSeq.current += 1;
		setAddons([]);
		setError(null);
		setStatus(null);
		setBusy(null);
		setScanning(false);
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
		if (!window.confirm(t("installed.deleteConfirm", { name: folderName }))) return;
		const forInstallId = install.id;
		setBusy(folderName);
		setStatus(null);
		try {
			await api.deleteAddon(forInstallId, folderName);
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
				</div>
			</div>

			{error ? <div className="banner">{error}</div> : null}
			{status ? <div className="banner ok-banner">{status}</div> : null}

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
