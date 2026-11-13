use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonSummary {
	pub id: i64,
	pub slug: String,
	pub name: String,
	pub author: String,
	pub summary: String,
	pub thumbnail_url: String,
	pub download_count: i64,
	pub updated_at: String,
	pub latest_game_version: String,
	pub categories: Vec<String>,
	pub flavors: Vec<String>,
	pub website_url: String,
	pub latest_file_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
	pub items: Vec<AddonSummary>,
	pub page: u32,
	pub page_size: u32,
	pub total: u32,
	pub source: String,
	pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
	pub search: Option<String>,
	pub page: Option<u32>,
	pub page_size: Option<u32>,
	pub categories: Option<Vec<String>>,
	pub game_version_type_ids: Option<Vec<i64>>,
	pub sort_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterOption {
	pub id: String,
	pub label: String,
	pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlavorOption {
	pub id: String,
	pub label: String,
	pub game_version_type_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterCatalog {
	pub categories: Vec<FilterOption>,
	pub flavors: Vec<FlavorOption>,
	pub game_versions: Vec<FilterOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WowInstall {
	pub id: String,
	pub label: String,
	pub flavor: String,
	pub flavor_key: String,
	pub install_path: String,
	pub addons_path: String,
	pub game_version: Option<String>,
	pub game_version_type_id: Option<i64>,
	/// Per-client Interface/WTF backup directory (empty when unset).
	#[serde(default)]
	pub backup_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledAddon {
	pub folder_name: String,
	pub title: String,
	pub author: Option<String>,
	pub notes: Option<String>,
	pub interface_version: Option<String>,
	pub version: Option<String>,
	pub curseforge_id: Option<i64>,
	pub curseforge_slug: Option<String>,
	pub file_id: Option<i64>,
	pub thumbnail_url: Option<String>,
	pub categories: Vec<String>,
	pub installed_at: Option<String>,
	pub updated_at: Option<String>,
	pub download_count: Option<i64>,
	pub latest_game_version: Option<String>,
	pub flavors: Vec<String>,
	pub has_update: bool,
	pub remote_version: Option<String>,
	pub remote_file_id: Option<i64>,
	pub website_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
	pub install_id: String,
	pub addon: AddonSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequest {
	pub install_id: String,
	pub folder_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRequest {
	pub install_id: String,
	pub folder_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualInstallRequest {
	pub install_id: String,
	pub archive_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestAddon {
	pub folder_name: String,
	pub curseforge_id: Option<i64>,
	pub curseforge_slug: Option<String>,
	pub file_id: Option<i64>,
	pub name: Option<String>,
	pub author: Option<String>,
	pub summary: Option<String>,
	pub thumbnail_url: Option<String>,
	pub categories: Vec<String>,
	pub flavors: Vec<String>,
	pub download_count: Option<i64>,
	pub latest_game_version: Option<String>,
	pub website_url: Option<String>,
	pub installed_at: Option<String>,
	pub updated_at: Option<String>,
	pub version: Option<String>,
	#[serde(default)]
	pub install_source: Option<String>,
	#[serde(default)]
	pub archive_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddonManifest {
	pub version: u32,
	pub addons: Vec<ManifestAddon>,
}

/// Cached result of an AddOns folder scan (disk + optional update checks).
/// Stored under %AppData%/wow-addon-manager/scan-cache/ as JSON.
/// Thumbnails stay as remote URLs (not embedded binary) — JSON is enough.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddonScanCache {
	pub version: u32,
	pub scanned_at: Option<String>,
	/// AddOns folder this cache belongs to (detect path moves / remounts).
	#[serde(default)]
	pub addons_path: Option<String>,
	pub addons: Vec<InstalledAddon>,
}
