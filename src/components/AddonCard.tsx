import { convertFileSrc } from "@tauri-apps/api/core";
import { resolveAddonModId } from "../addonDetail";
import { useI18n, type MessageKey } from "../i18n";
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

export type BrowseInstallState = {
	installed: InstalledAddon | null;
	needsUpdate: boolean;
	busy: boolean;
	progress: AddonJobProgress | null;
};

type BrowseProps = {
	mode: "browse";
	addon: AddonSummary;
	installs: WowInstall[];
	/** Sidebar-selected CurseForge gameVersionTypeIds; empty = no extra filter. */
	selectedFlavorTypeIds?: number[];
	/** Per-install-id job / installed state for this Browse card. */
	installStateById?: Record<string, BrowseInstallState>;
	onInstall: (installId: string) => void;
	onOpenDetail?: (modId: number, title: string) => void;
};

type InstalledProps = {
	mode: "installed";
	addon: InstalledAddon;
	busy?: boolean;
	updateProgress?: AddonJobProgress | null;
	onUpdate: () => void;
	onDelete: () => void;
	onOpenDetail?: (modId: number, title: string) => void;
};

type Props = BrowseProps | InstalledProps;

function Thumb({
	url,
	name,
	openLabel,
	onOpen,
}: {
	url?: string | null;
	name: string;
	openLabel: string;
	onOpen?: () => void;
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

	if (!onOpen) {
		return <div className="thumb">{body}</div>;
	}

	return (
		<button
			type="button"
			className="thumb thumb-link"
			title={openLabel}
			aria-label={openLabel}
			onClick={onOpen}
		>
			{body}
		</button>
	);
}

function TitleLink({
	title,
	openLabel,
	onOpen,
}: {
	title: string;
	openLabel: string;
	onOpen?: () => void;
}) {
	if (!onOpen) {
		return <h3 className="card-title">{title}</h3>;
	}
	return (
		<h3 className="card-title">
			<button
				type="button"
				className="card-title-link"
				title={openLabel}
				aria-label={`${openLabel}: ${title}`}
				onClick={onOpen}
			>
				{title}
			</button>
		</h3>
	);
}

function jobProgressLabel(
	t: (key: MessageKey, vars?: Record<string, string | number>) => string,
	progress: AddonJobProgress | null | undefined,
): string | null {
	if (!progress) return null;
	if (progress.stage === "extract") return t("card.updatingExtract");
	if (progress.stage === "install") return t("card.updatingInstall");
	const pct = progress.percent;
	if (typeof pct === "number") {
		return t("card.updatingDownloadPct", { percent: String(pct) });
	}
	return t("card.updatingDownload");
}

function jobProgressPct(progress: AddonJobProgress | null | undefined): number {
	if (!progress) return 0;
	if (progress.stage === "download" && typeof progress.percent === "number") {
		return Math.max(0, Math.min(100, progress.percent));
	}
	if (progress.stage === "extract" || progress.stage === "install") {
		return 100;
	}
	return 0;
}

function ProgressStatus({
	label,
	percent,
}: {
	label: string;
	percent: number;
}) {
	return (
		<div
			className="update-progress"
			role="status"
			aria-live="polite"
			aria-label={label}
		>
			<div className="update-progress-bar" style={{ width: `${percent}%` }} />
			<span className="update-progress-label">{label}</span>
		</div>
	);
}

function browseTargetVersion(addon: AddonSummary): string {
	const fromListing = addon.latestVersion?.trim();
	if (fromListing) return fromListing;
	const fromDate = addon.updatedAt?.trim();
	return fromDate || "";
}

function browseInstalledVersion(
	addon: AddonSummary,
	installed: InstalledAddon | null,
): string {
	const local = installed?.version?.trim();
	if (local) return local;
	return browseTargetVersion(addon);
}

export function AddonCard(props: Props) {
	const { t, locale } = useI18n();

	if (props.mode === "browse") {
		const {
			addon,
			installs,
			selectedFlavorTypeIds = [],
			installStateById = {},
			onInstall,
			onOpenDetail,
		} = props;
		const extra = extraCategoryCount(addon.categories);
		const targets =
			installs.length === 0
				? []
				: compatibleInstalls(addon, installs, selectedFlavorTypeIds);
		const modId = resolveAddonModId({ id: addon.id });
		const openLabel = t("card.openDetail");
		const openDetail =
			modId && onOpenDetail
				? () => onOpenDetail(modId, addon.name)
				: undefined;
		const targetVersion = browseTargetVersion(addon);
		return (
			<article className="addon-card addon-card-browse">
				<Thumb
					url={addon.thumbnailUrl}
					name={addon.name}
					openLabel={openLabel}
					onOpen={openDetail}
				/>
				<div className="card-body">
					<TitleLink title={addon.name} openLabel={openLabel} onOpen={openDetail} />
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
						targets.map((install) => {
							const state = installStateById[install.id];
							const busy = Boolean(state?.busy);
							const progress = state?.progress ?? null;
							const progressLabel = jobProgressLabel(t, progress);
							if (busy && progressLabel) {
								return (
									<ProgressStatus
										key={install.id}
										label={progressLabel}
										percent={jobProgressPct(progress)}
									/>
								);
							}
							const installed = state?.installed ?? null;
							if (installed && !state?.needsUpdate) {
								const version = browseInstalledVersion(addon, installed);
								return (
									<button
										key={install.id}
										type="button"
										className="btn"
										disabled
										title={translateInstallLabel(
											install.flavorKey,
											install.label,
											locale,
										)}
									>
										{version
											? t("card.versionInstalled", { version })
											: t("card.versionInstalled", { version: "—" })}
									</button>
								);
							}
							if (installed && state?.needsUpdate) {
								const version = targetVersion || browseInstalledVersion(addon, installed);
								return (
									<button
										key={install.id}
										type="button"
										className="btn primary"
										disabled={busy}
										onClick={() => onInstall(install.id)}
										title={translateInstallLabel(
											install.flavorKey,
											install.label,
											locale,
										)}
									>
										{version
											? t("card.updateToVersion", { version })
											: t("card.update")}
									</button>
								);
							}
							return (
								<button
									key={install.id}
									type="button"
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
							);
						})
					)}
				</div>
			</article>
		);
	}

	const { addon, busy, updateProgress, onUpdate, onDelete, onOpenDetail } = props;
	const categories = addon.categories.filter(
		(c) => c.trim() && c.toLowerCase() !== "manual",
	);
	const extra = extraCategoryCount(categories);
	const updating = Boolean(busy && updateProgress);
	const modId = resolveAddonModId({ curseforgeId: addon.curseforgeId });
	const openLabel = t("card.openDetail");
	const openDetail =
		modId && onOpenDetail ? () => onOpenDetail(modId, addon.title) : undefined;
	const progressLabel = jobProgressLabel(t, updateProgress);
	const progressPct = jobProgressPct(updateProgress);

	return (
		<article className="addon-card addon-card-installed">
			<Thumb
				url={addon.thumbnailUrl}
				name={addon.title}
				openLabel={openLabel}
				onOpen={openDetail}
			/>
			<div className="card-body">
				<TitleLink title={addon.title} openLabel={openLabel} onOpen={openDetail} />
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
					<ProgressStatus label={progressLabel} percent={progressPct} />
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
