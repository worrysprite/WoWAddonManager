use crate::curseforge;
use crate::types::{
	AddonScanCache, AddonScanProgress, AddonSummary, InstalledAddon, ManifestAddon, SearchParams,
	WowInstall,
};
use crate::wow::manifest;
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

type ScanProgressCb = Arc<dyn Fn(AddonScanProgress) + Send + Sync>;

pub const SCAN_CACHE_FILE: &str = "wow-addon-manager-scan.json";

pub async fn list_installed(
	install: &WowInstall,
	force_rescan: bool,
) -> Result<Vec<InstalledAddon>, String> {
	list_installed_with_progress(install, force_rescan, None).await
}

pub async fn list_installed_with_progress(
	install: &WowInstall,
	force_rescan: bool,
	on_progress: Option<ScanProgressCb>,
) -> Result<Vec<InstalledAddon>, String> {
	let addons_path = PathBuf::from(&install.addons_path);
	fs::create_dir_all(&addons_path).map_err(|e| e.to_string())?;

	if !force_rescan {
		if let Some(cache) = load_scan_cache(install) {
			if cache.scanned_at.is_some() {
				log::info!(
					"list_installed cache hit install={} count={}",
					install.label,
					cache.addons.len()
				);
				// Cache hit: no CurseForge search. Still localize any leftover CDN
				// thumbnail URLs into %AppData%/icons (no-op once already local).
				let before = cache.addons.len();
				let mut filtered = filter_unsupported_addons(install, cache.addons);
				filtered = crate::wow::suite::collapse_suite_rows(filtered);
				let _ = crate::wow::suite::sync_suite_flags_to_manifest(
					Path::new(&install.addons_path),
					&filtered,
				);
				let had_remote_thumbs = filtered.iter().any(|a| {
					a.thumbnail_url
						.as_deref()
						.is_some_and(|u| u.starts_with("http://") || u.starts_with("https://"))
				});
				crate::icon_cache::localize_installed_thumbnails(&mut filtered).await;
				if filtered.len() != before || had_remote_thumbs {
					let _ = save_scan_cache(install, &filtered);
				}
				return Ok(filtered);
			}
		}
	}

	log::warn!(
		"list_installed scanning install={} force={}",
		install.label,
		force_rescan
	);
	let mut results =
		scan_disk_with_updates(install, force_rescan, on_progress.as_ref()).await?;
	crate::icon_cache::localize_installed_thumbnails(&mut results).await;
	save_scan_cache(install, &results)?;
	log::info!(
		"list_installed scanned install={} count={}",
		install.label,
		results.len()
	);
	Ok(results)
}

/// After install / update / delete: refresh disk listing without network update checks.
pub fn refresh_local_cache(install: &WowInstall) -> Result<Vec<InstalledAddon>, String> {
	let addons_path = PathBuf::from(&install.addons_path);
	fs::create_dir_all(&addons_path).map_err(|e| e.to_string())?;

	let previous = load_scan_cache(install).unwrap_or_default();
	let prev_map: HashMap<_, _> = previous
		.addons
		.into_iter()
		.map(|a| (a.folder_name.clone(), a))
		.collect();

	let mut results = scan_disk_only(install, None)?;
	for addon in &mut results {
		if let Some(prev) = prev_map.get(&addon.folder_name) {
			// Keep prior update flags / icons when the installed file hasn't changed.
			if addon.file_id == prev.file_id {
				addon.has_update = prev.has_update;
				addon.remote_version = prev.remote_version.clone();
				addon.remote_file_id = prev.remote_file_id;
			}
			if addon.thumbnail_url.as_ref().map(|u| u.is_empty()).unwrap_or(true) {
				addon.thumbnail_url = prev.thumbnail_url.clone();
			}
			// Do not re-apply a previous wrong CurseForge id over a TOC-corrected one.
			if addon.curseforge_id.is_none() {
				addon.curseforge_id = prev.curseforge_id;
			}
			if addon.curseforge_slug.is_none()
				&& (addon.curseforge_id.is_none() || addon.curseforge_id == prev.curseforge_id)
			{
				addon.curseforge_slug = prev.curseforge_slug.clone();
			}
			if addon.website_url.as_ref().map(|u| u.is_empty()).unwrap_or(true) {
				addon.website_url = prev.website_url.clone();
			}
			if addon.related_folders.len() <= 1 && prev.related_folders.len() > 1 {
				addon.related_folders = prev.related_folders.clone();
				addon.is_primary = prev.is_primary;
			}
		}
	}

	let mut stored = manifest::load(&addons_path);
	if sync_curseforge_ids_from_scan(&mut stored, &results) {
		let _ = manifest::save(&addons_path, &stored);
	}

	save_scan_cache(install, &results)?;
	Ok(results)
}

pub fn remove_from_cache(install: &WowInstall, folder_name: &str) -> Result<(), String> {
	let Some(mut cache) = load_scan_cache(install) else {
		return Ok(());
	};
	cache.addons.retain(|a| {
		if a.folder_name == folder_name {
			return false;
		}
		!a.related_folders.iter().any(|f| f == folder_name)
	});
	cache.scanned_at = Some(Utc::now().to_rfc3339());
	cache.addons_path = Some(install.addons_path.clone());
	write_scan_cache(install, &cache)
}

