import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "../api";
import {
	isApiDistributionDisabledError,
	resolveCurseforgeAddonUrl,
} from "../curseforgeErrors";
import { useI18n } from "../i18n";
import { subscribeInstalledUpdates } from "../installedUpdates";
import type {
	AddonJobProgress,
	AddonSummary,
	FilterCatalog,
	InstalledAddon,
	SearchResult,
	WowInstall,
} from "../types";
import { AddonCard, type BrowseInstallState } from "./AddonCard";
import { AddonDetailModal } from "./AddonDetailModal";
import { ApiDistributionDisabledDialog } from "./ApiDistributionDisabledDialog";
import { FilterSidebar } from "./FilterSidebar";

type Props = {
	installs: WowInstall[];
	onInstalled: () => void;
};

function jobKey(addonId: number, installId: string): string {
	return `${addonId}:${installId}`;
}

function findInstalledMatch(
	addon: AddonSummary,
	list: InstalledAddon[],
): InstalledAddon | null {
	const byId = list.find((item) => item.curseforgeId === addon.id);
	if (byId) return byId;
	const slug = addon.slug.trim().toLowerCase();
	if (!slug) return null;
	return (
		list.find((item) => item.curseforgeSlug?.trim().toLowerCase() === slug) ??
		null
	);
}

function browseNeedsUpdate(
	addon: AddonSummary,
	installed: InstalledAddon,
): boolean {
	const remoteFile = addon.latestFileId;
	if (remoteFile == null) {
		return Boolean(installed.hasUpdate);
	}
	const localFile = installed.fileId;
	if (localFile == null) return true;
	return remoteFile > localFile;
}

