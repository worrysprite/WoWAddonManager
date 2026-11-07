import { useEffect, useState } from "react";
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
	const [message, setMessage] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		api
			.getSettings()
			.then((data) => {
				setSettings(data);
				setApiKey(data.curseforgeApiKey);
			})
			.catch((err) => setError(String(err)));
	}, []);

	async function handleSave(e: React.FormEvent) {
		e.preventDefault();
		setSaving(true);
		setError(null);
		setMessage(null);
		try {
			const next = await api.saveSettings(apiKey);
			setSettings(next);
			setApiKey(next.curseforgeApiKey);
			setMessage(
				next.hasApiKey
					? "已保存。浏览与下载将使用 CurseForge 官方 API。"
					: "已清空 API Key。将回退到网页 GET / 演示数据。",
			);
			onSaved(next);
		} catch (err) {
			setError(String(err));
		} finally {
			setSaving(false);
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
					<p className="muted">配置 CurseForge API Key 以启用官方接口</p>
				</div>
			</div>

			{error ? <div className="banner">{error}</div> : null}
			{message ? <div className="banner ok-banner">{message}</div> : null}

			<form className="settings-card" onSubmit={handleSave}>
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
						{saving ? "保存中…" : "保存"}
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
		</section>
	);
}