fn sanitize_installed_list(mut addons: Vec<InstalledAddon>) -> Vec<InstalledAddon> {
	for addon in &mut addons {
		addon.title = crate::wow::toc::clean_toc_title(&addon.title);
		if let Some(author) = addon.author.take() {
			let cleaned = crate::wow::toc::clean_toc_text(&author);
			addon.author = (!cleaned.is_empty()).then_some(cleaned);
		}
		if let Some(notes) = addon.notes.take() {
			addon.notes = crate::wow::toc::clean_toc_notes(&notes);
		}
	}
	addons
}

/// Drop addons whose TOC marks them unsupported for this client (also cleans titles).
fn filter_unsupported_addons(
	install: &WowInstall,
	addons: Vec<InstalledAddon>,
) -> Vec<InstalledAddon> {
	let addons_path = PathBuf::from(&install.addons_path);
	let toc_ctx = crate::wow::toc::ClientContext::from_install(
		&install.flavor_key,
		install.game_version.as_deref(),
	);
	sanitize_installed_list(addons)
		.into_iter()
		.filter(|addon| {
			if let Some(issue) = crate::wow::toc::folder_unsupported_reason(
				&addons_path,
				&addon.folder_name,
				&toc_ctx,
			) {
				log::info!(
					"缓存中跳过不兼容插件 folder={} reason={}",
					addon.folder_name,
					issue.reason
				);
				return false;
			}
			true
		})
		.collect()
}

fn appdata_scan_cache_dir() -> Option<PathBuf> {
	let dir = dirs::config_dir()?.join("wow-addon-manager").join("scan-cache");
	fs::create_dir_all(&dir).ok()?;
	Some(dir)
}

fn appdata_scan_cache_path(install: &WowInstall) -> Option<PathBuf> {
	Some(appdata_scan_cache_dir()?.join(format!("{}.json", install.id)))
}

fn legacy_scan_cache_path(addons_path: &Path) -> PathBuf {
	addons_path.join(SCAN_CACHE_FILE)
}

fn normalize_path_key(path: &str) -> String {
	path.replace('\\', "/").to_lowercase()
}

fn load_scan_cache(install: &WowInstall) -> Option<AddonScanCache> {
	let addons_path = PathBuf::from(&install.addons_path);
	let appdata = appdata_scan_cache_path(install).and_then(|path| {
		let content = fs::read_to_string(path).ok()?;
		serde_json::from_str::<AddonScanCache>(&content).ok()
	});
	if let Some(cache) = appdata {
		if cache_matches_install(&cache, install) {
			return Some(cache);
		}
	}

	// Migrate legacy cache written inside the AddOns folder.
	let legacy = {
		let content = fs::read_to_string(legacy_scan_cache_path(&addons_path)).ok()?;
		serde_json::from_str::<AddonScanCache>(&content).ok()
	};
	if let Some(mut cache) = legacy {
		cache.addons_path = Some(install.addons_path.clone());
		let _ = write_scan_cache(install, &cache);
		let _ = fs::remove_file(legacy_scan_cache_path(&addons_path));
		return Some(cache);
	}
	None
}

fn cache_matches_install(cache: &AddonScanCache, install: &WowInstall) -> bool {
	match &cache.addons_path {
		Some(path) => normalize_path_key(path) == normalize_path_key(&install.addons_path),
		// Older caches without path still usable for this install id file.
		None => true,
	}
}

fn save_scan_cache(install: &WowInstall, addons: &[InstalledAddon]) -> Result<(), String> {
	let cache = AddonScanCache {
		version: 2,
		scanned_at: Some(Utc::now().to_rfc3339()),
		addons_path: Some(install.addons_path.clone()),
		addons: addons.to_vec(),
	};
	write_scan_cache(install, &cache)
}

fn write_scan_cache(install: &WowInstall, cache: &AddonScanCache) -> Result<(), String> {
	let path = appdata_scan_cache_path(install)
		.ok_or_else(|| "无法定位 %AppData%/wow-addon-manager/scan-cache".to_string())?;
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent).map_err(|e| e.to_string())?;
	}
	let content = serde_json::to_string_pretty(cache).map_err(|e| e.to_string())?;
	fs::write(path, content).map_err(|e| e.to_string())
}

fn strip_manual_category(addon: &mut InstalledAddon) {
	addon
		.categories
		.retain(|c| !c.eq_ignore_ascii_case("Manual"));
}

