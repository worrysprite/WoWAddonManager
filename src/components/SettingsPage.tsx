import { useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import type { LogEntry, SettingsView } from "../types";

type Props = {
	onBack: () => void;
	onSaved: (settings: SettingsView) => void;
};

const LOG_LEVELS = ["error", "warn", "info", "debug", "trace"] as const;

export function SettingsPage({ onBack, onSaved }: Props) {
	const [settings, setSettings] = useState<SettingsView | null>(null);
	const [apiKey, setApiKey] = useState("");
	const [saving, setSaving] = useState(false);
	const [busy, setBusy] = useState(false);
	const [message, setMessage] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [logLevel, setLogLevel] = useState("warn");
	const [logEntries, setLogEntries] = useState<LogEntry[]>([]);
	const [logsOpen, setLogsOpen] = useState(false);
	const [logsBusy, setLogsBusy] = useState(false);
	const logPaneRef = useRef<HTMLPreElement | null>(null);

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

	async function refreshLogs() {
		const data = await api.getLogs();
		setLogLevel(data.level);
		setLogEntries(data.entries);
	}

	async function handleOpenLogs() {
		setLogsBusy(true);
		setError(null);
		try {
			await refreshLogs();
			setLogsOpen(true);
		} catch (err) {
			setError(String(err));
		} finally {
			setLogsBusy(false);
		}
	}

	async function handleChangeLogLevel(nextLevel: string) {
		setLogsBusy(true);
		setError(null);
		try {
			const applied = await api.setLogLevel(nextLevel);
			setLogLevel(applied);
			await refreshLogs();
			setMessage(`日志级别已设为 ${applied.toUpperCase()}`);
		} catch (err) {
			setError(String(err));
		} finally {
			setLogsBusy(false);
		}
	}

	async function handleClearLogs() {
		setLogsBusy(true);
		setError(null);
		try {
			await api.clearLogs();
			await refreshLogs();
			setMessage("日志已清空");
		} catch (err) {
			setError(String(err));
		} finally {
			setLogsBusy(false);
		}
	}

	async function handleSaveLogs() {
		setLogsBusy(true);
		setError(null);
		setMessage(null);
		try {
			const stamp = new Date().toISOString().replace(/[:.]/g, "-");
			const selected = await save({
				title: "保存日志文件",
				defaultPath: `wow-addon-manager-${stamp}.log`,
				filters: [{ name: "Log", extensions: ["log", "txt"] }],
			});
			if (!selected) return;
			await api.saveLogs(selected);
			setMessage(`日志已保存：${selected}`);
			await refreshLogs();
		} catch (err) {
			setError(String(err));
		} finally {
			setLogsBusy(false);
		}
	}

	useEffect(() => {
		if (!logsOpen) return;
		const timer = window.setInterval(() => {
			refreshLogs().catch(() => undefined);
		}, 1500);
		return () => window.clearInterval(timer);
	}, [logsOpen]);

	useEffect(() => {
		if (!logsOpen || !logPaneRef.current) return;
		logPaneRef.current.scrollTop = logPaneRef.current.scrollHeight;
	}, [logsOpen, logEntries.length]);

	return (
		<section className="settings-page">
			<div className="settings-header">
				<button className="btn" onClick={onBack}>
					← 返回
				</button>
				<div>
					<h2>设置</h2>
					<p className="muted">配置 API Key、魔兽世界安装目录与日志</p>
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

			<div className="settings-card">
				<h3 className="settings-card-title">日志</h3>
				<p className="muted settings-hint">
					默认级别为 WARN。打开后可查看内存中的运行日志，并保存为文件。
				</p>
				<div className="card-actions">
					<button
						className="btn primary"
						type="button"
						disabled={logsBusy}
						onClick={logsOpen ? () => setLogsOpen(false) : handleOpenLogs}
					>
						{logsOpen ? "收起日志" : "打开日志"}
					</button>
					<button
						className="btn"
						type="button"
						disabled={logsBusy || !logsOpen}
						onClick={handleSaveLogs}
					>
						保存日志文件
					</button>
					<button
						className="btn"
						type="button"
						disabled={logsBusy || !logsOpen}
						onClick={handleClearLogs}
					>
						清空
					</button>
				</div>

				{logsOpen ? (
					<>
						<label className="settings-field">
							<span>日志级别</span>
							<select
								value={logLevel}
								disabled={logsBusy}
								onChange={(e) => handleChangeLogLevel(e.target.value)}
							>
								{LOG_LEVELS.map((level) => (
									<option key={level} value={level}>
										{level.toUpperCase()}
									</option>
								))}
							</select>
						</label>
						<pre className="log-pane" ref={logPaneRef}>
							{logEntries.length === 0
								? "（暂无日志）"
								: logEntries
										.map(
											(entry) =>
												`${entry.timestamp} [${entry.level.padEnd(5)}] ${entry.target}: ${entry.message}`,
										)
										.join("\n")}
						</pre>
					</>
				) : null}
			</div>
		</section>
	);
}
