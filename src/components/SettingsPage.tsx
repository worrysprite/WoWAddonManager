import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import type { SettingsView } from "../types";

type Props = {
	onBack: () => void;
	onSaved: (settings: SettingsView) => void;
};

export function SettingsPage({ onBack, onSaved }: Props) {
	const [settings, setSettings] = useState<SettingsView | null>(null);
	const [apiKey, setApiKey] = useState("");
	const [saving, setSaving] = useState(false);
	const [busy, setBusy] = useState(false);
	const [message, setMessage] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);

	async function reload() {
		const data = await api.getSettings();
		setSettings(data);
		setApiKey(data.curseforgeApiKey);
		onSaved(data);
		return data;
	}

	useEffect(() => {
		reload().catch((err) => setError(String(err)));
	}, []);

	async function handleSaveApiKey(e: React.FormEvent) {
		e.preventDefault();
		setSaving(true);
		setError(null);
		setMessage(null);
		try {
			const next = await api.saveSettings(apiKey, settings?.wowRoots);
			setSettings(next);
			setApiKey(next.curseforgeApiKey);
			setMessage(
				next.hasApiKey
					? "已保存 API Key。浏览与下载将使用 CurseForge 官方 API。"
					: "已清空 API Key。将回退到网页 GET / 演示数据。",
			);
			onSaved(next);
		} catch (err) {
			setError(String(err));
		} finally {
			setSaving(false);
		}
	}

	async function handleAddWowPath() {
		setBusy(true);
		setError(null);
		setMessage(null);
		try {
			const selected = await open({
				directory: true,
				multiple: false,
				title: "选择 World of Warcraft 安装目录",
			});
			if (!selected || Array.isArray(selected)) return;
			const next = await api.addWowPath(selected);
			setSettings(next);
			setMessage(`已添加安装目录：${selected}`);
			onSaved(next);
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(false);
		}
	}

	async function handleRemoveWowPath(path: string) {
		setBusy(true);
		setError(null);
		setMessage(null);
		try {
			const next = await api.removeWowPath(path);
			setSettings(next);
			setMessage(`已移除：${path}`);
			onSaved(next);
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(false);
		}
	}

	async function handleRescan() {
		setBusy(true);
		setError(null);
		setMessage(null);
		try {
			const next = await api.rescanWowInstalls();
			setSettings(next);
			setMessage(
				next.hasWowInstall
					? `重新扫描完成，检测到 ${next.detectedInstalls.length} 个客户端。`
					: "重新扫描完成，仍未检测到魔兽世界客户端。",
			);
			onSaved(next);
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(false);
		}
	}

	return (
		<section className="settings-page">
			<div className="settings-header">
				<button className="btn" onClick={onBack}>
					← 返回
				</button>
				<div>
					<h2>设置</h2>
					<p className="muted">配置 API Key 与魔兽世界安装目录</p>
				</div>
			</div>

			{error ? <div className="banner">{error}</div> : null}
			{message ? <div className="banner ok-banner">{message}</div> : null}

			<form className="settings-card" onSubmit={handleSaveApiKey}>
				<h3 className="settings-card-title">CurseForge API</h3>
				<label className="settings-field">
					<span>CurseForge API Key</span>
					<input
						type="password"
						autoComplete="off"
						spellCheck={false}
						placeholder="粘贴 x-api-key"
						value={apiKey}
						onChange={(e) => setApiKey(e.target.value)}
					/>
				</label>

				<p className="muted settings-hint">
					填写后，搜索与下载走官方 API（`api.curseforge.com`）。清空并保存则恢复网页 GET /
					演示数据模式。
					{settings?.hasApiKey ? " 当前已启用官方 API。" : " 当前未配置 Key。"}
				</p>

				<div className="settings-links muted">
					{settings ? (
						<>
							<a href={settings.applyUrl} target="_blank" rel="noreferrer">
								申请 API Key
							</a>
							<span>·</span>
							<a href={settings.docsUrl} target="_blank" rel="noreferrer">
								API 文档
							</a>
						</>
					) : null}
				</div>

				<div className="card-actions">
					<button className="btn primary" type="submit" disabled={saving}>
						{saving ? "保存中…" : "保存 API Key"}
					</button>
					<button
						className="btn"
						type="button"
						disabled={saving || !apiKey}
						onClick={() => setApiKey("")}
					>
						清空输入
					</button>
				</div>
			</form>

			<div className="settings-card">
				<h3 className="settings-card-title">魔兽世界安装目录</h3>
				<p className="muted settings-hint">
					启动时会自动扫描注册表、Battle.net Agent、常见盘符路径。若未找到，可在此手动添加
					「World of Warcraft」根目录，或其中的 `_retail_` / `_classic_` 等版本目录。
				</p>

				<div className="card-actions">
					<button className="btn primary" type="button" disabled={busy} onClick={handleAddWowPath}>
						添加安装目录
					</button>
					<button className="btn" type="button" disabled={busy} onClick={handleRescan}>
						重新扫描
					</button>
				</div>

				{settings?.detectedInstalls?.length ? (
					<div className="settings-list">
						<div className="settings-list-label">已检测到的客户端</div>
						{settings.detectedInstalls.map((install) => (
							<div key={install.id} className="settings-list-item">
								<div>
									<strong>{install.label}</strong>
									<div className="muted">{install.installPath}</div>
								</div>
								<span className="muted">
									{install.gameVersion || "版本未知"}
								</span>
							</div>
						))}
					</div>
				) : (
					<div className="banner">尚未检测到魔兽世界客户端。</div>
				)}

				{settings?.wowRoots?.length ? (
					<div className="settings-list">
						<div className="settings-list-label">手动添加的路径</div>
						{settings.wowRoots.map((root) => (
							<div key={root} className="settings-list-item">
								<div className="muted">{root}</div>
								<button
									className="btn danger"
									type="button"
									disabled={busy}
									onClick={() => handleRemoveWowPath(root)}
								>
									移除
								</button>
							</div>
						))}
					</div>
				) : null}

				{settings?.invalidWowRoots?.length ? (
					<div className="banner">
						以下手动路径无效或不包含可识别的客户端：
						{settings.invalidWowRoots.join("；")}
					</div>
				) : null}
			</div>
		</section>
	);
}