async fn scan_disk_with_updates(
	install: &WowInstall,
	check_remote: bool,
	on_progress: Option<&ScanProgressCb>,
) -> Result<Vec<InstalledAddon>, String> {
	let mut results = scan_disk_only(install, on_progress)?;
	let addons_path = PathBuf::from(&install.addons_path);
	let mut stored = manifest::load(&addons_path);
	let mut manifest_dirty = false;

	for addon in &mut results {
		strip_manual_category(addon);
	}

	// Collapse suites first so update checks run once per CurseForge package.
	results = crate::wow::suite::collapse_suite_rows(results);
	let _ = crate::wow::suite::sync_suite_flags_to_manifest(&addons_path, &results);

	if sync_curseforge_ids_from_scan(&mut stored, &results) {
		manifest_dirty = true;
	}

	if check_remote {
		// One batch request for all known CurseForge ids instead of N searches.
		let ids: Vec<i64> = results.iter().filter_map(|a| a.curseforge_id).collect();
		let by_id =
			curseforge::get_addons_by_ids(&ids, install.game_version_type_id).await;

		let batch_ok = !ids.is_empty() && !by_id.is_empty();
		for addon in &mut results {
			let remote = match addon.curseforge_id {
				Some(id) => {
					if let Some(item) = by_id.get(&id) {
						Some(item.clone())
					} else if !batch_ok {
						// Batch failed or empty key — fall back to a single GET, never search.
						curseforge::get_addon_for_flavor(id, install.game_version_type_id).await
					} else {
						None
					}
				}
				None => find_remote_via_search(addon, install.game_version_type_id).await,
			};
			// Retail-only CurseForge ids bound by mistake (e.g. PostBox 1680332 on
			// Forever) have no latest_file_id for this flavor — search again.
			let remote = match remote {
				Some(item)
					if install.game_version_type_id.is_some() && item.latest_file_id.is_none() =>
				{
					log::warn!(
						"installed {} bound to mod {} with no file for preferred={:?}; re-searching",
						addon.folder_name,
						item.id,
						install.game_version_type_id
					);
					find_remote_via_search(addon, install.game_version_type_id).await
				}
				other => other,
			};
			if let Some(remote) = remote {
				apply_update_flags(addon, &remote);
				if apply_remote_metadata(addon, &remote) {
					upsert_remote_enrichment(&mut stored, addon);
					manifest_dirty = true;
				}
			}
		}
	} else {
		// First cache-less open: still fill missing icons/ids once, but never bypass
		// browse cache and prefer id lookup / short search terms.
		for addon in &mut results {
			if !needs_thumbnail(addon) && addon.curseforge_id.is_some() {
				continue;
			}
			if let Some(remote) =
				find_remote_for_installed(addon, install.game_version_type_id).await
			{
				if apply_remote_metadata(addon, &remote) {
					upsert_remote_enrichment(&mut stored, addon);
					manifest_dirty = true;
				}
			}
		}
	}

	if manifest_dirty {
		let _ = manifest::save(&addons_path, &stored);
	}

	results.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
	Ok(results)
}

fn scan_disk_only(
	install: &WowInstall,
	on_progress: Option<&ScanProgressCb>,
) -> Result<Vec<InstalledAddon>, String> {
	let addons_path = PathBuf::from(&install.addons_path);
	fs::create_dir_all(&addons_path).map_err(|e| e.to_string())?;
	let stored = manifest::load(&addons_path);
	let stored_map: HashMap<_, _> = stored
		.addons
		.iter()
		.map(|a| (a.folder_name.clone(), a.clone()))
		.collect();

	let mut results = Vec::new();
	for entry in fs::read_dir(&addons_path).map_err(|e| e.to_string())? {
		let entry = entry.map_err(|e| e.to_string())?;
		let path = entry.path();
		if !path.is_dir() {
			continue;
		}
		let folder_name = entry.file_name().to_string_lossy().to_string();
		if folder_name.starts_with('.') {
			continue;
		}
		let toc_ctx = crate::wow::toc::ClientContext::from_install(
			&install.flavor_key,
			install.game_version.as_deref(),
		);
		if let Some(issue) =
			crate::wow::toc::folder_unsupported_reason(&addons_path, &folder_name, &toc_ctx)
		{
			log::info!(
				"跳过不兼容插件 folder={folder_name} reason={}",
				issue.reason
			);
			continue;
		}
		let meta = crate::wow::toc::read_folder_toc_for_client(&addons_path, &folder_name, &toc_ctx)
			.unwrap_or_default();

		let tracked = stored_map.get(&folder_name);
		let raw_title = tracked
			.and_then(|t| t.name.clone())
			.or(meta.title.clone())
			.unwrap_or_else(|| folder_name.clone());
		let raw_author = tracked
			.and_then(|t| t.author.clone())
			.or(meta.author.clone());
		let raw_notes = meta
			.notes
			.clone()
			.or_else(|| tracked.and_then(|t| t.summary.clone()));

		// Prefer TOC X-Curse-Project-ID over a stale slug/title bind in the manifest.
		let tracked_is_suite = tracked
			.map(|t| t.related_folders.len() > 1)
			.unwrap_or(false);
		let (curseforge_id, curseforge_slug) = prefer_toc_curseforge_bind(
			meta.curseforge_project_id,
			tracked.and_then(|t| t.curseforge_id),
			tracked.and_then(|t| t.curseforge_slug.clone()),
			tracked_is_suite,
		);

		let title = crate::wow::toc::clean_toc_title(&raw_title);
		results.push(InstalledAddon {
			folder_name: folder_name.clone(),
			title: title.clone(),
			author: raw_author.map(|v| crate::wow::toc::clean_toc_text(&v)),
			notes: raw_notes.and_then(|v| crate::wow::toc::clean_toc_notes(&v)),
			interface_version: meta.interface,
			version: tracked
				.and_then(|t| t.version.clone())
				.or(meta.version),
			curseforge_id,
			curseforge_slug,
			file_id: tracked.and_then(|t| t.file_id),
			thumbnail_url: tracked
				.and_then(|t| t.thumbnail_url.clone())
				.filter(|u| !u.is_empty()),
			categories: tracked
				.map(|t| {
					t.categories
						.iter()
						.filter(|c| !c.eq_ignore_ascii_case("Manual"))
						.cloned()
						.collect()
				})
				.unwrap_or_default(),
			installed_at: tracked.and_then(|t| t.installed_at.clone()),
			updated_at: tracked.and_then(|t| t.updated_at.clone()),
			download_count: tracked.and_then(|t| t.download_count),
			latest_game_version: tracked.and_then(|t| t.latest_game_version.clone()),
			flavors: tracked.map(|t| t.flavors.clone()).unwrap_or_default(),
			has_update: false,
			remote_version: None,
			remote_file_id: None,
			website_url: tracked.and_then(|t| t.website_url.clone()),
			related_folders: tracked
				.map(|t| t.related_folders.clone())
				.unwrap_or_default(),
			is_primary: tracked.map(|t| t.is_primary).unwrap_or(false),
			archive_name: tracked.and_then(|t| t.archive_name.clone()),
		});
		if let Some(cb) = on_progress {
			cb(AddonScanProgress {
				name: title,
				count: results.len() as u32,
			});
		}
	}

	let mut results = crate::wow::suite::collapse_suite_rows(results);
	let _ = crate::wow::suite::sync_suite_flags_to_manifest(&addons_path, &results);
	results.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
	Ok(results)
}

