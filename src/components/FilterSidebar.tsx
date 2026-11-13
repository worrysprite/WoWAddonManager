import { useI18n } from "../i18n";
import {
	translateCategoryLabel,
	translateFlavorLabel,
} from "../i18n/catalogLabels";
import type { FilterCatalog } from "../types";

type Props = {
	catalog: FilterCatalog | null;
	categories: string[];
	flavors: number[];
	onToggleCategory: (value: string) => void;
	onToggleFlavor: (id: number) => void;
	onClear: () => void;
};

export function FilterSidebar({
	catalog,
	categories,
	flavors,
	onToggleCategory,
	onToggleFlavor,
	onClear,
}: Props) {
	const { t, locale } = useI18n();
	const activeCount = categories.length + flavors.length;

	return (
		<aside className="filters">
			<div className="filters-header">
				<h2>
					{t("filters.title")}
					{activeCount > 0 ? <span className="filter-count">{activeCount}</span> : null}
				</h2>
				{activeCount > 0 ? (
					<button className="clear-btn" onClick={onClear}>
						{t("filters.clearAll")}
					</button>
				) : null}
			</div>

			<details className="filter-section" open>
				<summary>{t("filters.categories")}</summary>
				<div className="filter-list">
					{(catalog?.categories ?? []).map((item) => (
						<label key={item.id} className="filter-item">
							<input
								type="checkbox"
								checked={categories.includes(item.value)}
								onChange={() => onToggleCategory(item.value)}
							/>
							<span>
								{translateCategoryLabel(item.value, item.label, locale)}
							</span>
						</label>
					))}
				</div>
			</details>

			<details className="filter-section" open>
				<summary>{t("filters.flavor")}</summary>
				<div className="filter-list">
					{(catalog?.flavors ?? []).map((item) => (
						<label key={item.id} className="filter-item">
							<input
								type="checkbox"
								checked={flavors.includes(item.gameVersionTypeId)}
								onChange={() => onToggleFlavor(item.gameVersionTypeId)}
							/>
							<span>
								{translateFlavorLabel(item.id, item.label, locale)}
							</span>
						</label>
					))}
				</div>
			</details>

			<details className="filter-section" open>
				<summary>{t("filters.gameVersion")}</summary>
				<div className="filter-list">
					{(catalog?.gameVersions ?? []).map((item) => (
						<label key={item.id} className="filter-item">
							<input type="checkbox" disabled />
							<span>{item.label}</span>
						</label>
					))}
				</div>
			</details>
		</aside>
	);
}
