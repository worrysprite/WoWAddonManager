import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import * as api from "../api";
import { sanitizeAddonHtml } from "../addonDetail";
import { useI18n } from "../i18n";
import type { AddonDetail } from "../types";
import { formatDate, formatDownloads } from "../utils";

type Props = {
	modId: number;
	fallbackTitle?: string;
	onClose: () => void;
};

function resolveMediaSrc(url: string): string {
	if (/^https?:\/\//i.test(url) || url.startsWith("data:") || url.startsWith("asset:")) {
		return url;
	}
	try {
		return convertFileSrc(url);
	} catch {
		return url;
	}
}

export function AddonDetailModal({ modId, fallbackTitle, onClose }: Props) {
	const { t } = useI18n();
	const [detail, setDetail] = useState<AddonDetail | null>(null);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [activeShot, setActiveShot] = useState(0);

	useEffect(() => {
		let cancelled = false;
		setLoading(true);
		setError(null);
		setDetail(null);
		setActiveShot(0);
		api
			.getAddonDetail(modId)
			.then((data) => {
				if (!cancelled) setDetail(data);
			})
			.catch((err) => {
				if (!cancelled) setError(String(err));
			})
			.finally(() => {
				if (!cancelled) setLoading(false);
			});
		return () => {
			cancelled = true;
		};
	}, [modId]);

	useEffect(() => {
		function onKey(event: KeyboardEvent) {
			if (event.key === "Escape") onClose();
		}
		window.addEventListener("keydown", onKey);
		return () => window.removeEventListener("keydown", onKey);
	}, [onClose]);

	const title = detail?.addon.name || fallbackTitle || t("detail.title");
	const shots = detail?.screenshots ?? [];
	const active = shots[activeShot] ?? null;
	const descriptionHtml = sanitizeAddonHtml(detail?.descriptionHtml || "");

	return (
		<div
			className="modal-backdrop"
			role="presentation"
			onClick={(event) => {
				if (event.target === event.currentTarget) onClose();
			}}
		>
			<div
				className="modal-card detail-modal"
				role="dialog"
				aria-modal="true"
				aria-labelledby="addon-detail-title"
			>
				<div className="detail-modal-header">
					<div className="detail-modal-heading">
						{detail?.addon.thumbnailUrl ? (
							<img
								className="detail-modal-thumb"
								src={resolveMediaSrc(detail.addon.thumbnailUrl)}
								alt=""
							/>
						) : null}
						<div>
							<h3 id="addon-detail-title">{title}</h3>
							{detail ? (
								<p className="muted">
									{t("card.by", {
										author: detail.addon.author || t("card.unknownAuthor"),
									})}
								</p>
							) : null}
						</div>
					</div>
					<button className="btn" type="button" onClick={onClose}>
						{t("detail.close")}
					</button>
				</div>

				{loading ? (
					<div className="list-loading" role="status" aria-live="polite">
						<div className="list-loading-spinner" aria-hidden="true" />
						<span>{t("detail.loading")}</span>
					</div>
				) : null}

				{error ? <div className="banner">{error}</div> : null}

				{detail ? (
					<div className="detail-modal-body">
						<div className="meta-row detail-meta">
							<span className="meta-item">
								↓ {formatDownloads(detail.addon.downloadCount)}
							</span>
							<span className="meta-item">⏱ {formatDate(detail.addon.updatedAt)}</span>
							<span className="meta-item">
								🎮 {detail.addon.latestGameVersion || "-"}
							</span>
							{detail.addon.categories[0] ? (
								<span className="tag">{detail.addon.categories[0]}</span>
							) : null}
							{detail.addon.flavors.length > 0 ? (
								<span className="meta-item">
									{detail.addon.flavors.slice(0, 3).join(" · ")}
									{detail.addon.flavors.length > 3
										? ` +${detail.addon.flavors.length - 3}`
										: ""}
								</span>
							) : null}
						</div>

						{detail.addon.summary ? (
							<p className="detail-summary">{detail.addon.summary}</p>
						) : null}

						{shots.length > 0 ? (
							<section className="detail-shots">
								<h4>{t("detail.screenshots")}</h4>
								{active ? (
									<div className="detail-shot-main">
										<img
											src={resolveMediaSrc(active.url)}
											alt={active.title || title}
										/>
									</div>
								) : null}
								{shots.length > 1 ? (
									<div className="detail-shot-strip">
										{shots.map((shot, index) => (
											<button
												key={`${shot.id}-${index}`}
												type="button"
												className={`detail-shot-thumb${
													index === activeShot ? " active" : ""
												}`}
												onClick={() => setActiveShot(index)}
											>
												<img
													src={resolveMediaSrc(shot.thumbnailUrl || shot.url)}
													alt={shot.title || `${index + 1}`}
												/>
											</button>
										))}
									</div>
								) : null}
							</section>
						) : null}

						<section className="detail-description">
							<h4>{t("detail.description")}</h4>
							{descriptionHtml.trim() ? (
								<div
									className="detail-description-html"
									// CurseForge returns HTML; sanitized above.
									dangerouslySetInnerHTML={{ __html: descriptionHtml }}
								/>
							) : (
								<p className="muted">{t("detail.noDescription")}</p>
							)}
						</section>
					</div>
				) : null}
			</div>
		</div>
	);
}