/// Update-matching bind policy:
/// - TOC `X-Curse-Project-ID` present → use that id (overrides stale manifest binds).
/// - TOC id absent + suite member (`related_folders` > 1) → keep tracked id/slug so
///   suite children stay grouped with the parent (many child TOCs omit project id).
/// - TOC id absent + solo → clear id/slug; callers fall back to name matching.
pub fn prefer_toc_curseforge_bind(
	toc_id: Option<i64>,
	tracked_id: Option<i64>,
	tracked_slug: Option<String>,
	tracked_is_suite_member: bool,
) -> (Option<i64>, Option<String>) {
	match toc_id {
		Some(toc) if tracked_id.is_some_and(|id| id != toc) => (Some(toc), None),
		Some(toc) => (Some(toc), tracked_slug),
		None if tracked_is_suite_member => (tracked_id, tracked_slug),
		None => (None, None),
	}
}

/// Persist TOC-corrected CurseForge ids into the on-disk manifest.
fn sync_curseforge_ids_from_scan(
	stored: &mut crate::types::AddonManifest,
	addons: &[InstalledAddon],
) -> bool {
	let mut dirty = false;
	for addon in addons {
		let Some(id) = addon.curseforge_id else {
			continue;
		};
		if let Some(entry) = stored
			.addons
			.iter_mut()
			.find(|a| a.folder_name == addon.folder_name)
		{
			if entry.curseforge_id != Some(id) {
				log::warn!(
					"TOC CurseForge id overrides stale bind folder={} old={:?} new={id}",
					addon.folder_name,
					entry.curseforge_id
				);
				entry.curseforge_id = Some(id);
				entry.curseforge_slug = addon.curseforge_slug.clone();
				dirty = true;
			} else if entry.curseforge_slug.is_none() && addon.curseforge_slug.is_some() {
				entry.curseforge_slug = addon.curseforge_slug.clone();
				dirty = true;
			}
		}
	}
	if dirty {
		stored.version = 1;
	}
	dirty
}

fn remote_matches_installed(addon: &InstalledAddon, item: &AddonSummary) -> bool {
	if let Some(id) = addon.curseforge_id {
		if item.id == id {
			return true;
		}
		// Known id that does not match this search hit — do not fall through to slug/title.
		return false;
	}
	if let Some(slug) = &addon.curseforge_slug {
		if item.slug.eq_ignore_ascii_case(slug) {
			return true;
		}
	}
	crate::wow::match_remote::names_loosely_equal(item.slug.as_str(), &addon.folder_name)
		|| crate::wow::match_remote::names_loosely_equal(item.name.as_str(), &addon.folder_name)
		|| crate::wow::match_remote::names_loosely_equal(item.name.as_str(), &addon.title)
}

fn needs_thumbnail(addon: &InstalledAddon) -> bool {
	addon
		.thumbnail_url
		.as_ref()
		.map(|u| u.trim().is_empty())
		.unwrap_or(true)
}

fn apply_update_flags(addon: &mut InstalledAddon, remote: &AddonSummary) {
	// No installable CurseForge file for this client flavor (e.g. Retail-only
	// project on Forever) → never show an update affordance.
	let Some(remote_file) = remote.latest_file_id else {
		return;
	};
	let newer_file = match addon.file_id {
		Some(local) => remote_file > local,
		None => true,
	};

	if newer_file {
		addon.has_update = true;
		addon.remote_version = Some(remote.updated_at.clone());
		addon.remote_file_id = Some(remote_file);
	}
}

