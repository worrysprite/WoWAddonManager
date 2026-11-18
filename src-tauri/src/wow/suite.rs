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
///
/// Groups are merged by union-find across:
/// - identical `folder_name` (true duplicates / stale cache parents)
/// - shared CurseForge id
/// - overlapping `related_folders` (suite children whose TOC cleared the id)
/// - same manual zip archive + install timestamp
///
/// Prefer CurseForge id for the displayed primary; fall back to name/folder
/// via [`pick_primary_folder`]. Child folders never become separate parents.
pub fn collapse_suite_rows(addons: Vec<InstalledAddon>) -> Vec<InstalledAddon> {
	if addons.is_empty() {
		return addons;
	}
	if addons.len() == 1 {
		let mut only = addons;
		if only[0].related_folders.is_empty() {
			only[0].related_folders = vec![only[0].folder_name.clone()];
		}
		return only;
	}

	let n = addons.len();
	let mut parent: Vec<usize> = (0..n).collect();

	fn find(parent: &mut [usize], mut i: usize) -> usize {
		while parent[i] != i {
			parent[i] = parent[parent[i]];
			i = parent[i];
		}
		i
	}
	fn union(parent: &mut [usize], a: usize, b: usize) {
		let ra = find(parent, a);
		let rb = find(parent, b);
		if ra != rb {
			parent[rb] = ra;
		}
	}

	let mut by_folder: HashMap<&str, Vec<usize>> = HashMap::new();
	for (i, addon) in addons.iter().enumerate() {
		by_folder
			.entry(addon.folder_name.as_str())
			.or_default()
			.push(i);
	}
	for idxs in by_folder.values() {
		for window in idxs.windows(2) {
			union(&mut parent, window[0], window[1]);
		}
	}

	let mut by_id: HashMap<i64, Vec<usize>> = HashMap::new();
	for (i, addon) in addons.iter().enumerate() {
		if let Some(id) = addon.curseforge_id {
			by_id.entry(id).or_default().push(i);
		}
	}
	for idxs in by_id.values() {
		for window in idxs.windows(2) {
			union(&mut parent, window[0], window[1]);
		}
	}

	// Suite children often lose CurseForge id after TOC-prefer clears a missing
	// X-Curse-Project-ID. Overlapping related_folders still mark the same package.
	// Do not merge into a folder that already has a *different* CurseForge id
	// (e.g. Questie must not swallow QuestieDB).
	for (i, addon) in addons.iter().enumerate() {
		if addon.related_folders.len() <= 1 {
			continue;
		}
		for folder in &addon.related_folders {
			if let Some(idxs) = by_folder.get(folder.as_str()) {
				for &j in idxs {
					if i == j {
						continue;
					}
					let other = &addons[j];
					let compatible = match (addon.curseforge_id, other.curseforge_id) {
						(Some(a), Some(b)) => a == b,
						_ => true,
					};
					if compatible {
						union(&mut parent, i, j);
					}
				}
			}
		}
	}

	let mut by_zip: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
	for (i, addon) in addons.iter().enumerate() {
		if let (Some(arch), Some(at)) = (&addon.archive_name, &addon.installed_at) {
			by_zip.entry((arch.as_str(), at.as_str())).or_default().push(i);
		}
	}
	for idxs in by_zip.values() {
		for window in idxs.windows(2) {
			union(&mut parent, window[0], window[1]);
		}
	}

	let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
	let mut order: Vec<usize> = Vec::new();
	for i in 0..n {
		let root = find(&mut parent, i);
		if !components.contains_key(&root) {
			order.push(root);
		}
		components.entry(root).or_default().push(i);
	}

	let listed_folders: HashSet<&str> = addons.iter().map(|a| a.folder_name.as_str()).collect();

	let mut collapsed = Vec::new();
	for root in order {
		let Some(idxs) = components.remove(&root) else {
			continue;
		};
		let members: Vec<&InstalledAddon> = idxs.iter().map(|&i| &addons[i]).collect();
		let member_names: HashSet<&str> =
			members.iter().map(|m| m.folder_name.as_str()).collect();
		collapsed.push(collapse_component(&members, &member_names, &listed_folders));
	}

	collapsed.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
	collapsed
}