export function BrowseTab({ installs, onInstalled }: Props) {
	const { t } = useI18n();
	const [catalog, setCatalog] = useState<FilterCatalog | null>(null);
	const [result, setResult] = useState<SearchResult | null>(null);
	const [search, setSearch] = useState("");
	const [draftSearch, setDraftSearch] = useState("");
	const [page, setPage] = useState(1);
	const [categories, setCategories] = useState<string[]>([]);
	const [flavors, setFlavors] = useState<number[]>([]);
	const [loading, setLoading] = useState(false);
	const [busyKeys, setBusyKeys] = useState<Record<string, true>>({});
	const [progressByKey, setProgressByKey] = useState<
		Record<string, AddonJobProgress>
	>({});
	const [installedByInstallId, setInstalledByInstallId] = useState<
		Record<string, InstalledAddon[]>
	>({});
	const [error, setError] = useState<string | null>(null);
	const [reloadToken, setReloadToken] = useState(0);
	const [detail, setDetail] = useState<{ modId: number; title: string } | null>(
		null,
	);
	const [apiDisabledDialog, setApiDisabledDialog] = useState<{
		addonName: string;
		curseforgeUrl: string;
	} | null>(null);
	const requestSeq = useRef(0);
	const installedLoadSeq = useRef(0);

	useEffect(() => {
		api.getFilterCatalog().then(setCatalog).catch(console.error);
	}, []);

	/** Commit the live search box so every re-query uses current input. */
	function applyLiveSearch() {
		setSearch(draftSearch.trim());
	}

	useEffect(() => {
		const seq = ++installedLoadSeq.current;
		let cancelled = false;
		(async () => {
			const entries = await Promise.all(
				installs.map(async (install) => {
					try {
						const list = await api.listInstalledAddons(install.id);
						return [install.id, list] as const;
					} catch {
						return [install.id, [] as InstalledAddon[]] as const;
					}
				}),
			);
			if (cancelled || seq !== installedLoadSeq.current) return;
			setInstalledByInstallId(Object.fromEntries(entries));
		})();
		return () => {
			cancelled = true;
		};
	}, [installs]);

	useEffect(() => {
		return subscribeInstalledUpdates((installId, list) => {
			setInstalledByInstallId((prev) => ({
				...prev,
				[installId]: list,
			}));
		});
	}, []);

	async function load() {
		const seq = ++requestSeq.current;
		setLoading(true);
		setError(null);
		// Always read the box — not the last committed keyword.
		const query = draftSearch.trim();
		try {
			const data = await api.searchAddons(
				{
					search: query,
					page,
					pageSize: 20,
					categories,
					gameVersionTypeIds: flavors,
					sortBy: "relevancy",
				},
				false,
			);
			if (seq === requestSeq.current) {
				setResult(data);
			}
		} catch (err) {
			if (seq === requestSeq.current) {
				setError(String(err));
			}
		} finally {
			if (seq === requestSeq.current) {
				setLoading(false);
			}
		}
	}

	useEffect(() => {
		load();
	}, [search, page, categories, flavors, reloadToken]);

	const totalPages = useMemo(() => {
		if (!result) return 1;
		return Math.max(1, Math.ceil(result.total / result.pageSize));
	}, [result]);

	function upsertInstalled(installId: string, installed: InstalledAddon) {
		setInstalledByInstallId((prev) => {
			const list = prev[installId] ?? [];
			const related = new Set(installed.relatedFolders ?? [installed.folderName]);
			const withoutSuite = list.filter(
				(item) =>
					item.folderName !== installed.folderName &&
					!related.has(item.folderName) &&
					!(
						installed.curseforgeId != null &&
						item.curseforgeId === installed.curseforgeId
					),
			);
			return {
				...prev,
				[installId]: [...withoutSuite, installed],
			};
		});
	}

	async function handleInstall(addon: AddonSummary, installId: string) {
		const key = jobKey(addon.id, installId);
		setBusyKeys((prev) => ({ ...prev, [key]: true }));
		setProgressByKey((prev) => ({
			...prev,
			[key]: { stage: "download", percent: 0 },
		}));
		setError(null);
		try {
			const installed = await api.installAddon(installId, addon, (progress) => {
				setProgressByKey((prev) => ({ ...prev, [key]: progress }));
			});
			upsertInstalled(installId, installed);
			onInstalled();
		} catch (err) {
			if (isApiDistributionDisabledError(err)) {
				const curseforgeUrl = resolveCurseforgeAddonUrl({
					websiteUrl: addon.websiteUrl,
					slug: addon.slug,
					id: addon.id,
				});
				if (curseforgeUrl) {
					setApiDisabledDialog({
						addonName: addon.name,
						curseforgeUrl,
					});
					return;
				}
			}
			setError(String(err));
		} finally {
			setBusyKeys((prev) => {
				const next = { ...prev };
				delete next[key];
				return next;
			});
			setProgressByKey((prev) => {
				const next = { ...prev };
				delete next[key];
				return next;
			});
		}
	}

	function installStateFor(
		addon: AddonSummary,
		installId: string,
	): BrowseInstallState {
		const key = jobKey(addon.id, installId);
		const list = installedByInstallId[installId] ?? [];
		const installed = findInstalledMatch(addon, list);
		const needsUpdate = installed ? browseNeedsUpdate(addon, installed) : false;
		return {
			installed,
			needsUpdate,
			busy: Boolean(busyKeys[key]),
			progress: progressByKey[key] ?? null,
		};
	}

	return (
		<div className="browse-layout">
			<FilterSidebar
				catalog={catalog}
				categories={categories}
				flavors={flavors}
				onToggleCategory={(value) => {
					applyLiveSearch();
					setPage(1);
					setCategories((prev) =>
						prev.includes(value) ? prev.filter((v) => v !== value) : [...prev, value],
					);
				}}
				onToggleFlavor={(id) => {
					applyLiveSearch();
					setPage(1);
					setFlavors((prev) =>
						prev.includes(id) ? prev.filter((v) => v !== id) : [...prev, id],
					);
				}}
				onClear={() => {
					applyLiveSearch();
					setCategories([]);
					setFlavors([]);
					setPage(1);
				}}
			/>

			<section className="main-pane">
				<div className="browse-search-row">
					<form
						className="browse-search"
						onSubmit={(e) => {
							e.preventDefault();
							const next = draftSearch.trim();
							setPage(1);
							if (next === search) {
								// Same query still needs visible feedback / refresh.
								setReloadToken((v) => v + 1);
							} else {
								setSearch(next);
							}
						}}
					>
						<input
							value={draftSearch}
							onChange={(e) => setDraftSearch(e.target.value)}
							placeholder={t("browse.searchPlaceholder")}
						/>
						<button
							type="submit"
							className="browse-search-btn"
							aria-label={t("browse.searchAria")}
							disabled={loading}
						>
							<svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
								<circle
									cx="11"
									cy="11"
									r="7"
									fill="none"
									stroke="currentColor"
									strokeWidth="2"
								/>
								<path
									d="M20 20l-3.5-3.5"
									fill="none"
									stroke="currentColor"
									strokeWidth="2"
									strokeLinecap="round"
								/>
							</svg>
						</button>
					</form>
				</div>

				{result?.message ? <div className="banner">{result.message}</div> : null}
				{error ? <div className="banner">{error}</div> : null}

				<div className="list-pane">
					{loading ? (
						<div className="list-loading" role="status" aria-live="polite">
							<div className="list-loading-spinner" aria-hidden="true" />
							<span>{t("browse.loadingList")}</span>
						</div>
					) : (
						<>
							{result?.items.map((addon) => {
								const installStateById: Record<string, BrowseInstallState> = {};
								for (const install of installs) {
									installStateById[install.id] = installStateFor(
										addon,
										install.id,
									);
								}
								return (
									<AddonCard
										key={addon.id}
										mode="browse"
										addon={addon}
										installs={installs}
										selectedFlavorTypeIds={flavors}
										installStateById={installStateById}
										onInstall={(installId) => handleInstall(addon, installId)}
										onOpenDetail={(modId, title) => setDetail({ modId, title })}
									/>
								);
							})}
							{result && result.items.length === 0 ? (
								<div className="empty">
									<strong>{t("browse.noMatchTitle")}</strong>
									{t("browse.noMatchHint")}
								</div>
							) : null}
						</>
					)}
				</div>

				<div className="pagination">
					<button
						className="btn"
						disabled={page <= 1 || loading}
						onClick={() => {
							applyLiveSearch();
							setPage((p) => Math.max(1, p - 1));
						}}
					>
						{t("browse.prevPage")}
					</button>
					<div className="page-info">
						{t("browse.pageInfo", { page, totalPages })}
						{result ? ` · ${t("browse.results", { total: result.total })}` : ""}
					</div>
					<button
						className="btn"
						disabled={page >= totalPages || loading}
						onClick={() => {
							applyLiveSearch();
							setPage((p) => p + 1);
						}}
					>
						{t("browse.nextPage")}
					</button>
				</div>
			</section>
			{detail ? (
				<AddonDetailModal
					modId={detail.modId}
					fallbackTitle={detail.title}
					onClose={() => setDetail(null)}
				/>
			) : null}
			{apiDisabledDialog ? (
				<ApiDistributionDisabledDialog
					addonName={apiDisabledDialog.addonName}
					curseforgeUrl={apiDisabledDialog.curseforgeUrl}
					onClose={() => setApiDisabledDialog(null)}
				/>
			) : null}
		</div>
	);
}
