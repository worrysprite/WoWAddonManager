import { useEffect } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useI18n } from "../i18n";

export type ApiDistributionDisabledDialogProps = {
	addonName: string;
	curseforgeUrl: string;
	onClose: () => void;
};

export function ApiDistributionDisabledDialog({
	addonName,
	curseforgeUrl,
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

	async function handleOpen() {
		try {
			await openUrl(curseforgeUrl);
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
				<h3 id="api-disabled-title">{t("apiDisabled.title")}</h3>
				<p className="api-disabled-body">
					{t("apiDisabled.body", { name: addonName })}
				</p>
				<p className="muted api-disabled-hint">{t("apiDisabled.hint")}</p>
				<div className="api-disabled-actions">
					<button className="btn" type="button" onClick={onClose}>
						{t("apiDisabled.close")}
					</button>
					<button className="btn primary" type="button" onClick={handleOpen}>
						{t("apiDisabled.openCurseForge")}
					</button>
				</div>
			</div>
		</div>
	);
}
