import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "./api";
import { BrowseTab } from "./components/BrowseTab";
import { InstalledTab } from "./components/InstalledTab";
import { SettingsPage } from "./components/SettingsPage";
import type { ApiHelp, SettingsView, WowInstall } from "./types";
import "./styles.css";

type TabId = "browse" | "settings" | string;

export default function App() {
	const [tab, setTab] = useState<TabId>("browse");
	const [previousTab, setPreviousTab] = useState<TabId>("browse");
	const [installs, setInstalls] = useState<WowInstall[]>([]);
	const [refreshKey, setRefreshKey] = useState(0);
	const [browseNonce, setBrowseNonce] = useState(0);
	const [apiHelp, setApiHelp] = useState<ApiHelp | null>(null);
	const [settings, setSettings] = useState<SettingsView | null>(null);
	const [status, setStatus] = useState<string | null>(null);

	async function refreshInstalls() {
		try {
			const list = await api.detectWowInstalls();
			setInstalls(list);
			if (tab !== "browse" && tab !== "settings" && !list.some((i) => i.id === tab)) {
				setTab("browse");
			}
		} catch (err) {
			setStatus(String(err));
		}
	}

	useEffect(() => {
		refreshInstalls();
		api.getApiHelp().then(setApiHelp).catch(console.error);
		api.getSettings().then(setSettings).catch(console.error);
	}, []);

	async function handleAddPath() {
		try {
			const selected = await open({
				directory: true,
				multiple: false,
				title: "选择 World of Warcraft 安装目录",
			});
			if (!selected || Array.isArray(selected)) return;
			const list = await api.addWowPath(selected);
			setInstalls(list);
			setStatus(`已添加：${selected}`);
		} catch (err) {
			setStatus(String(err));
		}
	}

	function openSettings() {
		if (tab !== "settings") {
			setPreviousTab(tab);
		}
		setTab("settings");
	}

	return (
		<div className="app-shell">
			<header className="topbar">
				<div className="brand">
					<div className="brand-mark">W</div>
					<div className="brand-title">WoW Addon Manager</div>
				</div>

				<nav className="tab-row">
					<button
						className={`tab-btn ${tab === "browse" ? "active" : ""}`}
						onClick={() => setTab("browse")}
					>
						Browse All
					</button>
					{installs.map((install) => (
						<button
							key={install.id}
							className={`tab-btn ${tab === install.id ? "active" : ""}`}
							onClick={() => setTab(install.id)}
						>
							{install.label}
						</button>
					))}
				</nav>

				<div className="toolbar-actions">
					<button className="btn" onClick={handleAddPath}>
						添加安装目录
					</button>
					<button className="btn" onClick={refreshInstalls}>
						重新扫描
					</button>
					<button
						className={`btn settings-btn ${tab === "settings" ? "active-settings" : ""}`}
						onClick={openSettings}
						title="设置"
					>
						设置
						{settings?.hasApiKey ? <span className="settings-dot" /> : null}
					</button>
				</div>
			</header>

			{status ? <div className="banner">{status}</div> : null}
			{apiHelp && tab === "browse" && !settings?.hasApiKey ? (
				<div className="banner">
					尚未配置 API Key，当前使用网页 GET / 演示数据。点击右上角
					<button className="linkish" onClick={openSettings}>
						设置
					</button>
					填写 Key 后改用官方接口。
					<a href={apiHelp.apply} target="_blank" rel="noreferrer">
						申请表
					</a>
					{" · "}
					<a href={apiHelp.docs} target="_blank" rel="noreferrer">
						文档
					</a>
				</div>
			) : null}
			{tab === "browse" && settings?.hasApiKey ? (
				<div className="banner ok-banner">已启用 CurseForge 官方 API。</div>
			) : null}

			<main className="content">
				{tab === "settings" ? (
					<SettingsPage
						onBack={() => setTab(previousTab === "settings" ? "browse" : previousTab)}
						onSaved={(next) => {
							setSettings(next);
							setBrowseNonce((v) => v + 1);
						}}
					/>
				) : tab === "browse" ? (
					<BrowseTab
						key={browseNonce}
						installs={installs}
						onInstalled={() => {
							setRefreshKey((v) => v + 1);
							setStatus("插件已安装（或写入占位目录）");
						}}
					/>
				) : (
					(() => {
						const install = installs.find((i) => i.id === tab);
						return install ? (
							<InstalledTab install={install} refreshKey={refreshKey} />
						) : (
							<div className="empty">未找到该客户端</div>
						);
					})()
				)}
			</main>
		</div>
	);
}
