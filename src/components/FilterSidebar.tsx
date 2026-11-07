import type { FilterCatalog } from "../types";

type Props = {
	catalog: FilterCatalog | null;
	categories: string[];
	flavors: number[];
	versionQuery: string;
	onVersionQuery: (value: string) => void;
	onToggleCategory: (value: string) => void;
	onToggleFlavor: (id: number) => void;
	onClear: () => void;
};

export function FilterSidebar({
	catalog,
	categories,
	flavors,
	versionQuery,
	onVersionQuery,
	onToggleCategory,
	onToggleFlavor,
	onClear,
}: Props) {
	const activeCount = categories.length + flavors.length;

	return (
		<aside className="filters">
			<div className="filters-header">
				<h2>
					Filters
					{activeCount > 0 ? <span className="filter-count">{activeCount}</span> : null}
				</h2>
				{activeCount > 0 ? (
					<button className="clear-btn" onClick={onClear}>
						Clear all
					</button>
				) : null}
			</div>

			<details className="filter-section" open>
				<summary>Categories</summary>
				<div className="filter-list">
					{(catalog?.categories ?? []).map((item) => (
						<label key={item.id} className="filter-item">
							<input
								type="checkbox"
								checked={categories.includes(item.value)}
								onChange={() => onToggleCategory(item.value)}
							/>
							<span>{item.label}</span>
						</label>
					))}
				</div>
			</details>

			<details className="filter-section" open>
				<summary>Flavor</summary>
				<div className="filter-list">
					{(catalog?.flavors ?? []).map((item) => (
						<label key={item.id} className="filter-item">
							<input
								type="checkbox"
								checked={flavors.includes(item.gameVersionTypeId)}
								onChange={() => onToggleFlavor(item.gameVersionTypeId)}
							/>
							<span>{item.label}</span>
						</label>
					))}
				</div>
			</details>

			<details className="filter-section" open>
				<summary>Game Version</summary>
				<input
					className="version-search"
					placeholder="Search versions"
					value={versionQuery}
					onChange={(e) => onVersionQuery(e.target.value)}
				/>
				<div className="filter-list">
					{(catalog?.gameVersions ?? [])
						.filter((item) =>
							item.label.toLowerCase().includes(versionQuery.trim().toLowerCase()),
						)
						.map((item) => (
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
