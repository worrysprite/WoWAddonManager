use crate::curseforge;
use crate::types::{AddonScanCache, InstalledAddon, SearchParams, WowInstall};
use crate::wow::manifest;
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const SCAN_CACHE_FILE: &str = "wow-addon-manager-scan.json";

pub async fn list_installed(install: &WowInstall, force_rescan: bool) -> Result<Vec<InstalledAddon>, String> {
	let addons_path = PathBuf::from(&install.addons_path);
	fs::create_dir_all(&addons_path).map_err(|e| e.to_string())?;

	if !force_rescan {
		if let Some(cache) = load_scan_cache(&addons_path) {
			if cache.scanned_at.is_some() {
				return Ok(cache.addons);
			}
		}
	}

	let results = scan_disk_with_updates(install).await?;
	save_scan_cache(&addons_path, &results)?;
	Ok(results)
}

/// After install / update / delete: refresh disk listing without network update checks.
pub fn refresh_local_cache(install: &WowInstall) -> Result<Vec<InstalledAddon>, String> {
	let addons_path = PathBuf::from(&install.addons_path);
	fs::create_dir_all(&addons_path).map_err(|e| e.to_string())?;

	let previous = load_scan_cache(&addons_path).unwrap_or_default();
	let prev_map: HashMap<_, _> = previous
		.addons
		.into_iter()
		.map(|a| (a.folder_name.clone(), a))
		.collect();

	let mut results = scan_disk_only(install)?;
	for addon in &mut results {
		if let Some(prev) = prev_map.get(&addon.folder_name) {
			// Keep prior update flags when the installed file hasn't changed.
			if addon.file_id == prev.file_id {
				addon.has_update = prev.has_update;
				addon.remote_version = prev.remote_version.clone();
				addon.remote_file_id = prev.remote_file_id;
			}
		}
	}

	save_scan_cache(&addons_path, &results)?;
	Ok(results)
}

pub fn remove_from_cache(install: &WowInstall, folder_name: &str) -> Result<(), String> {
	let addons_path = PathBuf::from(&install.addons_path);
	let Some(mut cache) = load_scan_cache(&addons_path) else {
		return Ok(());
	};
	cache.addons.retain(|a| a.folder_name != folder_name);
	cache.scanned_at = Some(Utc::now().to_rfc3339());
	write_scan_cache(&addons_path, &cache)
}

fn scan_cache_path(addons_path: &Path) -> PathBuf {
	addons_path.join(SCAN_CACHE_FILE)
}

fn load_scan_cache(addons_path: &Path) -> Option<AddonScanCache> {
	let content = fs::read_to_string(scan_cache_path(addons_path)).ok()?;
	serde_json::from_str(&content).ok()
}

fn save_scan_cache(addons_path: &Path, addons: &[InstalledAddon]) -> Result<(), String> {
	let cache = AddonScanCache {
		version: 1,
		scanned_at: Some(Utc::now().to_rfc3339()),
		addons: addons.to_vec(),
	};
	write_scan_cache(addons_path, &cache)
}

fn write_scan_cache(addons_path: &Path, cache: &AddonScanCache) -> Result<(), String> {
	fs::create_dir_all(addons_path).map_err(|e| e.to_string())?;
	let content = serde_json::to_string_pretty(cache).map_err(|e| e.to_string())?;
	fs::write(scan_cache_path(addons_path), content).map_err(|e| e.to_string())
}

async fn scan_disk_with_updates(install: &WowInstall) -> Result<Vec<InstalledAddon>, String> {
	let mut results = scan_disk_only(install)?;
	for addon in &mut results {
		if let Some(update) = check_update(addon).await {
			addon.has_update = true;
			addon.remote_version = update.0;
			addon.remote_file_id = update.1;
		}
	}
	results.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
	Ok(results)
}

