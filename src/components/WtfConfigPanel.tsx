import { useEffect, useRef, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import { useToast } from "../hooks/useToast";
import { useI18n, type MessageKey } from "../i18n";
import { translateInstallLabel } from "../i18n/catalogLabels";
import type {
	SyncReport,
	WtfAccount,
	WtfCharacterRef,
	WtfProfileTree,
	WowInstall,
} from "../types";

type Props = {
	install: WowInstall;
	onNeedBackupPath: () => void;
};

type SyncDialogState = {
	kind: "backup" | "restore";
	running: boolean;
	liveLog: string;
	report: SyncReport | null;
	error: string | null;
};

type CopyTarget =
	| { kind: "account"; account: string }
	| { kind: "character"; account: string; realm: string; character: string };

type CopyCategoryId =
	| "savedVariables"
	| "addOns"
	| "layout"
	| "configCache"
	| "chatCache"
	| "bindings"
	| "macros"
	| "editMode";

type CopyCategory = {
	id: CopyCategoryId;
	labelKey: MessageKey;
	files: string[];
	defaultChecked: boolean;
};

const CHARACTER_COPY_CATEGORIES: CopyCategory[] = [
	{
		id: "savedVariables",
		labelKey: "wtf.copyItemSavedVariables",
		files: ["SavedVariables"],
		defaultChecked: true,
	},
	{
		id: "addOns",
		labelKey: "wtf.copyItemAddOns",
		files: ["AddOns.txt"],
		defaultChecked: false,
	},
	{
		id: "layout",
		labelKey: "wtf.copyItemLayout",
		files: ["layout-local.txt", "layout-cache.txt"],
		defaultChecked: false,
	},
	{
		id: "configCache",
		labelKey: "wtf.copyItemConfigCache",
		files: ["config-cache.wtf"],
		defaultChecked: false,
	},
	{
		id: "chatCache",
		labelKey: "wtf.copyItemChatCache",
		files: ["chat-cache.txt"],
		defaultChecked: false,
	},
	{
		id: "bindings",
		labelKey: "wtf.copyItemBindings",
		files: ["bindings-cache.wtf"],
		defaultChecked: false,
	},
];

const ACCOUNT_COPY_CATEGORIES: CopyCategory[] = [
	{
		id: "savedVariables",
		labelKey: "wtf.copyItemSavedVariables",
		files: ["SavedVariables", "SavedVariables.lua"],
		defaultChecked: true,
	},
	{
		id: "configCache",
		labelKey: "wtf.copyItemConfigCache",
		files: ["config-cache.wtf"],
		defaultChecked: false,
	},
	{
		id: "macros",
		labelKey: "wtf.copyItemMacros",
		files: ["macros-cache.txt"],
		defaultChecked: false,
	},
	{
		id: "bindings",
		labelKey: "wtf.copyItemBindings",
		files: ["bindings-cache.wtf"],
		defaultChecked: false,
	},
	{
		id: "editMode",
		labelKey: "wtf.copyItemEditMode",
		files: ["edit-mode-cache-account.txt"],
		defaultChecked: false,
	},
];

function defaultCopySelection(
	kind: CopyTarget["kind"],
): Partial<Record<CopyCategoryId, boolean>> {
	const cats =
		kind === "account" ? ACCOUNT_COPY_CATEGORIES : CHARACTER_COPY_CATEGORIES;
	const out: Partial<Record<CopyCategoryId, boolean>> = {};
	for (const cat of cats) {
		out[cat.id] = cat.defaultChecked;
	}
	return out;
}

function selectedCopyFiles(
	kind: CopyTarget["kind"],
	selection: Partial<Record<CopyCategoryId, boolean>>,
): string[] {
	const cats =
		kind === "account" ? ACCOUNT_COPY_CATEGORIES : CHARACTER_COPY_CATEGORIES;
	const files: string[] = [];
	for (const cat of cats) {
		if (selection[cat.id]) {
			files.push(...cat.files);
		}
	}
	return files;
}

function charKey(account: string, realm: string, name: string): string {
	return `${account}\0${realm}\0${name}`;
}

function charLabel(account: string, realm: string, name: string): string {
	return `${account} / ${realm} / ${name}`;
}

export function WtfConfigPanel({ install, onNeedBackupPath }: Props) {
	const { t, locale } = useI18n();
	const { showToast, toastNode } = useToast();
	const hasBackupPath = Boolean(install.backupPath?.trim());
	const installLabel = translateInstallLabel(install.flavorKey, install.label, locale);
	const [tree, setTree] = useState<WtfProfileTree | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [busyKey, setBusyKey] = useState<string | null>(null);
	const [expanded, setExpanded] = useState<Record<string, boolean>>({});
	const [copySource, setCopySource] = useState<CopyTarget | null>(null);
	const [copyDestKey, setCopyDestKey] = useState("");
	const [copySelection, setCopySelection] = useState<
		Partial<Record<CopyCategoryId, boolean>>
	>(() => defaultCopySelection("character"));
	const [syncDialog, setSyncDialog] = useState<SyncDialogState | null>(null);
	const syncOutputRef = useRef<HTMLPreElement | null>(null);
	const syncLiveLogRef = useRef("");
	const syncFlushTimerRef = useRef<number | null>(null);
	const installIdRef = useRef(install.id);
	installIdRef.current = install.id;

	async function load(forInstallId = install.id) {
		setLoading(true);
		setError(null);
		try {
			const next = await api.listWtfProfiles(forInstallId);
			if (installIdRef.current !== forInstallId) return;
			setTree(next);
			setExpanded((prev) => {
				const out: Record<string, boolean> = {};
				for (const account of next.accounts) {
					out[account.name] = prev[account.name] ?? true;
				}
				return out;
			});
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
			setTree(null);
		} finally {
			if (installIdRef.current === forInstallId) {
				setLoading(false);
			}
		}
	}

	useEffect(() => {
		setTree(null);
		setError(null);
		setCopySource(null);
		setCopyDestKey("");
		setCopySelection(defaultCopySelection("character"));
		setBusyKey(null);
		setSyncDialog(null);
		void load(install.id);
	}, [install.id]);

	useEffect(() => {
		if (!syncDialog || !syncOutputRef.current) return;
		syncOutputRef.current.scrollTop = syncOutputRef.current.scrollHeight;
	}, [syncDialog?.liveLog, syncDialog?.report?.log, syncDialog?.error, syncDialog?.running]);

	useEffect(() => {
		return () => {
			if (syncFlushTimerRef.current !== null) {
				window.clearTimeout(syncFlushTimerRef.current);
				syncFlushTimerRef.current = null;
			}
		};
	}, []);

	function closeCopyDialog() {
		if (busyKey === "copy") return;
		setCopySource(null);
		setCopyDestKey("");
	}

	function beginSyncDialog(kind: "backup" | "restore") {
		if (syncFlushTimerRef.current !== null) {
			window.clearTimeout(syncFlushTimerRef.current);
			syncFlushTimerRef.current = null;
		}
		syncLiveLogRef.current = "";
		setSyncDialog({
			kind,
			running: true,
			liveLog: "",
			report: null,
			error: null,
		});
	}

	function appendSyncLine(line: string) {
		syncLiveLogRef.current += `${line}\n`;
		if (syncFlushTimerRef.current !== null) return;
		syncFlushTimerRef.current = window.setTimeout(() => {
			syncFlushTimerRef.current = null;
			const liveLog = syncLiveLogRef.current;
			setSyncDialog((prev) => (prev ? { ...prev, liveLog } : prev));
		}, 50);
	}

	function finishSyncDialog(
		kind: "backup" | "restore",
		patch: Partial<SyncDialogState>,
	) {
		if (syncFlushTimerRef.current !== null) {
			window.clearTimeout(syncFlushTimerRef.current);
			syncFlushTimerRef.current = null;
		}
		setSyncDialog({
			kind,
			running: false,
			liveLog: syncLiveLogRef.current,
			report: null,
			error: null,
			...patch,
		});
	}

	async function handleBackup() {
		const forInstallId = install.id;
		setError(null);
		if (!hasBackupPath) {
			showToast(t("installed.backupNeedPath"));
			onNeedBackupPath();
			return;
		}
		setBusyKey("__backup__");
		beginSyncDialog("backup");
		try {
			const report = await api.backupWtf(forInstallId, appendSyncLine);
			if (installIdRef.current !== forInstallId) return;
			finishSyncDialog("backup", { report, error: null });
			showToast(t("wtf.backupDone", { path: report.path }));
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			const message = String(err);
			if (message.includes("未设置备份路径")) {
				setSyncDialog(null);
				showToast(t("installed.backupNeedPath"));
				onNeedBackupPath();
				return;
			}
			finishSyncDialog("backup", { report: null, error: message });
			setError(message);
		} finally {
			if (installIdRef.current === forInstallId) {
				setBusyKey(null);
			}
		}
	}

	async function handleRestore() {
		const forInstallId = install.id;
		setError(null);
		if (!hasBackupPath) {
			showToast(t("installed.backupNeedPath"));
			onNeedBackupPath();
			return;
		}
		const confirmed = await ask(
			t("wtf.restoreConfirm", { label: installLabel }),
			{
				title: t("wtf.restoreTitle"),
				kind: "warning",
				okLabel: t("wtf.restoreOk"),
				cancelLabel: t("wtf.restoreCancel"),
			},
		);
		if (!confirmed) return;
		if (installIdRef.current !== forInstallId) return;
		setBusyKey("__restore__");
		beginSyncDialog("restore");
		let report: SyncReport | null = null;
		try {
			report = await api.restoreWtf(forInstallId, appendSyncLine);
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			const message = String(err);
			if (message.includes("未设置备份路径")) {
				setSyncDialog(null);
				showToast(t("installed.backupNeedPath"));
				onNeedBackupPath();
				return;
			}
			if (
				message.includes("未找到该客户端的 WTF 备份") ||
				message.includes("未找到该客户端的备份目录")
			) {
				finishSyncDialog("restore", {
					report: null,
					error: t("wtf.restoreMissing"),
				});
				setError(t("wtf.restoreMissing"));
				return;
			}
			finishSyncDialog("restore", { report: null, error: message });
			setError(message);
			return;
		} finally {
			if (installIdRef.current === forInstallId && report === null) {
				setBusyKey(null);
			}
		}
		if (installIdRef.current !== forInstallId) return;
		finishSyncDialog("restore", { report, error: null });
		showToast(t("wtf.restoreDone", { path: report.path }));
		try {
			await load(forInstallId);
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
		} finally {
			if (installIdRef.current === forInstallId) {
				setBusyKey(null);
			}
		}
	}

	function resolveWtfPath(current = tree): string {
		return (
			current?.wtfPath ||
			`${install.installPath.replace(/[\\/]+$/, "")}/WTF`
		);
	}

	async function handleOpenWtf() {
		setError(null);
		try {
			await api.openDir(resolveWtfPath());
		} catch (err) {
			setError(String(err));
		}
	}

	async function handleDeleteAccount(account: WtfAccount) {
		const forInstallId = install.id;
		const confirmed = await ask(
			t("wtf.deleteAccountConfirm", { name: account.name }),
			{
				title: t("wtf.deleteAccountTitle"),
				kind: "warning",
				okLabel: t("wtf.deleteOk"),
				cancelLabel: t("wtf.deleteCancel"),
			},
		);
		if (!confirmed || installIdRef.current !== forInstallId) return;
		const key = `acc:${account.name}`;
		setBusyKey(key);
		setError(null);
		try {
			await api.deleteWtfAccount(forInstallId, account.name);
			if (installIdRef.current !== forInstallId) return;
			showToast(t("wtf.deleteAccountDone", { name: account.name }));
			await load(forInstallId);
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
		} finally {
			if (installIdRef.current === forInstallId) {
				setBusyKey(null);
			}
		}
	}

	async function handleDeleteCharacter(
		account: string,
		realm: string,
		name: string,
	) {
		const forInstallId = install.id;
		const label = charLabel(account, realm, name);
		const confirmed = await ask(
			t("wtf.deleteCharacterConfirm", { name: label }),
			{
				title: t("wtf.deleteCharacterTitle"),
				kind: "warning",
				okLabel: t("wtf.deleteOk"),
				cancelLabel: t("wtf.deleteCancel"),
			},
		);
		if (!confirmed || installIdRef.current !== forInstallId) return;
		const key = `char:${charKey(account, realm, name)}`;
		setBusyKey(key);
		setError(null);
		try {
			await api.deleteWtfCharacter(forInstallId, account, realm, name);
			if (installIdRef.current !== forInstallId) return;
			showToast(t("wtf.deleteCharacterDone", { name: label }));
			await load(forInstallId);
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
		} finally {
			if (installIdRef.current === forInstallId) {
				setBusyKey(null);
			}
		}
	}

	function openCopyAccount(account: string) {
		setCopySource({ kind: "account", account });
		setCopyDestKey("");
		setCopySelection(defaultCopySelection("account"));
	}

	function openCopyCharacter(account: string, realm: string, character: string) {
		setCopySource({ kind: "character", account, realm, character });
		setCopyDestKey("");
		setCopySelection(defaultCopySelection("character"));
	}

	async function confirmCopy() {
		if (!copySource || !copyDestKey || !tree) return;
		const items = selectedCopyFiles(copySource.kind, copySelection);
		if (items.length === 0) return;
		const forInstallId = install.id;
		setError(null);
		setBusyKey("copy");
		try {
			if (copySource.kind === "account") {
				const toAccount = copyDestKey;
				const confirmed = await ask(
					t("wtf.copyAccountConfirm", {
						from: copySource.account,
						to: toAccount,
					}),
					{
						title: t("wtf.copyTitle"),
						kind: "info",
						okLabel: t("wtf.copyOk"),
						cancelLabel: t("wtf.copyCancel"),
					},
				);
				if (!confirmed || installIdRef.current !== forInstallId) return;
				await api.copyWtfAccount(
					forInstallId,
					{ account: copySource.account },
					{ account: toAccount },
					items,
				);
				if (installIdRef.current !== forInstallId) return;
				showToast(
					t("wtf.copyAccountDone", {
						from: copySource.account,
						to: toAccount,
					}),
				);
			} else {
				const [toAccount, toRealm, toCharacter] = copyDestKey.split("\0");
				if (!toAccount || !toRealm || !toCharacter) return;
				const fromLabel = charLabel(
					copySource.account,
					copySource.realm,
					copySource.character,
				);
				const toLabel = charLabel(toAccount, toRealm, toCharacter);
				const confirmed = await ask(
					t("wtf.copyCharacterConfirm", {
						from: fromLabel,
						to: toLabel,
					}),
					{
						title: t("wtf.copyTitle"),
						kind: "info",
						okLabel: t("wtf.copyOk"),
						cancelLabel: t("wtf.copyCancel"),
					},
				);
				if (!confirmed || installIdRef.current !== forInstallId) return;
				const from: WtfCharacterRef = {
					account: copySource.account,
					realm: copySource.realm,
					character: copySource.character,
				};
				await api.copyWtfCharacter(
					forInstallId,
					from,
					{
						account: toAccount,
						realm: toRealm,
						character: toCharacter,
					},
					items,
				);
				if (installIdRef.current !== forInstallId) return;
				showToast(
					t("wtf.copyCharacterDone", { from: fromLabel, to: toLabel }),
				);
			}
			setCopySource(null);
			setCopyDestKey("");
			await load(forInstallId);
		} catch (err) {
			if (installIdRef.current !== forInstallId) return;
			setError(String(err));
		} finally {
			if (installIdRef.current === forInstallId) {
				setBusyKey(null);
			}
		}
	}

	const accounts = tree?.accounts ?? [];
	const wtfPath = resolveWtfPath();
	const syncBusy = busyKey === "__backup__" || busyKey === "__restore__";
	const toolbarBlocked = loading || busyKey !== null;
	const copyCategories =
		copySource?.kind === "account"
			? ACCOUNT_COPY_CATEGORIES
			: CHARACTER_COPY_CATEGORIES;
	const copyItemsSelected = copySource
		? selectedCopyFiles(copySource.kind, copySelection).length > 0
		: false;
	const otherAccounts =
		copySource?.kind === "account"
			? accounts.filter(
					(a) =>
						a.name.toLowerCase() !== copySource.account.toLowerCase(),
				)
			: [];
	const otherCharacters =
		copySource?.kind === "character"
			? accounts.flatMap((a) =>
					a.characters
						.filter(
							(c) =>
								charKey(a.name, c.realm, c.name) !==
								charKey(
									copySource.account,
									copySource.realm,
									copySource.character,
								),
						)
						.map((c) => ({
							key: charKey(a.name, c.realm, c.name),
							label: charLabel(a.name, c.realm, c.name),
						})),
				)
			: [];

	return (
		<div className="wtf-panel">
			{syncDialog ? (
				<div className="modal-backdrop" role="presentation">
					<div
						className="modal-card sync-modal"
						role="dialog"
						aria-modal="true"
						aria-labelledby="wtf-sync-modal-title"
					>
						<div className="sync-modal-header">
							<h3 id="wtf-sync-modal-title">
								{syncDialog.kind === "backup"
									? t("installed.syncBackupTitle")
									: t("installed.syncRestoreTitle")}
							</h3>
							{syncDialog.running ? (
								<span className="muted">{t("installed.syncRunning")}</span>
							) : syncDialog.error ? (
								<span className="sync-modal-failed">{t("installed.syncFailed")}</span>
							) : syncDialog.report ? (
								<span className="sync-modal-ok">{t("installed.syncDone")}</span>
							) : null}
						</div>
						{syncDialog.report ? (
							<div className="muted sync-modal-meta">
								<div>
									{t("installed.syncItems", {
										items: syncDialog.report.items.join(" / ") || "—",
									})}
								</div>
								<div className="sync-modal-path">{syncDialog.report.path}</div>
							</div>
						) : null}
						<label className="settings-field">
							<span>{t("installed.syncOutput")}</span>
							<pre className="sync-output" ref={syncOutputRef}>
								{syncDialog.liveLog.trim()
									? syncDialog.liveLog
									: syncDialog.running
										? t("installed.syncRunning")
										: syncDialog.error ||
											syncDialog.report?.log ||
											"—"}
							</pre>
						</label>
						<div className="card-actions">
							<button
								className="btn primary"
								type="button"
								disabled={syncDialog.running}
								onClick={() => setSyncDialog(null)}
							>
								{t("installed.syncClose")}
							</button>
						</div>
					</div>
				</div>
			) : null}

			<div className="wtf-toolbar">
				<div>
					<h2>{t("wtf.title", { label: installLabel })}</h2>
					<div className="muted">
						{wtfPath}
						{install.gameVersion ? ` · ${install.gameVersion}` : ""}
					</div>
					<div className="muted">{t("wtf.hint")}</div>
				</div>
				<div className="toolbar-actions">
					<button
						className="btn"
						type="button"
						onClick={() => void load()}
						disabled={toolbarBlocked}
					>
						{loading && !syncBusy ? t("wtf.refreshing") : t("wtf.refresh")}
					</button>
					<button
						className="btn"
						type="button"
						onClick={() => void handleOpenWtf()}
						disabled={toolbarBlocked}
					>
						{t("wtf.openFolder")}
					</button>
					<button
						className="btn"
						type="button"
						onClick={() => void handleBackup()}
						disabled={toolbarBlocked}
						title={t("wtf.backup")}
					>
						{busyKey === "__backup__" ? t("wtf.backingUp") : t("wtf.backup")}
					</button>
					<button
						className="btn"
						type="button"
						onClick={() => void handleRestore()}
						disabled={toolbarBlocked}
						title={t("wtf.restore")}
					>
						{busyKey === "__restore__" ? t("wtf.restoring") : t("wtf.restore")}
					</button>
				</div>
			</div>

			{error ? <div className="banner">{error}</div> : null}

			{loading && !tree ? (
				<div className="list-loading" role="status" aria-live="polite">
					<div className="list-loading-spinner" aria-hidden="true" />
					<span>{t("wtf.loading")}</span>
				</div>
			) : accounts.length === 0 ? (
				<div className="empty">
					<strong>{t("wtf.emptyTitle")}</strong>
					{t("wtf.emptyHint")}
				</div>
			) : (
				<div className="wtf-list">
					{accounts.map((account) => {
						const isOpen = expanded[account.name] !== false;
						const accBusy = busyKey === `acc:${account.name}`;
						return (
							<div className="wtf-account" key={account.name}>
								<div className="wtf-row wtf-account-row">
									<button
										type="button"
										className="wtf-expand"
										aria-expanded={isOpen}
										onClick={() =>
											setExpanded((prev) => ({
												...prev,
												[account.name]: !isOpen,
											}))
										}
									>
										<span aria-hidden="true">{isOpen ? "▾" : "▸"}</span>
										<strong>{account.name}</strong>
										<span className="muted">
											{t("wtf.accountMeta", {
												count: String(account.characters.length),
											})}
										</span>
									</button>
									<div className="wtf-actions">
										<button
											className="btn"
											type="button"
											disabled={busyKey !== null || accounts.length < 2}
											onClick={() => openCopyAccount(account.name)}
										>
											{t("wtf.copy")}
										</button>
										<button
											className="btn danger"
											type="button"
											disabled={busyKey !== null}
											onClick={() => void handleDeleteAccount(account)}
										>
											{accBusy ? t("wtf.deleting") : t("wtf.delete")}
										</button>
									</div>
								</div>
								{isOpen ? (
									account.characters.length === 0 ? (
										<div className="wtf-empty-chars muted">
											{t("wtf.noCharacters")}
										</div>
									) : (
										<ul className="wtf-char-list">
											{account.characters.map((ch) => {
												const key = charKey(
													account.name,
													ch.realm,
													ch.name,
												);
												const charBusy = busyKey === `char:${key}`;
												return (
													<li className="wtf-row wtf-char-row" key={key}>
														<div className="wtf-char-label">
															<span className="wtf-realm">{ch.realm}</span>
															<span className="wtf-char-name">{ch.name}</span>
														</div>
														<div className="wtf-actions">
															<button
																className="btn"
																type="button"
																disabled={
																	busyKey !== null ||
																	accounts.every(
																		(a) => a.characters.length === 0,
																	) ||
																	accounts.reduce(
																		(n, a) => n + a.characters.length,
																		0,
																	) < 2
																}
																onClick={() =>
																	openCopyCharacter(
																		account.name,
																		ch.realm,
																		ch.name,
																	)
																}
															>
																{t("wtf.copy")}
															</button>
															<button
																className="btn danger"
																type="button"
																disabled={busyKey !== null}
																onClick={() =>
																	void handleDeleteCharacter(
																		account.name,
																		ch.realm,
																		ch.name,
																	)
																}
															>
																{charBusy
																	? t("wtf.deleting")
																	: t("wtf.delete")}
															</button>
														</div>
													</li>
												);
											})}
										</ul>
									)
								) : null}
							</div>
						);
					})}
				</div>
			)}

			{copySource ? (
				<div
					className="modal-backdrop"
					role="presentation"
					onClick={(event) => {
						if (event.target === event.currentTarget) {
							closeCopyDialog();
						}
					}}
				>
					<div
						className="modal-card wtf-copy-modal"
						role="dialog"
						aria-modal="true"
						aria-labelledby="wtf-copy-title"
					>
						<h3 id="wtf-copy-title">{t("wtf.copyTitle")}</h3>
						<p className="muted">
							{copySource.kind === "account"
								? t("wtf.copyAccountFrom", { name: copySource.account })
								: t("wtf.copyCharacterFrom", {
										name: charLabel(
											copySource.account,
											copySource.realm,
											copySource.character,
										),
									})}
						</p>
						<fieldset
							className="wtf-copy-items"
							disabled={busyKey === "copy"}
						>
							<legend>{t("wtf.copyItems")}</legend>
							{copyCategories.map((cat) => (
								<label className="wtf-copy-item" key={cat.id}>
									<input
										type="checkbox"
										checked={!!copySelection[cat.id]}
										onChange={(e) =>
											setCopySelection((prev) => ({
												...prev,
												[cat.id]: e.target.checked,
											}))
										}
									/>
									<span>{t(cat.labelKey)}</span>
								</label>
							))}
						</fieldset>
						<label className="settings-field">
							<span>{t("wtf.copyDest")}</span>
							{copySource.kind === "account" ? (
								<select
									value={copyDestKey}
									onChange={(e) => setCopyDestKey(e.target.value)}
									disabled={busyKey === "copy"}
								>
									<option value="">{t("wtf.copyDestPlaceholder")}</option>
									{otherAccounts.map((a) => (
										<option key={a.name} value={a.name}>
											{a.name}
										</option>
									))}
								</select>
							) : (
								<select
									value={copyDestKey}
									onChange={(e) => setCopyDestKey(e.target.value)}
									disabled={busyKey === "copy"}
								>
									<option value="">{t("wtf.copyDestPlaceholder")}</option>
									{otherCharacters.map((c) => (
										<option key={c.key} value={c.key}>
											{c.label}
										</option>
									))}
								</select>
							)}
						</label>
						<div className="card-actions">
							<button
								className="btn primary"
								type="button"
								disabled={
									!copyDestKey || !copyItemsSelected || busyKey === "copy"
								}
								onClick={() => void confirmCopy()}
							>
								{busyKey === "copy" ? t("wtf.copying") : t("wtf.copyOk")}
							</button>
							<button
								className="btn"
								type="button"
								disabled={busyKey === "copy"}
								onClick={closeCopyDialog}
							>
								{t("wtf.copyCancel")}
							</button>
						</div>
					</div>
				</div>
			) : null}
			{toastNode}
		</div>
	);
}