/// Fill missing CurseForge metadata (icon, ids, categories). Returns true if anything changed.
fn apply_remote_metadata(addon: &mut InstalledAddon, remote: &AddonSummary) -> bool {
	let mut changed = false;

	if needs_thumbnail(addon) && !remote.thumbnail_url.trim().is_empty() {
		addon.thumbnail_url = Some(remote.thumbnail_url.clone());
		changed = true;
	}

	// Allow correcting a wrong Retail-only binding when a Forever-capable mod is found.
	if addon.curseforge_id != Some(remote.id) {
		if addon.curseforge_id.is_none() || remote.latest_file_id.is_some() {
			addon.curseforge_id = Some(remote.id);
			changed = true;
		}
	}
	// When bound by id (including TOC corrections), keep slug in sync with that mod.
	// Also allow correcting a Retail-only slug when a Forever-capable remote is found.
	if addon.curseforge_id == Some(remote.id)
		&& addon.curseforge_slug.as_deref() != Some(remote.slug.as_str())
	{
		addon.curseforge_slug = Some(remote.slug.clone());
		changed = true;
	} else if addon.curseforge_slug.as_deref() != Some(remote.slug.as_str()) {
		if addon.curseforge_slug.is_none() || remote.latest_file_id.is_some() {
			addon.curseforge_slug = Some(remote.slug.clone());
			changed = true;
		}
	}
	if addon.website_url.as_ref().map(|u| u.is_empty()).unwrap_or(true)
		&& !remote.website_url.is_empty()
	{
		addon.website_url = Some(remote.website_url.clone());
		changed = true;
	}

	let categories_empty_or_manual = addon.categories.is_empty()
		|| addon
			.categories
			.iter()
			.all(|c| c.eq_ignore_ascii_case("Manual"));
	if categories_empty_or_manual && !remote.categories.is_empty() {
		addon.categories = remote.categories.clone();
		changed = true;
	}

	if addon.flavors.is_empty() && !remote.flavors.is_empty() {
		addon.flavors = remote.flavors.clone();
		changed = true;
	}
	if addon
		.latest_game_version
		.as_ref()
		.map(|v| v.is_empty())
		.unwrap_or(true)
		&& !remote.latest_game_version.is_empty()
	{
		addon.latest_game_version = Some(remote.latest_game_version.clone());
		changed = true;
	}

	changed
}

fn upsert_remote_enrichment(stored: &mut crate::types::AddonManifest, addon: &InstalledAddon) {
	if let Some(entry) = stored
		.addons
		.iter_mut()
		.find(|a| a.folder_name == addon.folder_name)
	{
		if entry.thumbnail_url.as_ref().map(|u| u.is_empty()).unwrap_or(true) {
			entry.thumbnail_url = addon.thumbnail_url.clone();
		}
		if entry.curseforge_id != addon.curseforge_id && addon.curseforge_id.is_some() {
			entry.curseforge_id = addon.curseforge_id;
			entry.curseforge_slug = addon.curseforge_slug.clone();
		} else {
			if entry.curseforge_id.is_none() {
				entry.curseforge_id = addon.curseforge_id;
			}
			if entry.curseforge_slug.as_deref() != addon.curseforge_slug.as_deref()
				&& addon.curseforge_slug.is_some()
				&& (entry.curseforge_slug.is_none() || entry.curseforge_id == addon.curseforge_id)
			{
				entry.curseforge_slug = addon.curseforge_slug.clone();
			}
		}
		if entry.website_url.as_ref().map(|u| u.is_empty()).unwrap_or(true) {
			entry.website_url = addon.website_url.clone();
		}
		let cats_empty_or_manual = entry.categories.is_empty()
			|| entry
				.categories
				.iter()
				.all(|c| c.eq_ignore_ascii_case("Manual"));
		if cats_empty_or_manual && !addon.categories.is_empty() {
			entry.categories = addon.categories.clone();
		}
		if entry.flavors.is_empty() && !addon.flavors.is_empty() {
			entry.flavors = addon.flavors.clone();
		}
		if entry
			.latest_game_version
			.as_ref()
			.map(|v| v.is_empty())
			.unwrap_or(true)
		{
			entry.latest_game_version = addon.latest_game_version.clone();
		}
	} else {
		stored.addons.push(ManifestAddon {
			folder_name: addon.folder_name.clone(),
			curseforge_id: addon.curseforge_id,
			curseforge_slug: addon.curseforge_slug.clone(),
			file_id: addon.file_id,
			name: Some(addon.title.clone()),
			author: addon.author.clone(),
			summary: addon.notes.clone(),
			thumbnail_url: addon.thumbnail_url.clone(),
			categories: addon.categories.clone(),
			flavors: addon.flavors.clone(),
			download_count: addon.download_count,
			latest_game_version: addon.latest_game_version.clone(),
			website_url: addon.website_url.clone(),
			installed_at: addon.installed_at.clone(),
			updated_at: addon.updated_at.clone(),
			version: addon.version.clone(),
			install_source: Some("manual".into()),
			archive_name: addon.archive_name.clone(),
			related_folders: addon.related_folders.clone(),
			is_primary: addon.is_primary,
		});
	}
	stored.version = 1;
}

async fn find_remote_for_installed(
	addon: &InstalledAddon,
	preferred_type_id: Option<i64>,
) -> Option<AddonSummary> {
	if let Some(id) = addon.curseforge_id {
		if let Some(summary) = curseforge::get_addon_for_flavor(id, preferred_type_id).await {
			if preferred_type_id.is_none() || summary.latest_file_id.is_some() {
				return Some(summary);
			}
		}
	}
	find_remote_via_search(addon, preferred_type_id).await
}