fn collapse_component(
	members: &[&InstalledAddon],
	member_names: &HashSet<&str>,
	listed_folders: &HashSet<&str>,
) -> InstalledAddon {
	debug_assert!(!members.is_empty());
	if members.len() == 1 {
		let mut only = members[0].clone();
		if only.related_folders.is_empty() {
			only.related_folders = vec![only.folder_name.clone()];
		}
		// Keep existing related_folders: cache often stores only the suite parent
		// row after a prior collapse; wiping it would hide "含 N 个插件目录".
		return only;
	}

	let mut member_folders: Vec<String> =
		members.iter().map(|m| m.folder_name.clone()).collect();
	member_folders.sort();
	member_folders.dedup();

	// Related list = member folders plus suite claims that are not a different
	// listed addon outside this component (stale duplicate parents still show
	// the full suite size when the child row was already collapsed away).
	let mut related = member_folders.clone();
	for m in members {
		for folder in &m.related_folders {
			if member_names.contains(folder.as_str())
				|| !listed_folders.contains(folder.as_str())
			{
				if !related.iter().any(|f| f == folder) {
					related.push(folder.clone());
				}
			}
		}
	}
	related.sort();
	related.dedup();

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
		.unwrap_or_else(|| {
			pick_primary_folder(&member_folders, slug.as_deref(), name.as_deref())
		});

	let mut primary = members
		.iter()
		.find(|m| m.folder_name == primary_name)
		.cloned()
		.cloned()
		.unwrap_or_else(|| members[0].clone());

	// Prefer any known CurseForge id/slug from the suite (parent TOC or sibling).
	if primary.curseforge_id.is_none() {
		primary.curseforge_id = members.iter().find_map(|m| m.curseforge_id);
	}
	if primary.curseforge_slug.is_none() {
		primary.curseforge_slug = members.iter().find_map(|m| m.curseforge_slug.clone());
	}
	if primary.thumbnail_url.as_ref().map(|u| u.is_empty()).unwrap_or(true) {
		primary.thumbnail_url = members.iter().find_map(|m| m.thumbnail_url.clone());
	}
	if primary.website_url.as_ref().map(|u| u.is_empty()).unwrap_or(true) {
		primary.website_url = members.iter().find_map(|m| m.website_url.clone());
	}

	primary.is_primary = true;
	primary.related_folders = related;
	primary.has_update = members.iter().any(|m| m.has_update);
	if primary.remote_file_id.is_none() {
		primary.remote_file_id = members.iter().find_map(|m| m.remote_file_id);
	}
	if primary.remote_version.is_none() {
		primary.remote_version = members.iter().find_map(|m| m.remote_version.clone());
	}
	primary
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

	fn sample(
		folder: &str,
		title: &str,
		id: Option<i64>,
		related: Vec<&str>,
		is_primary: bool,
		notes: Option<&str>,
	) -> InstalledAddon {
		InstalledAddon {
			folder_name: folder.into(),
			title: title.into(),
			author: Some("Questie Team".into()),
			notes: notes.map(|n| n.into()),
			interface_version: None,
			version: None,
			curseforge_id: id,
			curseforge_slug: id.map(|_| "questie".into()),
			file_id: None,
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
			related_folders: related.into_iter().map(|s| s.to_string()).collect(),
			is_primary,
			archive_name: None,
		}
	}

	#[test]
	fn collapse_hides_child_folders() {
		let rows = vec![
			sample(
				"EllesmereUI",
				"EllesmereUI",
				Some(99),
				vec![],
				true,
				None,
			),
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

	/// Rescan path: parent keeps TOC CurseForge id; child TOC omits id so
	/// prefer_toc clears it, but related_folders still overlap → one parent.
	#[test]
	fn collapse_merges_suite_when_child_lost_curseforge_id() {
		let rows = vec![
			sample(
				"Questie",
				"Questie",
				Some(3358),
				vec!["Questie", "QuestieCompat"],
				true,
				Some("A standalone Classic QuestHelper"),
			),
			sample(
				"QuestieCompat",
				"Questie",
				None,
				vec!["Questie", "QuestieCompat"],
				false,
				Some("Compatibility layer"),
			),
			sample(
				"QuestieDB",
				"QuestieDB",
				Some(9999),
				vec!["QuestieDB"],
				true,
				Some("Database"),
			),
		];
		let collapsed = collapse_suite_rows(rows);
		assert_eq!(collapsed.len(), 2, "Questie suite + QuestieDB only");
		let questie = collapsed
			.iter()
			.find(|a| a.folder_name == "Questie")
			.expect("Questie parent");
		assert_eq!(questie.related_folders.len(), 2);
		assert_eq!(questie.curseforge_id, Some(3358));
		assert!(collapsed.iter().all(|a| a.folder_name != "QuestieCompat"));
		assert!(collapsed.iter().any(|a| a.folder_name == "QuestieDB"));
	}

	/// Stale cache can store two already-collapsed parents for the same suite.
	#[test]
	fn collapse_dedupes_duplicate_suite_parents() {
		let rows = vec![
			sample(
				"Questie",
				"Questie",
				Some(3358),
				vec!["Questie", "QuestieCompat"],
				true,
				Some("notes A"),
			),
			sample(
				"Questie",
				"Questie",
				None,
				vec!["Questie", "QuestieCompat"],
				true,
				Some("notes B"),
			),
		];
		let collapsed = collapse_suite_rows(rows);
		assert_eq!(collapsed.len(), 1);
		assert_eq!(collapsed[0].folder_name, "Questie");
		assert_eq!(collapsed[0].related_folders.len(), 2);
		assert_eq!(collapsed[0].curseforge_id, Some(3358));
	}

	#[test]
	fn collapse_does_not_merge_distinct_curseforge_ids_via_related() {
		let rows = vec![
			sample(
				"Questie",
				"Questie",
				Some(3358),
				vec!["Questie", "QuestieDB"],
				true,
				None,
			),
			sample(
				"QuestieDB",
				"QuestieDB",
				Some(9999),
				vec!["QuestieDB"],
				true,
				None,
			),
		];
		let collapsed = collapse_suite_rows(rows);
		assert_eq!(collapsed.len(), 2);
	}
}
