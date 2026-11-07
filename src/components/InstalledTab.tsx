import { useEffect, useState } from "react";
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
	const [busy, setBusy] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [status, setStatus] = useState<string | null>(null);

	async function reload() {
		setLoading(true);
		setError(null);
		try {
			const list = await api.listInstalledAddons(install.id);
			setAddons(list);
		} catch (err) {
			setError(String(err));
		} finally {
			setLoading(false);
		}
	}

	useEffect(() => {
		reload();
	}, [install.id, refreshKey]);

	async function handleUpdate(folderName: string) {
		setBusy(folderName);
		setStatus(null);
		try {
			await api.updateAddon(install.id, folderName);
			await reload();
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(null);
		}
	}

	async function handleDelete(folderName: string) {
		if (!window.confirm(`确定删除插件 ${folderName}？`)) return;
		setBusy(folderName);
		setStatus(null);
		try {
			await api.deleteAddon(install.id, folderName);
			await reload();
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(null);
		}
	}

	async function handleManualInstall() {
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
			setBusy("__manual__");
			const installed = await api.installFromArchive(install.id, selected);
			await reload();
			const names = installed.map((item) => item.title).join("、");
			setStatus(`已手动安装：${names || selected}`);
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(null);
		}
	}

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
						disabled={loading || busy !== null}
					>
						手动安装
					</button>
					<button className="btn" onClick={reload} disabled={loading || busy !== null}>
						刷新
					</button>
				</div>
			</div>

			{error ? <div className="banner">{error}</div> : null}
			{status ? <div className="banner ok-banner">{status}</div> : null}

			<div className="list-pane">
				{loading && addons.length === 0 ? (
					<div className="empty">正在扫描插件目录…</div>
				) : null}
				{addons.map((addon) => (
					<AddonCard
						key={addon.folderName}
						mode="installed"
						addon={addon}
						busy={busy === addon.folderName}
						onUpdate={() => handleUpdate(addon.folderName)}
						onDelete={() => handleDelete(addon.folderName)}
					/>
				))}
				{!loading && addons.length === 0 ? (
					<div className="empty">
						<strong>还没有已安装插件</strong>
						可从 Browse All 安装，或点击右上角「手动安装」选择压缩包
					</div>
				) : null}
			</div>
		</section>
	);
}