/// Name-based match for installed addons. Always uses browse cache (no force_refresh);
/// folder then title, at most two terms. Prefer folder/title over a possibly-wrong
/// Retail-only slug when a flavor filter is active.
async fn find_remote_via_search(
	addon: &InstalledAddon,
	preferred_type_id: Option<i64>,
) -> Option<AddonSummary> {
	let mut terms = Vec::new();
	terms.push(addon.folder_name.clone());
	if !addon.title.is_empty()
		&& !terms
			.iter()
			.any(|t| t.eq_ignore_ascii_case(&addon.title))
	{
		terms.push(addon.title.clone());
	}

	for term in terms.into_iter().take(2) {
		let result = curseforge::search_addons(
			SearchParams {
				search: Some(term),
				page: Some(1),
				page_size: Some(10),
				categories: None,
				game_version_type_ids: preferred_type_id.map(|id| vec![id]),
				sort_by: Some("relevancy".into()),
			},
			false,
		)
		.await;
		if let Some(item) = result
			.items
			.into_iter()
			.find(|item| remote_matches_installed(addon, item))
		{
			if let Some(resolved) =
				curseforge::get_addon_for_flavor(item.id, preferred_type_id).await
			{
				if preferred_type_id.is_none() || resolved.latest_file_id.is_some() {
					return Some(resolved);
				}
				continue;
			}
			if preferred_type_id.is_none() {
				return Some(item);
			}
		}
	}
	None
}


