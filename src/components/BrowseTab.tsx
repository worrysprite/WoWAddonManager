import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "../api";
import { useI18n } from "../i18n";
import type { AddonSummary, FilterCatalog, SearchResult, WowInstall } from "../types";
import { AddonCard } from "./AddonCard";
import { FilterSidebar } from "./FilterSidebar";

type Props = {
	installs: WowInstall[];
	onInstalled: () => void;
};

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
	const [busyId, setBusyId] = useState<number | null>(null);
	const [error, setError] = useState<string | null>(null);
	const requestSeq = useRef(0);

	useEffect(() => {
		api.getFilterCatalog().then(setCatalog).catch(console.error);
	}, []);

	async function load(forceRefresh: boolean) {
		const seq = ++requestSeq.current;
		setLoading(true);
		setError(null);
		try {
			const data = await api.searchAddons(
				{
					search,
					page,
					pageSize: 20,
					categories,
					gameVersionTypeIds: flavors,
					sortBy: "relevancy",
				},
				forceRefresh,
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
		load(false);
	}, [search, page, categories, flavors]);

	const totalPages = useMemo(() => {
		if (!result) return 1;
		return Math.max(1, Math.ceil(result.total / result.pageSize));
	}, [result]);

	async function handleInstall(addon: AddonSummary, installId: string) {
		setBusyId(addon.id);
		try {
			await api.installAddon(installId, addon);
			onInstalled();
		} catch (err) {
			setError(String(err));
		} finally {
			setBusyId(null);
		}
	}

	return (
		<div className="browse-layout">
			<FilterSidebar
				catalog={catalog}
				categories={categories}
				flavors={flavors}
				onToggleCategory={(value) => {
					setPage(1);
					setCategories((prev) =>
						prev.includes(value) ? prev.filter((v) => v !== value) : [...prev, value],
					);
				}}
				onToggleFlavor={(id) => {
					setPage(1);
					setFlavors((prev) =>
						prev.includes(id) ? prev.filter((v) => v !== id) : [...prev, id],
					);
				}}
				onClear={() => {
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
							setPage(1);
							setSearch(draftSearch.trim());
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
					<button
						type="button"
						className="btn browse-refresh-btn"
						disabled={loading}
						onClick={() => load(true)}
						title={t("browse.refreshTitle")}
					>
						{loading ? t("browse.loading") : t("browse.refresh")}
					</button>
				</div>

				<p className="browse-attribution">
					{t("browse.attributionBefore")}
					<a
						href="https://www.curseforge.com/"
						target="_blank"
						rel="noreferrer"
					>
						CurseForge
					</a>
					{t("browse.attributionAfter")}
				</p>

				{result?.message ? <div className="banner">{result.message}</div> : null}
				{error ? <div className="banner">{error}</div> : null}

				<div className="list-pane">
					{loading && !result ? <div className="empty">{t("browse.loadingList")}</div> : null}
					{result?.items.map((addon) => (
						<AddonCard
							key={addon.id}
							mode="browse"
							addon={addon}
							installs={installs}
							busy={busyId === addon.id}
							onInstall={(installId) => handleInstall(addon, installId)}
						/>
					))}
					{result && result.items.length === 0 ? (
						<div className="empty">
							<strong>{t("browse.noMatchTitle")}</strong>
							{t("browse.noMatchHint")}
						</div>
					) : null}
				</div>

				<div className="pagination">
					<button
						className="btn"
						disabled={page <= 1 || loading}
						onClick={() => setPage((p) => Math.max(1, p - 1))}
					>
						{t("browse.prevPage")}
					</button>
					<div className="page-info">
						{t("browse.pageInfo", { page, totalPages })}
						{result ? ` · ${t("browse.results", { total: result.total })}` : ""}
						{result ? ` · ${result.source}` : ""}
					</div>
					<button
						className="btn"
						disabled={page >= totalPages || loading}
						onClick={() => setPage((p) => p + 1)}
					>
						{t("browse.nextPage")}
					</button>
				</div>
			</section>
		</div>
	);
}
