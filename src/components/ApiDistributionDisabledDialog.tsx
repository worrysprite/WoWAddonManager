import { useEffect } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useI18n } from "../i18n";

export type ApiDisabledAddonEntry = {
	addonName: string;
	curseforgeUrl: string;
};

export type ApiDistributionDisabledDialogProps = {
	addons: ApiDisabledAddonEntry[];
	onClose: () => void;
};

export function ApiDistributionDisabledDialog({
	addons,
	onClose,
}: ApiDistributionDisabledDialogProps) {
	const { t } = useI18n();

	useEffect(() => {
		function onKey(event: KeyboardEvent) {
			if (event.key === "Escape") onClose();
		}
		window.addEventListener("keydown", onKey);
		return () => window.removeEventListener("keydown", onKey);
	}, [onClose]);

	async function handleOpen(url: string) {
		try {
			await openUrl(url);
		} catch (err) {
			console.error(err);
		}
	}

	return (
		<div
			className="modal-backdrop"
			role="presentation"
			onClick={(event) => {
				if (event.target === event.currentTarget) onClose();
			}}
		>
			<div
				className="modal-card api-disabled-modal"
				role="dialog"
				aria-modal="true"
				aria-labelledby="api-disabled-title"
			>
				<div className="api-disabled-header">
					<h3 id="api-disabled-title">{t("apiDisabled.title")}</h3>
					<button
						className="api-disabled-close"
						type="button"
						aria-label={t("apiDisabled.close")}
						onClick={onClose}
					>
						×
					</button>
				</div>
				<p className="api-disabled-body">{t("apiDisabled.body")}</p>
				<ul className="api-disabled-list">
					{addons.map((entry) => (
						<li
							key={entry.curseforgeUrl}
							className="api-disabled-row"
						>
							<span className="api-disabled-row-name" title={entry.addonName}>
								{entry.addonName}
							</span>
							<button
								className="btn primary"
								type="button"
								onClick={() => void handleOpen(entry.curseforgeUrl)}
							>
								{t("apiDisabled.openCurseForge")}
							</button>
						</li>
					))}
				</ul>
				<p className="muted api-disabled-hint">{t("apiDisabled.hint")}</p>
			</div>
		</div>
	);
}
