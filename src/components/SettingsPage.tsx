import { useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import { useI18n, type LocalePreference } from "../i18n";
import type { LogEntry, SettingsView } from "../types";

type SettingsFocus = "backup" | null;

type Props = {
	onBack: () => void;
	onSaved: (settings: SettingsView) => void;
	focusSection?: SettingsFocus;
	onFocusHandled?: () => void;
};

const LOG_LEVELS = ["error", "warn", "info", "debug", "trace"] as const;

export function SettingsPage({
	onBack,
	onSaved,
	focusSection = null,
	onFocusHandled,
}: Props) {
	const { t, preference, setPreference } = useI18n();
	const [settings, setSettings] = useState<SettingsView | null>(null);
	const [apiKey, setApiKey] = useState("");
	const [localeDraft, setLocaleDraft] = useState<LocalePreference>("system");
	const [saving, setSaving] = useState(false);
	const [busy, setBusy] = useState(false);
	const [ready, setReady] = useState(false);
	const [message, setMessage] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [logLevel, setLogLevel] = useState("warn");
	const [logEntries, setLogEntries] = useState<LogEntry[]>([]);
	const [logsOpen, setLogsOpen] = useState(false);
	const [logsBusy, setLogsBusy] = useState(false);
	const [highlightBackup, setHighlightBackup] = useState(false);
	const logPaneRef = useRef<HTMLPreElement | null>(null);
	const backupCardRef = useRef<HTMLDivElement | null>(null);
	const saveChain = useRef(Promise.resolve());

	async function reload() {
		const data = await api.getSettings();
		setSettings(data);
		setApiKey(data.curseforgeApiKey);
		const pref = (data.locale || "system") as LocalePreference;
		setLocaleDraft(pref === "zh-CN" || pref === "en" ? pref : "system");
		setReady(true);
		onSaved(data);
		return data;
	}

	useEffect(() => {
		reload().catch((err) => setError(String(err)));
	}, []);

	useEffect(() => {
		setLocaleDraft(preference);
	}, [preference]);

	useEffect(() => {
		if (focusSection !== "backup" || !ready) return;
		setHighlightBackup(true);
		setMessage(t("settings.backupPathUnset"));
		const card = backupCardRef.current;
		if (card) {
			card.scrollIntoView({ behavior: "smooth", block: "center" });
		}
		onFocusHandled?.();
		const timer = window.setTimeout(() => setHighlightBackup(false), 2600);
		return () => window.clearTimeout(timer);
	}, [focusSection, ready, t, onFocusHandled]);

	function enqueueSave<T>(work: () => Promise<T>): Promise<T> {
		const run = saveChain.current.then(work, work);
		saveChain.current = run.then(
			() => undefined,
			() => undefined,
		);
		return run;
	}

	async function handleSaveApiKey(e: React.FormEvent) {
		e.preventDefault();
		setSaving(true);
		setError(null);
		setMessage(null);
		try {
			const next = await enqueueSave(() =>
				api.saveSettings({
					curseforgeApiKey: apiKey,
					wowRoots: settings?.wowRoots,
					locale: localeDraft,
				}),
			);
			setSettings(next);
			setApiKey(next.curseforgeApiKey);
			setMessage(next.hasApiKey ? t("settings.apiSaved") : t("settings.apiCleared"));
			onSaved(next);
		} catch (err) {
			setError(String(err));
		} finally {
			setSaving(false);
		}
	}

	async function handleSaveLocale(nextLocale: LocalePreference) {
		if (!ready || !settings) return;
		setLocaleDraft(nextLocale);
		setBusy(true);
		setError(null);
		setMessage(null);
		try {
			// Only patch locale — never send an empty API key draft.
			const next = await enqueueSave(() =>
				api.saveSettings({
					locale: nextLocale,
				}),
			);
			setSettings(next);
			setPreference(nextLocale);
			setMessage(t("settings.languageSaved"));
			onSaved(next);
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(false);
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
				title: t("settings.pickWowDir"),
			});
			if (!selected || Array.isArray(selected)) return;
			const next = await api.addWowPath(selected);
			setSettings(next);
			setMessage(t("settings.pathAdded", { path: selected }));
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
			setMessage(t("settings.pathRemoved", { path }));
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
					? t("settings.rescanDoneFound", { count: next.detectedInstalls.length })
					: t("settings.rescanDoneNone"),
			);
			onSaved(next);
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(false);
		}
	}

	async function handleChooseBackupPath() {
		setBusy(true);
		setError(null);
		setMessage(null);
		try {
			const selected = await open({
				directory: true,
				multiple: false,
				title: t("settings.pickBackupDir"),
				defaultPath: settings?.backupPath || undefined,
			});
			if (!selected || Array.isArray(selected)) return;
			const next = await enqueueSave(() =>
				api.saveSettings({
					backupPath: selected,
				}),
			);
			setSettings(next);
			setMessage(t("settings.backupPathSaved", { path: selected }));
			setHighlightBackup(false);
			onSaved(next);
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(false);
		}
	}

	async function handleClearBackupPath() {
		setBusy(true);
		setError(null);
		setMessage(null);
		try {
			const next = await enqueueSave(() =>
				api.saveSettings({
					backupPath: "",
				}),
			);
			setSettings(next);
			setMessage(t("settings.backupPathCleared"));
			onSaved(next);
		} catch (err) {
			setError(String(err));
		} finally {
			setBusy(false);
		}
	}

	async function handleOpenBackupPath() {
		if (!settings?.backupPath) return;
		setError(null);
		try {
			await api.openDir(settings.backupPath);
		} catch (err) {
			setError(String(err));
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
			setMessage(t("settings.logLevelSet", { level: applied.toUpperCase() }));
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
			setMessage(t("settings.logsCleared"));
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
				title: t("settings.saveLogTitle"),
				defaultPath: `wow-addon-manager-${stamp}.log`,
				filters: [{ name: "Log", extensions: ["log", "txt"] }],
			});
			if (!selected) return;
			await api.saveLogs(selected);
			setMessage(t("settings.logsSaved", { path: selected }));
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
					{t("settings.back")}
				</button>
				<div>
					<h2>{t("settings.title")}</h2>
					<p className="muted">{t("settings.subtitle")}</p>
				</div>
			</div>

			{error ? <div className="banner">{error}</div> : null}
			{message ? <div className="banner ok-banner">{message}</div> : null}

			<div className="settings-card">
				<h3 className="settings-card-title">{t("settings.language")}</h3>
				<p className="muted settings-hint">{t("settings.languageHint")}</p>
				<label className="settings-field">
					<span>{t("settings.language")}</span>
					<select
						value={localeDraft}
						disabled={!ready || busy || saving}
						onChange={(e) => handleSaveLocale(e.target.value as LocalePreference)}
					>
						<option value="system">{t("settings.languageSystem")}</option>
						<option value="zh-CN">{t("settings.languageZh")}</option>
						<option value="en">{t("settings.languageEn")}</option>
					</select>
				</label>
			</div>

			<form className="settings-card" onSubmit={handleSaveApiKey}>
				<h3 className="settings-card-title">{t("settings.apiTitle")}</h3>
				<label className="settings-field">
					<span>{t("settings.apiKey")}</span>
					<input
						type="password"
						autoComplete="off"
						spellCheck={false}
						placeholder={t("settings.apiPlaceholder")}
						value={apiKey}
						onChange={(e) => setApiKey(e.target.value)}
					/>
				</label>

				<p className="muted settings-hint">
					{t("settings.apiHint")}
					{settings?.hasApiKey ? t("settings.apiEnabled") : t("settings.apiDisabled")}
				</p>

				<div className="settings-links muted">
					{settings ? (
						<>
							<a href={settings.applyUrl} target="_blank" rel="noreferrer">
								{t("settings.applyKey")}
							</a>
							<span>·</span>
							<a href={settings.docsUrl} target="_blank" rel="noreferrer">
								{t("settings.apiDocs")}
							</a>
						</>
					) : null}
				</div>

				<div className="card-actions">
					<button className="btn primary" type="submit" disabled={saving}>
						{saving ? t("settings.saving") : t("settings.saveApiKey")}
					</button>
					<button
						className="btn"
						type="button"
						disabled={saving || !apiKey}
						onClick={() => setApiKey("")}
					>
						{t("settings.clearInput")}
					</button>
				</div>
			</form>

			<div className="settings-card">
				<h3 className="settings-card-title">{t("settings.wowTitle")}</h3>
				<p className="muted settings-hint">{t("settings.wowHint")}</p>

				<div className="card-actions">
					<button className="btn primary" type="button" disabled={busy} onClick={handleAddWowPath}>
						{t("settings.addWowPath")}
					</button>
					<button className="btn" type="button" disabled={busy} onClick={handleRescan}>
						{t("settings.rescan")}
					</button>
				</div>

				{settings?.detectedInstalls?.length ? (
					<div className="settings-list">
						<div className="settings-list-label">{t("settings.detectedClients")}</div>
						{settings.detectedInstalls.map((install) => (
							<div key={install.id} className="settings-list-item">
								<div>
									<strong>{install.label}</strong>
									<div className="muted">{install.installPath}</div>
								</div>
								<span className="muted">
									{install.gameVersion || t("settings.versionUnknown")}
								</span>
							</div>
						))}
					</div>
				) : (
					<div className="banner">{t("settings.noClients")}</div>
				)}

				{settings?.wowRoots?.length ? (
					<div className="settings-list">
						<div className="settings-list-label">{t("settings.manualPaths")}</div>
						{settings.wowRoots.map((root) => (
							<div key={root} className="settings-list-item">
								<div className="muted">{root}</div>
								<button
									className="btn danger"
									type="button"
									disabled={busy}
									onClick={() => handleRemoveWowPath(root)}
								>
									{t("settings.remove")}
								</button>
							</div>
						))}
					</div>
				) : null}

				{settings?.invalidWowRoots?.length ? (
					<div className="banner">
						{t("settings.invalidPaths")}
						{settings.invalidWowRoots.join("；")}
					</div>
				) : null}
			</div>

			<div
				className={`settings-card${highlightBackup ? " settings-card-focus" : ""}`}
				ref={backupCardRef}
				id="settings-backup"
			>
				<h3 className="settings-card-title">{t("settings.backupTitle")}</h3>
				<p className="muted settings-hint">{t("settings.backupHint")}</p>
				<label className="settings-field">
					<span>{t("settings.backupPath")}</span>
					<input
						type="text"
						readOnly
						spellCheck={false}
						placeholder={t("settings.backupPathPlaceholder")}
						value={settings?.backupPath || ""}
					/>
				</label>
				<div className="card-actions">
					<button
						className="btn primary"
						type="button"
						disabled={busy || !ready}
						onClick={handleChooseBackupPath}
					>
						{t("settings.chooseBackupPath")}
					</button>
					<button
						className="btn"
						type="button"
						disabled={busy || !settings?.backupPath}
						onClick={handleOpenBackupPath}
					>
						{t("settings.openBackupPath")}
					</button>
					<button
						className="btn"
						type="button"
						disabled={busy || !settings?.backupPath}
						onClick={handleClearBackupPath}
					>
						{t("settings.clearBackupPath")}
					</button>
				</div>
			</div>

			<div className="settings-card">
				<h3 className="settings-card-title">{t("settings.logsTitle")}</h3>
				<p className="muted settings-hint">{t("settings.logsHint")}</p>
				<div className="card-actions">
					<button
						className="btn primary"
						type="button"
						disabled={logsBusy}
						onClick={logsOpen ? () => setLogsOpen(false) : handleOpenLogs}
					>
						{logsOpen ? t("settings.closeLogs") : t("settings.openLogs")}
					</button>
					<button
						className="btn"
						type="button"
						disabled={logsBusy || !logsOpen}
						onClick={handleSaveLogs}
					>
						{t("settings.saveLogs")}
					</button>
					<button
						className="btn"
						type="button"
						disabled={logsBusy || !logsOpen}
						onClick={handleClearLogs}
					>
						{t("settings.clearLogs")}
					</button>
				</div>

				{logsOpen ? (
					<>
						<label className="settings-field">
							<span>{t("settings.logLevel")}</span>
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
								? t("settings.noLogs")
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
