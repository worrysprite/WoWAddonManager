//! Multi-folder CurseForge / manual zip suites (e.g. EllesmereUI).
//! One download installs many AddOns folders; the UI shows one primary row.

use crate::types::InstalledAddon;
use crate::wow::manifest;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Choose the main folder for a suite (slug/name match, then common-prefix parent).
pub fn pick_primary_folder(
	folders: &[String],
	slug: Option<&str>,
	name: Option<&str>,
) -> String {
	if folders.is_empty() {
		return String::new();
	}
	if folders.len() == 1 {
		return folders[0].clone();
	}

	if let Some(slug) = slug.map(str::trim).filter(|s| !s.is_empty()) {
		let sanitized = sanitize_folder(slug);
		if let Some(hit) = folders.iter().find(|f| {
			f.eq_ignore_ascii_case(slug) || f.eq_ignore_ascii_case(&sanitized)
		}) {
			return hit.clone();
		}
	}

	if let Some(name) = name.map(str::trim).filter(|s| !s.is_empty()) {
		let compact = alnum_only(name);
		if let Some(hit) = folders.iter().find(|f| {
			f.eq_ignore_ascii_case(name) || alnum_only(f).eq_ignore_ascii_case(&compact)
		}) {
			return hit.clone();
		}
	}

	// EllesmereUI + EllesmereUI_Foo: the shared prefix folder is the core.
	if let Some(hit) = folders.iter().find(|candidate| {
		let prefix_us = format!("{}_", candidate.to_lowercase());
		let prefix_dash = format!("{}-", candidate.to_lowercase());
		folders.iter().filter(|o| o.as_str() != candidate.as_str()).all(|other| {
			let lower = other.to_lowercase();
			lower.starts_with(&prefix_us) || lower.starts_with(&prefix_dash)
		})
	}) {
		return hit.clone();
	}

	folders
		.iter()
		.min_by_key(|f| (f.len(), f.to_lowercase()))
		.cloned()
		.unwrap_or_else(|| folders[0].clone())
}

/// All folders belonging to the same install suite as `folder_name`.
pub fn suite_folders_for(addons_path: &Path, folder_name: &str) -> Vec<String> {
	let stored = manifest::load(addons_path);
	if let Some(entry) = manifest::find(&stored, folder_name) {
		if entry.related_folders.len() > 1 {
			let mut folders = entry.related_folders.clone();
			folders.sort();
			folders.dedup();
			if !folders.iter().any(|f| f == folder_name) {
				folders.push(folder_name.to_string());
				folders.sort();
			}
			return folders;
		}
		if let Some(id) = entry.curseforge_id {
			let mut folders: Vec<String> = stored
				.addons
				.iter()
				.filter(|a| a.curseforge_id == Some(id))
				.map(|a| a.folder_name.clone())
				.collect();
			if folders.len() > 1 {
				folders.sort();
				folders.dedup();
				return folders;
			}
		}
		if let (Some(arch), Some(at)) = (&entry.archive_name, &entry.installed_at) {
			let mut folders: Vec<String> = stored
				.addons
				.iter()
				.filter(|a| {
					a.archive_name.as_deref() == Some(arch.as_str())
						&& a.installed_at.as_deref() == Some(at.as_str())
				})
				.map(|a| a.folder_name.clone())
				.collect();
			if folders.len() > 1 {
				folders.sort();
				folders.dedup();
				return folders;
			}
		}
	}
	vec![folder_name.to_string()]
}

/// Collapse multi-folder suites so the installed list shows one row per package.
pub fn collapse_suite_rows(addons: Vec<InstalledAddon>) -> Vec<InstalledAddon> {
	if addons.len() <= 1 {
		return addons;
	}

	let mut groups: HashMap<String, Vec<InstalledAddon>> = HashMap::new();
	let mut order: Vec<String> = Vec::new();

	for addon in addons {
		let key = suite_group_key(&addon);
		if !groups.contains_key(&key) {
			order.push(key.clone());
		}
		groups.entry(key).or_default().push(addon);
	}

	let mut collapsed = Vec::new();
	for key in order {
		let Some(mut members) = groups.remove(&key) else {
			continue;
		};
		if members.len() == 1 {
			let mut only = members.pop().unwrap();
			if only.related_folders.is_empty() {
				only.related_folders = vec![only.folder_name.clone()];
			}
			collapsed.push(only);
			continue;
		}

		let folders: Vec<String> = members.iter().map(|m| m.folder_name.clone()).collect();
		let slug = members.iter().find_map(|m| m.curseforge_slug.clone());
		let name = members.iter().find_map(|m| {
			if m.curseforge_id.is_some() {
				Some(m.title.clone())
			} else {
				None
			}
		});
		let primary_name = members
			.iter()
			.find(|m| m.is_primary)
			.map(|m| m.folder_name.clone())
			.unwrap_or_else(|| pick_primary_folder(&folders, slug.as_deref(), name.as_deref()));

		let mut primary = members
			.iter()
			.find(|m| m.folder_name == primary_name)
			.cloned()
			.unwrap_or_else(|| members[0].clone());

		primary.is_primary = true;
		primary.related_folders = folders;
		primary.has_update = members.iter().any(|m| m.has_update);
		if primary.remote_file_id.is_none() {
			primary.remote_file_id = members.iter().find_map(|m| m.remote_file_id);
		}
		if primary.remote_version.is_none() {
			primary.remote_version = members.iter().find_map(|m| m.remote_version.clone());
		}
		collapsed.push(primary);
	}

	collapsed.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
	collapsed
}