#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::{AddonSummary, WowInstall};

	fn sample_installed(title: &str) -> InstalledAddon {
		InstalledAddon {
			folder_name: title.into(),
			title: title.into(),
			author: None,
			notes: None,
			interface_version: None,
			version: None,
			curseforge_id: None,
			curseforge_slug: None,
			file_id: None,
			thumbnail_url: None,
			categories: vec!["Manual".into()],
			installed_at: None,
			updated_at: None,
			download_count: None,
			latest_game_version: None,
			flavors: vec![],
			has_update: false,
			remote_version: None,
			remote_file_id: None,
			website_url: None,
			related_folders: vec![],
			is_primary: false,
			archive_name: None,
		}
	}

	#[test]
	fn prefer_toc_id_overrides_stale_postbox_bind() {
		let (id, slug) = prefer_toc_curseforge_bind(
			Some(1639171),
			Some(1680332),
			Some("postbox".into()),
			false,
		);
		assert_eq!(id, Some(1639171));
		assert_eq!(slug, None);
	}

	#[test]
	fn prefer_toc_id_keeps_slug_when_ids_agree() {
		let (id, slug) =
			prefer_toc_curseforge_bind(Some(42), Some(42), Some("weakauras-2".into()), false);
		assert_eq!(id, Some(42));
		assert_eq!(slug.as_deref(), Some("weakauras-2"));
	}

	#[test]
	fn prefer_toc_absent_clears_bind_for_name_fallback() {
		let (id, slug) =
			prefer_toc_curseforge_bind(None, Some(1680332), Some("postbox".into()), false);
		assert_eq!(id, None);
		assert_eq!(slug, None);
	}

	#[test]
	fn prefer_toc_absent_keeps_suite_member_bind() {
		let (id, slug) =
			prefer_toc_curseforge_bind(None, Some(3358), Some("questie".into()), true);
		assert_eq!(id, Some(3358));
		assert_eq!(slug.as_deref(), Some("questie"));
	}

	#[test]
	fn remote_matches_by_folder_name_without_curseforge_meta() {
		let addon = sample_installed("WeakAuras");
		let item = AddonSummary {
			id: 1,
			slug: "weakauras-2".into(),
			name: "WeakAuras".into(),
			author: String::new(),
			summary: String::new(),
			thumbnail_url: String::new(),
			download_count: 0,
			updated_at: String::new(),
			latest_game_version: String::new(),
			categories: vec![],
			flavors: vec![],
			website_url: String::new(),
			latest_file_id: Some(99),
		};
		assert!(remote_matches_installed(&addon, &item));
	}

	#[test]
	fn remote_matches_rejects_slug_hit_when_id_differs() {
		let mut addon = sample_installed("Postbox");
		addon.curseforge_id = Some(1639171);
		addon.curseforge_slug = None;
		let retail = AddonSummary {
			id: 1680332,
			slug: "postbox".into(),
			name: "PostBox".into(),
			author: String::new(),
			summary: String::new(),
			thumbnail_url: String::new(),
			download_count: 0,
			updated_at: String::new(),
			latest_game_version: String::new(),
			categories: vec![],
			flavors: vec![],
			website_url: String::new(),
			latest_file_id: Some(1),
		};
		assert!(!remote_matches_installed(&addon, &retail));
	}

	#[test]
	fn apply_update_flags_ignores_remote_without_file() {
		let mut addon = sample_installed("PostBox");
		addon.file_id = Some(1);
		addon.updated_at = Some("2026-01-01".into());
		let remote = AddonSummary {
			id: 1680332,
			slug: "postbox".into(),
			name: "PostBox".into(),
			author: String::new(),
			summary: String::new(),
			thumbnail_url: String::new(),
			download_count: 0,
			updated_at: "2026-11-15".into(),
			latest_game_version: String::new(),
			categories: vec![],
			flavors: vec!["Retail".into()],
			website_url: String::new(),
			latest_file_id: None,
		};
		apply_update_flags(&mut addon, &remote);
		assert!(!addon.has_update);
		assert!(addon.remote_file_id.is_none());
	}

	#[test]
	fn apply_remote_metadata_fills_missing_thumbnail_and_clears_manual() {
		let mut addon = sample_installed("WeakAuras");
		let remote = AddonSummary {
			id: 42,
			slug: "weakauras-2".into(),
			name: "WeakAuras".into(),
			author: "Author".into(),
			summary: String::new(),
			thumbnail_url: "https://example.com/icon.png".into(),
			download_count: 10,
			updated_at: "2026-01-01".into(),
			latest_game_version: "11.0.0".into(),
			categories: vec!["Buffs & Debuffs".into()],
			flavors: vec!["Retail".into()],
			website_url: "https://www.curseforge.com/wow/addons/weakauras-2".into(),
			latest_file_id: Some(99),
		};
		assert!(apply_remote_metadata(&mut addon, &remote));
		assert_eq!(
			addon.thumbnail_url.as_deref(),
			Some("https://example.com/icon.png")
		);
		assert_eq!(addon.curseforge_id, Some(42));
		assert_eq!(addon.categories, vec!["Buffs & Debuffs".to_string()]);
		assert!(!needs_thumbnail(&addon));
	}

	fn temp_install() -> (std::path::PathBuf, WowInstall) {
		let dir = std::env::temp_dir().join(format!(
			"wam-scan-cache-{}-{}",
			std::process::id(),
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		let _ = fs::remove_dir_all(&dir);
		let addons = dir.join("AddOns");
		fs::create_dir_all(&addons).unwrap();
		let addon_dir = addons.join("CachedAddon");
		fs::create_dir_all(&addon_dir).unwrap();
		fs::write(
			addon_dir.join("CachedAddon.toc"),
			"## Interface: 120100\n## Title: Cached Addon\n## Author: Test\n## Version: 1.0\n",
		)
		.unwrap();
		let install = WowInstall {
			id: "test".into(),
			label: "Test".into(),
			flavor: "Retail".into(),
			flavor_key: "retail".into(),
			install_path: dir.display().to_string(),
			addons_path: addons.display().to_string(),
			game_version: None,
			game_version_type_id: None,
			backup_path: String::new(),
		};
		(dir, install)
	}

	#[test]
	fn local_cache_roundtrip_skips_rescan() {
		let (dir, install) = temp_install();
		let first = refresh_local_cache(&install).unwrap();
		assert_eq!(first.len(), 1);
		assert_eq!(first[0].folder_name, "CachedAddon");
		let cache_path = appdata_scan_cache_path(&install).expect("appdata cache path");
		assert!(cache_path.exists());

		let cached = load_scan_cache(&install).unwrap();
		assert!(cached.scanned_at.is_some());
		assert_eq!(cached.addons.len(), 1);
		assert_eq!(
			cached.addons_path.as_deref(),
			Some(install.addons_path.as_str())
		);

		let _ = fs::remove_file(cache_path);
		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn scan_disk_prefers_toc_project_id_over_stale_manifest() {
		let dir = std::env::temp_dir().join(format!(
			"wam-scan-toc-id-{}-{}",
			std::process::id(),
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		let _ = fs::remove_dir_all(&dir);
		let addons = dir.join("AddOns");
		let addon_dir = addons.join("Postbox");
		fs::create_dir_all(&addon_dir).unwrap();
		fs::write(
			addon_dir.join("Postbox.toc"),
			"## Interface: 16001\n## Title: PostBox\n## X-Curse-Project-ID: 1639171\n",
		)
		.unwrap();
		manifest::upsert(
			&addons,
			ManifestAddon {
				folder_name: "Postbox".into(),
				curseforge_id: Some(1680332),
				curseforge_slug: Some("postbox".into()),
				file_id: None,
				name: Some("PostBox".into()),
				author: None,
				summary: None,
				thumbnail_url: None,
				categories: vec![],
				flavors: vec![],
				download_count: None,
				latest_game_version: None,
				website_url: None,
				installed_at: None,
				updated_at: None,
				version: None,
				install_source: Some("manual".into()),
				archive_name: None,
				related_folders: vec![],
				is_primary: false,
			},
		)
		.unwrap();

		let install = WowInstall {
			id: "forever-test".into(),
			label: "Forever".into(),
			flavor: "Forever".into(),
			flavor_key: "forever".into(),
			install_path: dir.display().to_string(),
			addons_path: addons.display().to_string(),
			game_version: Some("1.0.0.16001".into()),
			game_version_type_id: Some(88568),
			backup_path: String::new(),
		};
		let list = scan_disk_only(&install, None).unwrap();
		assert_eq!(list.len(), 1);
		assert_eq!(list[0].curseforge_id, Some(1639171));
		assert_eq!(list[0].curseforge_slug, None);

		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn migrates_legacy_addons_folder_cache_to_appdata() {
		let (dir, install) = temp_install();
		let legacy = AddonScanCache {
			version: 1,
			scanned_at: Some("2026-01-01T00:00:00Z".into()),
			addons_path: None,
			addons: vec![sample_installed("CachedAddon")],
		};
		let legacy_path = legacy_scan_cache_path(Path::new(&install.addons_path));
		fs::write(
			&legacy_path,
			serde_json::to_string_pretty(&legacy).unwrap(),
		)
		.unwrap();

		let cached = load_scan_cache(&install).unwrap();
		assert_eq!(cached.addons.len(), 1);
		assert!(appdata_scan_cache_path(&install).unwrap().exists());
		assert!(!legacy_path.exists());

		let _ = fs::remove_file(appdata_scan_cache_path(&install).unwrap());
		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn scan_disk_collapses_suite_when_child_toc_omits_project_id() {
		let dir = std::env::temp_dir().join(format!(
			"wam-scan-suite-{}-{}",
			std::process::id(),
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		let _ = fs::remove_dir_all(&dir);
		let addons = dir.join("AddOns");
		for (folder, toc) in [
			(
				"Questie",
				"## Interface: 16001\n## Title: Questie\n## Notes: A standalone Classic QuestHelper\n## Author: Questie Team\n## X-Curse-Project-ID: 3358\n",
			),
			(
				"QuestieCompat",
				"## Interface: 16001\n## Title: Questie\n## Notes: Compatibility layer\n## Author: Questie Team\n",
			),
			(
				"QuestieDB",
				"## Interface: 16001\n## Title: QuestieDB\n## Notes: Database\n## Author: Questie Team\n## X-Curse-Project-ID: 9999\n",
			),
		] {
			let path = addons.join(folder);
			fs::create_dir_all(&path).unwrap();
			fs::write(path.join(format!("{folder}.toc")), toc).unwrap();
		}

		let related = vec!["Questie".into(), "QuestieCompat".into()];
		for (folder, id, primary) in [
			("Questie", Some(3358_i64), true),
			("QuestieCompat", Some(3358), false),
			("QuestieDB", Some(9999), true),
		] {
			manifest::upsert(
				&addons,
				ManifestAddon {
					folder_name: folder.into(),
					curseforge_id: id,
					curseforge_slug: Some(if folder == "QuestieDB" {
						"questiedb".into()
					} else {
						"questie".into()
					}),
					file_id: None,
					name: Some(folder.into()),
					author: Some("Questie Team".into()),
					summary: None,
					thumbnail_url: None,
					categories: vec![],
					flavors: vec![],
					download_count: None,
					latest_game_version: None,
					website_url: None,
					installed_at: None,
					updated_at: None,
					version: None,
					install_source: Some("curseforge".into()),
					archive_name: None,
					related_folders: if folder == "QuestieDB" {
						vec!["QuestieDB".into()]
					} else {
						related.clone()
					},
					is_primary: primary,
				},
			)
			.unwrap();
		}

		let install = WowInstall {
			id: "suite-rescan".into(),
			label: "Forever".into(),
			flavor: "Forever".into(),
			flavor_key: "forever".into(),
			install_path: dir.display().to_string(),
			addons_path: addons.display().to_string(),
			game_version: Some("1.0.0.16001".into()),
			game_version_type_id: Some(88568),
			backup_path: String::new(),
		};
		let list = scan_disk_only(&install).unwrap();
		assert_eq!(list.len(), 2, "Questie suite + QuestieDB, no duplicate parent");
		assert!(list.iter().any(|a| a.folder_name == "Questie"));
		assert!(list.iter().any(|a| a.folder_name == "QuestieDB"));
		assert!(list.iter().all(|a| a.folder_name != "QuestieCompat"));
		let questie = list.iter().find(|a| a.folder_name == "Questie").unwrap();
		assert_eq!(questie.related_folders.len(), 2);
		assert_eq!(questie.curseforge_id, Some(3358));

		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn scan_skips_game_client_not_supported() {
		let (dir, mut install) = temp_install();
		install.game_version = Some("12.1.5.70783".into());
		let bad = PathBuf::from(&install.addons_path).join("Questie");
		fs::create_dir_all(&bad).unwrap();
		fs::write(
			bad.join("Questie.toc"),
			"## Interface: 11507\n## Title: Questie|cFFFF0000 game client not supported|r\n## Notes: Questie only supports Classic official servers.\n## Author: Questie Team\n## Version: 1.0\n",
		)
		.unwrap();

		let listed = scan_disk_only(&install, None).unwrap();
		assert!(listed.iter().all(|a| a.folder_name != "Questie"));
		assert_eq!(listed.len(), 1);
		assert_eq!(listed[0].folder_name, "CachedAddon");

		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn scan_disk_emits_progress_per_addon() {
		let (dir, install) = temp_install();
		let events = Arc::new(std::sync::Mutex::new(Vec::<AddonScanProgress>::new()));
		let events_cb = Arc::clone(&events);
		let cb: ScanProgressCb = Arc::new(move |progress: AddonScanProgress| {
			events_cb.lock().unwrap().push(progress);
		});
		let listed = scan_disk_only(&install, Some(&cb)).unwrap();
		let events = events.lock().unwrap();
		assert!(!events.is_empty());
		for (idx, event) in events.iter().enumerate() {
			assert_eq!(event.count, (idx + 1) as u32);
			assert!(!event.name.is_empty());
		}
		// Progress counts pre-suite-collapse folders; listed is collapsed.
		assert!(events.last().unwrap().count as usize >= listed.len());

		let _ = fs::remove_dir_all(dir);
	}
}
