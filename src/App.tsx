import { useEffect, useState } from "react";
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

	async function applySettings(next: SettingsView) {
		setSettings(next);
		setInstalls(next.detectedInstalls);
		if (
			tab !== "browse" &&
			tab !== "settings" &&
			!next.detectedInstalls.some((i) => i.id === tab)
		) {
			setTab("browse");
		}
	}

	async function refreshAll() {
		try {
			const next = await api.getSettings();
			await applySettings(next);
		} catch (err) {
			setStatus(String(err));
		}
	}

	async function handleRescanClients() {
		setStatus("正在重新扫描魔兽客户端…");
		try {
			const next = await api.rescanWowInstalls();
			await applySettings(next);
			setStatus(
				next.hasWowInstall
					? `已重新扫描，发现 ${next.detectedInstalls.length} 个客户端`
					: "已重新扫描，未发现客户端",
			);
		} catch (err) {
			setStatus(String(err));
		}
	}

	useEffect(() => {
		refreshAll();
		api.getApiHelp().then(setApiHelp).catch(console.error);
	}, []);

	function openSettings() {
		if (tab !== "settings") {
			setPreviousTab(tab);
		}
		setTab("settings");
	}

	const showApiWarning = Boolean(apiHelp && !settings?.hasApiKey);
	const showWowWarning = Boolean(settings && !settings.hasWowInstall);
	const showInvalidWowWarning = Boolean(settings?.invalidWowRoots?.length);

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
					<button className="btn" onClick={handleRescanClients}>
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

			{showApiWarning ? (
				<div className="banner">
					尚未配置 API Key，当前使用网页 GET / 演示数据。请到
					<button className="linkish" onClick={openSettings}>
						设置
					</button>
					填写 Key 后改用官方接口。
					{apiHelp ? (
						<>
							<a href={apiHelp.apply} target="_blank" rel="noreferrer">
								申请表
							</a>
							{" · "}
							<a href={apiHelp.docs} target="_blank" rel="noreferrer">
								文档
							</a>
						</>
					) : null}
				</div>
			) : null}

			{settings?.hasApiKey ? (
				<div className="banner ok-banner">已启用 CurseForge 官方 API。</div>
			) : null}

			{showWowWarning ? (
				<div className="banner">
					未检测到魔兽世界安装目录。请到
					<button className="linkish" onClick={openSettings}>
						设置
					</button>
					手动添加「World of Warcraft」根目录，或点击右上角「重新扫描」。
				</div>
			) : null}

			{showInvalidWowWarning ? (
				<div className="banner">
					部分手动设置的安装目录无效：{settings?.invalidWowRoots.join("；")}。请到
					<button className="linkish" onClick={openSettings}>
						设置
					</button>
					修正或移除。
				</div>
			) : null}

			<main className="content">
				{tab === "settings" ? (
					<SettingsPage
						onBack={() => setTab(previousTab === "settings" ? "browse" : previousTab)}
						onSaved={(next) => {
							applySettings(next);
							// Remount browse so it refetches after API key / cache invalidation.
							setBrowseNonce((v) => v + 1);
						}}
					/>
				) : null}

				{/* Keep browse mounted across tab switches to avoid re-requesting the same query. */}
				<div style={{ display: tab === "browse" ? undefined : "none" }}>
					<BrowseTab
						key={browseNonce}
						installs={installs}
						onInstalled={() => {
							setRefreshKey((v) => v + 1);
							setStatus("插件已安装（或写入占位目录）");
						}}
					/>
				</div>

				{tab !== "settings" && tab !== "browse"
					? (() => {
							const install = installs.find((i) => i.id === tab);
							return install ? (
								<InstalledTab
									key={install.id}
									install={install}
									refreshKey={refreshKey}
								/>
							) : (
								<div className="empty">未找到该客户端</div>
							);
						})()
					: null}
			</main>
		</div>
	);
}