fn suite_group_key(addon: &InstalledAddon) -> String {
	if let Some(id) = addon.curseforge_id {
		return format!("cf:{id}");
	}
	if addon.related_folders.len() > 1 {
		let mut folders = addon.related_folders.clone();
		folders.sort();
		return format!("rel:{}", folders.join("|"));
	}
	if let (Some(arch), Some(at)) = (&addon.archive_name, &addon.installed_at) {
		return format!("zip:{arch}|{at}");
	}
	format!("solo:{}", addon.folder_name)
}

fn sanitize_folder(slug: &str) -> String {
	slug.chars()
		.map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
		.collect()
}

fn alnum_only(value: &str) -> String {
	value.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

/// Persist is_primary / related_folders for a suite after install or collapse repair.
pub fn sync_suite_flags_to_manifest(addons_path: &Path, suites: &[InstalledAddon]) -> Result<(), String> {
	let mut stored = manifest::load(addons_path);
	let mut dirty = false;
	let known: HashSet<String> = stored
		.addons
		.iter()
		.map(|a| a.folder_name.clone())
		.collect();

	for suite in suites {
		if suite.related_folders.len() <= 1 {
			continue;
		}
		for folder in &suite.related_folders {
			if !known.contains(folder) {
				continue;
			}
			if let Some(entry) = stored
				.addons
				.iter_mut()
				.find(|a| a.folder_name == *folder)
			{
				let want_primary = folder == &suite.folder_name;
				if entry.related_folders != suite.related_folders || entry.is_primary != want_primary
				{
					entry.related_folders = suite.related_folders.clone();
					entry.is_primary = want_primary;
					dirty = true;
				}
			}
		}
	}

	if dirty {
		manifest::save(addons_path, &stored)?;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn picks_prefix_parent_for_ellesmere_style() {
		let folders = vec![
			"EllesmereUI_ActionBars".into(),
			"EllesmereUI".into(),
			"EllesmereUI_UnitFrames".into(),
		];
		assert_eq!(
			pick_primary_folder(&folders, Some("ellesmere-ui"), Some("EllesmereUI")),
			"EllesmereUI"
		);
	}

	#[test]
	fn collapse_hides_child_folders() {
		let rows = vec![
			InstalledAddon {
				folder_name: "EllesmereUI".into(),
				title: "EllesmereUI".into(),
				author: None,
				notes: None,
				interface_version: None,
				version: None,
				curseforge_id: Some(99),
				curseforge_slug: Some("ellesmere-ui".into()),
				file_id: Some(1),
				thumbnail_url: None,
				categories: vec![],
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
				is_primary: true,
				archive_name: None,
			},
			InstalledAddon {
				folder_name: "EllesmereUI_Foo".into(),
				title: "EllesmereUI Foo".into(),
				author: None,
				notes: None,
				interface_version: None,
				version: None,
				curseforge_id: Some(99),
				curseforge_slug: Some("ellesmere-ui".into()),
				file_id: Some(1),
				thumbnail_url: None,
				categories: vec![],
				installed_at: None,
				updated_at: None,
				download_count: None,
				latest_game_version: None,
				flavors: vec![],
				has_update: true,
				remote_version: Some("x".into()),
				remote_file_id: Some(2),
				website_url: None,
				related_folders: vec![],
				is_primary: false,
				archive_name: None,
			},
		];
		let collapsed = collapse_suite_rows(rows);
		assert_eq!(collapsed.len(), 1);
		assert_eq!(collapsed[0].folder_name, "EllesmereUI");
		assert_eq!(collapsed[0].related_folders.len(), 2);
		assert!(collapsed[0].has_update);
	}
}
