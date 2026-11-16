import { convertFileSrc } from "@tauri-apps/api/core";
import { openAddonDetailPage, resolveAddonWebsiteUrl } from "../addonDetail";
import { useI18n } from "../i18n";
import { translateInstallLabel } from "../i18n/catalogLabels";
import type {
	AddonJobProgress,
	AddonSummary,
	InstalledAddon,
	WowInstall,
} from "../types";
import {
	compatibleInstalls,
	extraCategoryCount,
	formatDate,
	formatDownloads,
	primaryCategory,
} from "../utils";

function resolveThumbSrc(url: string): string {
	if (/^https?:\/\//i.test(url) || url.startsWith("data:") || url.startsWith("asset:")) {
		return url;
	}
	try {
		return convertFileSrc(url);
	} catch {
		return url;
	}
}

type BrowseProps = {
	mode: "browse";
	addon: AddonSummary;
	installs: WowInstall[];
	/** Sidebar-selected CurseForge gameVersionTypeIds; empty = no extra filter. */
	selectedFlavorTypeIds?: number[];
	busy?: boolean;
	onInstall: (installId: string) => void;
};

type InstalledProps = {
	mode: "installed";
	addon: InstalledAddon;
	busy?: boolean;
	updateProgress?: AddonJobProgress | null;
	onUpdate: () => void;
	onDelete: () => void;
};

type Props = BrowseProps | InstalledProps;

function Thumb({
	url,
	name,
	href,
	openLabel,
}: {
	url?: string | null;
	name: string;
	href?: string | null;
	openLabel: string;
}) {
	const initial = name.slice(0, 1).toUpperCase();
	const body = !url ? (
		<span className="thumb-fallback">{initial}</span>
	) : (
		<img
			src={resolveThumbSrc(url)}
			alt=""
			onError={(e) => {
				const img = e.currentTarget;
				img.style.display = "none";
				if (img.parentElement && !img.parentElement.querySelector(".thumb-fallback")) {
					const span = document.createElement("span");
					span.className = "thumb-fallback";
					span.textContent = initial;
					img.parentElement.appendChild(span);
				}
			}}
		/>
	);

	if (!href) {
		return <div className="thumb">{body}</div>;
	}

	return (
		<button
			type="button"
			className="thumb thumb-link"
			title={openLabel}
			aria-label={openLabel}
			onClick={() => {
				openAddonDetailPage(href, name).catch(console.error);
			}}
		>
			{body}
		</button>
	);
}

function TitleLink({
	title,
	href,
	openLabel,
}: {
	title: string;
	href?: string | null;
	openLabel: string;
}) {
	if (!href) {
		return <h3 className="card-title">{title}</h3>;
	}
	return (
		<h3 className="card-title">
			<button
				type="button"
				className="card-title-link"
				title={openLabel}
				aria-label={`${openLabel}: ${title}`}
				onClick={() => {
					openAddonDetailPage(href, title).catch(console.error);
				}}
			>
				{title}
			</button>
		</h3>
	);
}