fn scan_disk_only(install: &WowInstall) -> Result<Vec<InstalledAddon>, String> {
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
		let toc = find_toc(&path, &folder_name);
		let meta = toc.as_ref().map(|p| parse_toc(p)).unwrap_or_default();
		let tracked = stored_map.get(&folder_name);

		results.push(InstalledAddon {
			folder_name: folder_name.clone(),
			title: tracked
				.and_then(|t| t.name.clone())
				.or(meta.title)
				.unwrap_or(folder_name.clone()),
			author: tracked
				.and_then(|t| t.author.clone())
				.or(meta.author),
			notes: meta.notes,
			interface_version: meta.interface,
			version: tracked
				.and_then(|t| t.version.clone())
				.or(meta.version),
			curseforge_id: tracked.and_then(|t| t.curseforge_id),
			curseforge_slug: tracked.and_then(|t| t.curseforge_slug.clone()),
			file_id: tracked.and_then(|t| t.file_id),
			thumbnail_url: tracked.and_then(|t| t.thumbnail_url.clone()),
			categories: tracked.map(|t| t.categories.clone()).unwrap_or_default(),
			installed_at: tracked.and_then(|t| t.installed_at.clone()),
			updated_at: tracked.and_then(|t| t.updated_at.clone()),
			download_count: tracked.and_then(|t| t.download_count),
			latest_game_version: tracked.and_then(|t| t.latest_game_version.clone()),
			flavors: tracked.map(|t| t.flavors.clone()).unwrap_or_default(),
			has_update: false,
			remote_version: None,
			remote_file_id: None,
			website_url: tracked.and_then(|t| t.website_url.clone()),
		});
	}

	results.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
	Ok(results)
}

async fn check_update(addon: &InstalledAddon) -> Option<(Option<String>, Option<i64>)> {
	let search_term = addon
		.curseforge_slug
		.clone()
		.or_else(|| Some(addon.title.clone()))?;
	let result = curseforge::search_addons(
		SearchParams {
			search: Some(search_term),
			page: Some(1),
			page_size: Some(5),
			categories: None,
			game_version_type_ids: None,
			sort_by: Some("relevancy".into()),
		},
		false,
	)
	.await;

	let remote = result.items.into_iter().find(|item| {
		if let Some(id) = addon.curseforge_id {
			return item.id == id;
		}
		if let Some(slug) = &addon.curseforge_slug {
			return item.slug == *slug;
		}
		item.name.eq_ignore_ascii_case(&addon.title)
	})?;

	let remote_file = remote.latest_file_id;
	let local_file = addon.file_id;
	let newer_file = match (local_file, remote_file) {
		(Some(local), Some(remote)) if remote > local => true,
		(None, Some(_)) => true,
		_ => match (&addon.updated_at, &remote.updated_at) {
			(Some(local), remote_date) if !remote_date.is_empty() => remote_date > local,
			(None, remote_date) if !remote_date.is_empty() => true,
			_ => false,
		},
	};

	if newer_file {
		Some((Some(remote.updated_at), remote_file))
	} else {
		None
	}
}

#[derive(Default)]
struct TocMeta {
	title: Option<String>,
	author: Option<String>,
	notes: Option<String>,
	version: Option<String>,
	interface: Option<String>,
}

fn find_toc(dir: &Path, folder_name: &str) -> Option<PathBuf> {
	let preferred = dir.join(format!("{folder_name}.toc"));
	if preferred.exists() {
		return Some(preferred);
	}
	for entry in fs::read_dir(dir).ok()?.flatten() {
		let path = entry.path();
		if path.extension().and_then(|s| s.to_str()) == Some("toc") {
			return Some(path);
		}
	}
	None
}

fn parse_toc(path: &Path) -> TocMeta {
	let content = fs::read_to_string(path).unwrap_or_default();
	let mut meta = TocMeta::default();
	for line in content.lines() {
		let line = line.trim();
		if let Some(rest) = line.strip_prefix("## Title:") {
			meta.title = Some(strip_color(rest.trim()).to_string());
		} else if let Some(rest) = line.strip_prefix("## Author:") {
			meta.author = Some(rest.trim().to_string());
		} else if let Some(rest) = line.strip_prefix("## Notes:") {
			meta.notes = Some(strip_color(rest.trim()).to_string());
		} else if let Some(rest) = line.strip_prefix("## Version:") {
			meta.version = Some(rest.trim().to_string());
		} else if let Some(rest) = line.strip_prefix("## Interface:") {
			meta.interface = Some(rest.trim().to_string());
		}
	}
	meta
}

fn strip_color(input: &str) -> &str {
	input
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::WowInstall;

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
		};
		(dir, install)
	}

	#[test]
	fn local_cache_roundtrip_skips_rescan() {
		let (dir, install) = temp_install();
		let first = refresh_local_cache(&install).unwrap();
		assert_eq!(first.len(), 1);
		assert_eq!(first[0].folder_name, "CachedAddon");
		assert!(scan_cache_path(Path::new(&install.addons_path)).exists());

		let cached = load_scan_cache(Path::new(&install.addons_path)).unwrap();
		assert!(cached.scanned_at.is_some());
		assert_eq!(cached.addons.len(), 1);

		let _ = fs::remove_dir_all(dir);
	}
}
