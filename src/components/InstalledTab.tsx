import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import type { InstalledAddon, WowInstall } from "../types";
import { AddonCard } from "./AddonCard";

type Props = {
	install: WowInstall;
	refreshKey: number;
};

export function InstalledTab({ install, refreshKey }: Props) {
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
			setStatus(`已重新扫描，共 ${list.length} 个插件`);
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
		// Drop any in-flight result from the previous client immediately.
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
		if (!window.confirm(`确定删除插件 ${folderName}？`)) return;
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
				title: "选择插件压缩包",
				filters: [
					{
						name: "Addon Archive",
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
			setStatus(`已手动安装：${names || selected}`);
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
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
					<h2>{install.label} 已安装插件</h2>
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
						手动安装
					</button>
					<button className="btn" onClick={handleRescan} disabled={blocked}>
						{scanning ? "扫描中…" : "重新扫描"}
					</button>
				</div>
			</div>

			{error ? <div className="banner">{error}</div> : null}
			{status ? <div className="banner ok-banner">{status}</div> : null}

			<div className="list-pane">
				{(loading || scanning) && addons.length === 0 ? (
					<div className="empty">
						{scanning ? "正在重新扫描插件目录…" : "正在读取插件列表…"}
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
						<strong>还没有已安装插件</strong>
						可从 Browse All 安装，或点击右上角「手动安装」选择压缩包。
					</div>
				) : null}
			</div>
		</section>
	);
}
