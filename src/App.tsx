import { useEffect, useState } from "react";
import * as api from "./api";
import { BrowseTab } from "./components/BrowseTab";
import { InstalledTab } from "./components/InstalledTab";
import { SettingsPage } from "./components/SettingsPage";
import { useI18n, type LocalePreference } from "./i18n";
import type { ApiHelp, SettingsView, WowInstall } from "./types";
import "./styles.css";

type TabId = "browse" | "settings" | string;

export default function App() {
	const { t, setPreference, setSystemLocale } = useI18n();
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
		setSystemLocale(next.systemLocale);
		const pref = (next.locale || "system") as LocalePreference;
		setPreference(pref === "zh-CN" || pref === "en" ? pref : "system");
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
		setStatus(t("banner.rescanningClients"));
		try {
			const next = await api.rescanWowInstalls();
			await applySettings(next);
			setStatus(
				next.hasWowInstall
					? t("banner.rescanClientsFound", { count: next.detectedInstalls.length })
					: t("banner.rescanClientsNone"),
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
					<div className="brand-title">{t("app.brand")}</div>
				</div>

				<nav className="tab-row">
					<button
						className={`tab-btn ${tab === "browse" ? "active" : ""}`}
						onClick={() => setTab("browse")}
					>
						{t("nav.browse")}
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
						{t("nav.rescan")}
					</button>
					<button
						className={`btn settings-btn ${tab === "settings" ? "active-settings" : ""}`}
						onClick={openSettings}
						title={t("nav.settings")}
					>
						{t("nav.settings")}
						{settings?.hasApiKey ? <span className="settings-dot" /> : null}
					</button>
				</div>
			</header>

			{status ? <div className="banner">{status}</div> : null}

			{showApiWarning ? (
				<div className="banner">
					{t("banner.noApiKey")}
					<button className="linkish" onClick={openSettings}>
						{t("banner.goSettings")}
					</button>
					{t("banner.fillKey")}
					{apiHelp ? (
						<>
							<a href={apiHelp.apply} target="_blank" rel="noreferrer">
								{t("banner.applyForm")}
							</a>
							{" · "}
							<a href={apiHelp.docs} target="_blank" rel="noreferrer">
								{t("banner.docs")}
							</a>
						</>
					) : null}
				</div>
			) : null}

			{settings?.hasApiKey ? (
				<div className="banner ok-banner">{t("banner.apiEnabled")}</div>
			) : null}

			{showWowWarning ? (
				<div className="banner">
					{t("banner.noWow")}
					<button className="linkish" onClick={openSettings}>
						{t("banner.goSettings")}
					</button>
					{t("banner.addWowHint")}
				</div>
			) : null}

			{showInvalidWowWarning ? (
				<div className="banner">
					{t("banner.invalidWow")}
					{settings?.invalidWowRoots.join("；")}。{t("banner.fixOrRemove")}
					<button className="linkish" onClick={openSettings}>
						{t("banner.goSettings")}
					</button>
					。
				</div>
			) : null}

			<main className="content">
				{tab === "settings" ? (
					<SettingsPage
						onBack={() => setTab(previousTab === "settings" ? "browse" : previousTab)}
						onSaved={(next) => {
							applySettings(next);
							setBrowseNonce((v) => v + 1);
						}}
					/>
				) : null}

				<div style={{ display: tab === "browse" ? undefined : "none" }}>
					<BrowseTab
						key={browseNonce}
						installs={installs}
						onInstalled={() => {
							setRefreshKey((v) => v + 1);
							setStatus(t("banner.addonInstalled"));
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
								<div className="empty">{t("installed.clientMissing")}</div>
							);
						})()
					: null}
			</main>
		</div>
	);
}
