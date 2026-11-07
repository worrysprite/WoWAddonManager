use crate::curseforge;
use crate::types::{InstalledAddon, SearchParams, WowInstall};
use crate::wow::manifest;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub async fn list_installed(install: &WowInstall) -> Result<Vec<InstalledAddon>, String> {
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

		let mut addon = InstalledAddon {
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
		};

		if let Some(update) = check_update(&addon).await {
			addon.has_update = true;
			addon.remote_version = update.0;
			addon.remote_file_id = update.1;
		}

		results.push(addon);
	}

	results.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
	Ok(results)
}

async fn check_update(addon: &InstalledAddon) -> Option<(Option<String>, Option<i64>)> {
	let search_term = addon
		.curseforge_slug
		.clone()
		.or_else(|| Some(addon.title.clone()))?;
	let result = curseforge::search_addons(SearchParams {
		search: Some(search_term),
		page: Some(1),
		page_size: Some(5),
		categories: None,
		game_version_type_ids: None,
		sort_by: Some("relevancy".into()),
	})
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
		_ => {
			// Fallback: compare updated date string if present
			match (&addon.updated_at, &remote.updated_at) {
				(Some(local), remote_date) if !remote_date.is_empty() => remote_date > local,
				(None, remote_date) if !remote_date.is_empty() => true,
				_ => false,
			}
		}
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
	// Keep simple: remove leading |cXXXXXXXX if present by returning raw; UI can show as-is.
	input
}