export function AddonCard(props: Props) {
	const { t, locale } = useI18n();

	if (props.mode === "browse") {
		const { addon, installs, selectedFlavorTypeIds = [], busy, onInstall } = props;
		const extra = extraCategoryCount(addon.categories);
		const targets =
			installs.length === 0
				? []
				: compatibleInstalls(addon, installs, selectedFlavorTypeIds);
		const detailUrl = resolveAddonWebsiteUrl({
			websiteUrl: addon.websiteUrl,
			slug: addon.slug,
		});
		const openLabel = t("card.openDetail");
		return (
			<article className="addon-card addon-card-browse">
				<Thumb
					url={addon.thumbnailUrl}
					name={addon.name}
					href={detailUrl}
					openLabel={openLabel}
				/>
				<div className="card-body">
					<TitleLink title={addon.name} href={detailUrl} openLabel={openLabel} />
					<p className="card-author">
						{t("card.by", { author: addon.author || t("card.unknownAuthor") })}
					</p>
					<p className="card-summary">{addon.summary || t("card.noDescription")}</p>
					<div className="meta-row">
						<span className="tag">
							{primaryCategory(addon.categories)}
							{extra > 0 ? <span className="tag-extra">+{extra}</span> : null}
						</span>
						<span className="meta-item">↓ {formatDownloads(addon.downloadCount)}</span>
						<span className="meta-item">⏱ {formatDate(addon.updatedAt)}</span>
						<span className="meta-item">🎮 {addon.latestGameVersion || "-"}</span>
						{addon.flavors[0] ? (
							<span className="meta-item">
								{addon.flavors[0]}
								{addon.flavors.length > 1 ? ` +${addon.flavors.length - 1}` : ""}
							</span>
						) : null}
					</div>
				</div>
				<div className="card-actions card-actions-side">
					{installs.length === 0 ? (
						<span className="muted">{t("card.needWowPath")}</span>
					) : targets.length === 0 ? (
						<span className="muted">{t("card.noCompatibleInstall")}</span>
					) : (
						targets.map((install) => (
							<button
								key={install.id}
								className="btn primary"
								disabled={busy}
								onClick={() => onInstall(install.id)}
							>
								{t("card.installTo", {
									label: translateInstallLabel(
										install.flavorKey,
										install.label,
										locale,
									),
								})}
							</button>
						))
					)}
				</div>
			</article>
		);
	}

	const { addon, busy, updateProgress, onUpdate, onDelete } = props;
	const categories = addon.categories.filter(
		(c) => c.trim() && c.toLowerCase() !== "manual",
	);
	const extra = extraCategoryCount(categories);
	const updating = Boolean(busy && updateProgress);
	const detailUrl = resolveAddonWebsiteUrl({
		websiteUrl: addon.websiteUrl,
		slug: addon.curseforgeSlug,
	});
	const openLabel = t("card.openDetail");
	const progressLabel = (() => {
		if (!updateProgress) return null;
		if (updateProgress.stage === "extract") return t("card.updatingExtract");
		if (updateProgress.stage === "install") return t("card.updatingInstall");
		const pct = updateProgress.percent;
		if (typeof pct === "number") {
			return t("card.updatingDownloadPct", { percent: String(pct) });
		}
		return t("card.updatingDownload");
	})();
	const progressPct =
		updateProgress?.stage === "download" && typeof updateProgress.percent === "number"
			? Math.max(0, Math.min(100, updateProgress.percent))
			: updateProgress?.stage === "extract" || updateProgress?.stage === "install"
				? 100
				: 0;

	return (
		<article className="addon-card addon-card-installed">
			<Thumb
				url={addon.thumbnailUrl}
				name={addon.title}
				href={detailUrl}
				openLabel={openLabel}
			/>
			<div className="card-body">
				<TitleLink title={addon.title} href={detailUrl} openLabel={openLabel} />
				<p className="card-author">
					{t("card.by", { author: addon.author || t("card.unknownAuthor") })}
				</p>
				<p className="card-summary">{addon.notes || t("card.installedAddon")}</p>
				<div className="meta-row">
					{categories.length > 0 ? (
						<span className="tag">
							{primaryCategory(categories)}
							{extra > 0 ? <span className="tag-extra">+{extra}</span> : null}
						</span>
					) : null}
					<span className="meta-item">⏱ {formatDate(addon.updatedAt)}</span>
					<span className="meta-item">
						🎮 {addon.latestGameVersion || addon.interfaceVersion || "-"}
					</span>
					<span className="meta-item">
						{t("card.installedAt", { date: formatDate(addon.installedAt) })}
					</span>
					{(addon.relatedFolders?.length ?? 0) > 1 ? (
						<span className="meta-item">
							{t("card.suiteFolders", {
								count: String(addon.relatedFolders!.length),
							})}
						</span>
					) : null}
				</div>
			</div>
			<div className="card-actions card-actions-side">
				{updating && progressLabel ? (
					<div
						className="update-progress"
						role="status"
						aria-live="polite"
						aria-label={progressLabel}
					>
						<div
							className="update-progress-bar"
							style={{ width: `${progressPct}%` }}
						/>
						<span className="update-progress-label">{progressLabel}</span>
					</div>
				) : (
					<>
						{addon.hasUpdate ? (
							<button className="btn primary" disabled={busy} onClick={onUpdate}>
								{t("card.update")}
							</button>
						) : null}
						{!busy ? (
							<button className="btn danger" onClick={onDelete}>
								{t("card.delete")}
							</button>
						) : null}
					</>
				)}
			</div>
		</article>
	);
}
