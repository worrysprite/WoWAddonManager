use crate::curseforge;
use crate::types::{
	AddonJobProgress, AddonSummary, InstalledAddon, ManifestAddon, SearchParams, WowInstall,
};
use crate::wow::{archive, detect, manifest, scan, toc};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

type ProgressCb = Arc<dyn Fn(AddonJobProgress) + Send + Sync>;

fn emit_progress(on_progress: Option<&ProgressCb>, progress: AddonJobProgress) {
	if let Some(cb) = on_progress {
		cb(progress);
	}
}

fn resolve_install(install_id: &str) -> Result<WowInstall, String> {
	let cfg = crate::config::load();
	let report = detect::detect_with_extra_roots(&cfg.extra_wow_roots);
	find_install_in(&report.installs, install_id)
}

pub async fn install_addon(install_id: &str, addon: AddonSummary) -> Result<InstalledAddon, String> {
	install_addon_with_progress(install_id, addon, None).await
}

pub async fn install_addon_with_progress(
	install_id: &str,
	addon: AddonSummary,
	on_progress: Option<ProgressCb>,
) -> Result<InstalledAddon, String> {
	let install = resolve_install(install_id)?;
	let addons_path = PathBuf::from(&install.addons_path);
	fs::create_dir_all(&addons_path).map_err(|e| e.to_string())?;

	let folders = match curseforge::download_addon_zip(
		addon.id,
		&addon.slug,
		addon.latest_file_id,
		install.game_version_type_id,
		on_progress.as_ref().map(|cb| cb.as_ref() as _),
	)
	.await
	{
		Ok(bytes) => {
			emit_progress(on_progress.as_ref(), AddonJobProgress::extract());
			archive::extract_zip_bytes(&bytes, &addons_path)?
		}
		Err(err) => {
			// Do not write a Retail placeholder TOC — on Forever that only
			// masks the real download/flavor error as "不兼容".
			return Err(format!("下载失败：{err}"));
		}
	};

	emit_progress(on_progress.as_ref(), AddonJobProgress::install());

	let toc_ctx = crate::wow::toc::ClientContext::from_install(
		&install.flavor_key,
		install.game_version.as_deref(),
	);
	let mut accepted = Vec::new();
	let mut reject_reason: Option<String> = None;

	for folder in &folders {
		if let Some(issue) =
			crate::wow::toc::folder_unsupported_reason(&addons_path, folder, &toc_ctx)
		{
			reject_reason = Some(issue.reason);
			log::warn!(
				"拒绝安装不兼容插件 folder={folder} addon={}",
				addon.name
			);
			continue;
		}
		accepted.push(folder.clone());
	}

	// CurseForge packages are one unit: any incompatible folder fails the whole install.
	if reject_reason.is_some() || accepted.is_empty() {
		for folder in &folders {
			let target = addons_path.join(folder);
			let _ = fs::remove_dir_all(&target);
			let _ = manifest::remove(&addons_path, folder);
		}
		let _ = scan::refresh_local_cache(&install);
		return Err(format!(
			"安装失败：{} 与当前客户端不兼容。{}",
			addon.name,
			reject_reason.unwrap_or_else(|| "TOC 标记为 game client not supported".into())
		));
	}

	let primary = crate::wow::suite::pick_primary_folder(
		&accepted,
		Some(&addon.slug),
		Some(&addon.name),
	);
	let now = Utc::now().to_rfc3339();
	let thumbnail_url = crate::icon_cache::ensure_cached(Some(addon.id), &addon.thumbnail_url)
		.await
		.or_else(|| {
			let trimmed = addon.thumbnail_url.trim();
			(!trimmed.is_empty()).then(|| trimmed.to_string())
		});

	for folder in &accepted {
		let entry = ManifestAddon {
			folder_name: folder.clone(),
			curseforge_id: Some(addon.id),
			curseforge_slug: Some(addon.slug.clone()),
			file_id: addon.latest_file_id,
			name: Some(addon.name.clone()),
			author: Some(addon.author.clone()),
			summary: Some(addon.summary.clone()),
			thumbnail_url: thumbnail_url.clone(),
			categories: addon.categories.clone(),
			flavors: addon.flavors.clone(),
			download_count: Some(addon.download_count),
			latest_game_version: Some(addon.latest_game_version.clone()),
			website_url: Some(addon.website_url.clone()),
			installed_at: Some(now.clone()),
			updated_at: Some(addon.updated_at.clone()),
			// Human file label only — never fall back to updatedAt (ISO timestamps).
			version: addon
				.latest_version
				.clone()
				.filter(|v| !v.trim().is_empty()),
			install_source: Some("curseforge".into()),
			archive_name: None,
			related_folders: accepted.clone(),
			is_primary: folder == &primary,
		};
		manifest::upsert(&addons_path, entry)?;
	}

	let list = scan::refresh_local_cache(&install)?;
	list.into_iter()
		.find(|a| a.folder_name == primary)
		.ok_or_else(|| "安装完成但未能读取插件信息".to_string())
}

fn promote_addon_folder(src: &Path, dest: &Path) -> Result<(), String> {
	if dest.exists() {
		fs::remove_dir_all(dest).map_err(|e| format!("无法覆盖已有插件目录 {}: {e}", dest.display()))?;
	}
	if let Some(parent) = dest.parent() {
		fs::create_dir_all(parent).map_err(|e| e.to_string())?;
	}
	if fs::rename(src, dest).is_err() {
		archive::copy_dir_recursive(src, dest)?;
		let _ = fs::remove_dir_all(src);
	}
	Ok(())
}

pub async fn install_from_archive(
	install_id: &str,
	archive_path: &str,
) -> Result<Vec<InstalledAddon>, String> {
	let install = resolve_install(install_id)?;
	let addons_path = PathBuf::from(&install.addons_path);
	fs::create_dir_all(&addons_path).map_err(|e| e.to_string())?;

	let archive = PathBuf::from(archive_path);
	if !archive.exists() {
		return Err("压缩包不存在".into());
	}

	// Extract into an isolated staging dir first so rejected folders never
	// overwrite / delete an already-installed addon of the same name.
	let staging = addons_path.join(format!(
		".wam-manual-{}",
		Utc::now().timestamp_millis()
	));
	if let Err(err) = fs::create_dir_all(&staging) {
		return Err(err.to_string());
	}
	let folders = match archive::extract_archive(&archive, &staging) {
		Ok(folders) => folders,
		Err(err) => {
			let _ = fs::remove_dir_all(&staging);
			return Err(err);
		}
	};
	let now = Utc::now().to_rfc3339();
	let archive_name = archive
		.file_name()
		.and_then(|s| s.to_str())
		.unwrap_or("archive")
		.to_string();

	let toc_ctx = crate::wow::toc::ClientContext::from_install(
		&install.flavor_key,
		install.game_version.as_deref(),
	);
	let mut pending = Vec::new();
	let mut rejected = Vec::new();

	for folder in &folders {
		if let Some(issue) =
			crate::wow::toc::folder_unsupported_reason(&staging, folder, &toc_ctx)
		{
			let title = crate::wow::toc::read_folder_toc_for_client(&staging, folder, &toc_ctx)
				.and_then(|m| m.title)
				.unwrap_or_else(|| folder.clone());
			rejected.push(format!("{folder}（{title}）: {}", issue.reason));
			log::warn!(
				"拒绝安装不兼容插件 folder={folder} reason={}",
				issue.reason
			);
			continue;
		}
		let meta = crate::wow::toc::read_folder_toc_for_client(&staging, folder, &toc_ctx);
		pending.push((folder.clone(), meta));
	}

	// UI suites / multi-addon zips: keep compatible folders only.
	if pending.is_empty() {
		let _ = fs::remove_dir_all(&staging);
		let _ = scan::refresh_local_cache(&install);
		return Err(format!(
			"安装失败：压缩包内插件与当前客户端不兼容。{}",
			if rejected.is_empty() {
				String::new()
			} else {
				format!("详情：{}", rejected.join("；"))
			}
		));
	}

	let mut accepted = Vec::new();
	let mut pending_meta = Vec::new();
	for (folder, meta) in pending {
		let src = staging.join(&folder);
		let dest = addons_path.join(&folder);
		if let Err(err) = promote_addon_folder(&src, &dest) {
			let _ = fs::remove_dir_all(&staging);
			return Err(err);
		}
		pending_meta.push((folder.clone(), meta));
		accepted.push(folder);
	}

	let archive_stem = PathBuf::from(&archive_name)
		.file_stem()
		.and_then(|s| s.to_str())
		.map(|s| s.to_string());
	let primary = crate::wow::suite::pick_primary_folder(
		&accepted,
		archive_stem.as_deref(),
		archive_stem.as_deref(),
	);

	for (folder, meta) in pending_meta {
		let title = meta.as_ref().and_then(|m| m.title.clone());
		let author = meta.as_ref().and_then(|m| m.author.clone());
		let notes = meta.as_ref().and_then(|m| m.notes.clone());
		let version = meta.as_ref().and_then(|m| m.version.clone());
		let entry = ManifestAddon {
			folder_name: folder.clone(),
			curseforge_id: None,
			curseforge_slug: None,
			file_id: None,
			name: title.or_else(|| Some(folder.clone())),
			author,
			summary: notes.or_else(|| Some(format!("手动安装自 {archive_name}"))),
			thumbnail_url: None,
			categories: Vec::new(),
			flavors: Vec::new(),
			download_count: None,
			latest_game_version: None,
			website_url: None,
			installed_at: Some(now.clone()),
			updated_at: Some(now.clone()),
			version,
			install_source: Some("manual".into()),
			archive_name: Some(archive_name.clone()),
			related_folders: accepted.clone(),
			is_primary: folder == primary,
		};
		if let Err(err) = manifest::upsert(&addons_path, entry) {
			let _ = fs::remove_dir_all(&staging);
			return Err(err);
		}
	}

	let _ = fs::remove_dir_all(&staging);

	if !rejected.is_empty() {
		log::warn!(
			"合集部分插件与当前客户端不兼容已跳过：{}",
			rejected.join("；")
		);
	}

	let list = scan::refresh_local_cache(&install)?;
	let installed = list
		.into_iter()
		.filter(|a| a.folder_name == primary || accepted.iter().any(|f| f == &a.folder_name))
		.collect::<Vec<_>>();

	Ok(installed)
}

fn remote_matches_local(
	item: &AddonSummary,
	folder_name: &str,
	tracked: Option<&ManifestAddon>,
	title: Option<&str>,
	preferred_id: Option<i64>,
) -> bool {
	if let Some(id) = preferred_id {
		return item.id == id;
	}
	// Name matching (no TOC project id): folder / title / tracked display name only.
	// Do not use stale curseforge_id / slug binds — those caused Forever PostBox → Retail PostBox.
	if let Some(tracked) = tracked {
		if let Some(name) = tracked.name.as_deref() {
			if crate::wow::match_remote::names_loosely_equal(item.name.as_str(), name) {
				return true;
			}
		}
	}
	crate::wow::match_remote::names_loosely_equal(item.slug.as_str(), folder_name)
		|| crate::wow::match_remote::names_loosely_equal(item.name.as_str(), folder_name)
		|| title
			.map(|t| crate::wow::match_remote::names_loosely_equal(item.name.as_str(), t))
			.unwrap_or(false)
}

fn has_installable_file_for_flavor(item: &AddonSummary, preferred_type_id: Option<i64>) -> bool {
	if preferred_type_id.is_none() {
		return true;
	}
	item.latest_file_id.is_some()
}

/// Resolve a CurseForge mod for update.
/// Policy: prefer TOC `X-Curse-Project-ID` when present; otherwise match by name.
/// If the TOC/preferred id has no file for this client flavor, fall through to name search.
async fn find_curseforge_for_update(
	folder_name: &str,
	tracked: Option<&ManifestAddon>,
	title: Option<&str>,
	preferred_type_id: Option<i64>,
	toc_project_id: Option<i64>,
) -> Option<AddonSummary> {
	if let Some(id) = toc_project_id {
		if let Some(item) = curseforge::get_addon_for_flavor(id, preferred_type_id).await {
			if has_installable_file_for_flavor(&item, preferred_type_id) {
				return Some(item);
			}
			log::warn!(
				"TOC CurseForge mod {id} has no file for preferred={preferred_type_id:?}; searching name alternatives"
			);
		}
	}

	// Name matching (no stale curseforge_id / slug binds).
	let mut terms = Vec::new();
	terms.push(folder_name.to_string());
	if let Some(title) = title {
		if !terms.iter().any(|t| t.eq_ignore_ascii_case(title)) {
			terms.push(title.to_string());
		}
	}
	if let Some(name) = tracked.and_then(|t| t.name.clone()) {
		if !terms.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
			terms.push(name);
		}
	}

	for term in terms {
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
		if let Some(item) = result.items.into_iter().find(|item| {
			remote_matches_local(item, folder_name, tracked, title, None)
		}) {
			// Search hits may carry a Retail latestFileId; re-resolve for this client.
			if let Some(resolved) =
				curseforge::get_addon_for_flavor(item.id, preferred_type_id).await
			{
				if has_installable_file_for_flavor(&resolved, preferred_type_id) {
					return Some(resolved);
				}
				continue;
			}
			if has_installable_file_for_flavor(&item, preferred_type_id) {
				return Some(item);
			}
		}
	}
	None
}

pub async fn update_addon_with_progress(
	install_id: &str,
	folder_name: &str,
	on_progress: Option<ProgressCb>,
) -> Result<InstalledAddon, String> {
	let install = resolve_install(install_id)?;
	let addons_path = PathBuf::from(&install.addons_path);
	let stored = manifest::load(&addons_path);
	let tracked = manifest::find(&stored, folder_name).cloned();

	let toc_ctx = toc::ClientContext::from_install(
		&install.flavor_key,
		install.game_version.as_deref(),
	);
	let toc_meta =
		toc::read_folder_toc_for_client(&addons_path, folder_name, &toc_ctx).unwrap_or_default();
	let title = tracked
		.as_ref()
		.and_then(|t| t.name.clone())
		.or(toc_meta.title.clone())
		.or_else(|| Some(folder_name.to_string()));

	emit_progress(
		on_progress.as_ref(),
		AddonJobProgress::download(Some(0), None, None),
	);

	let summary = find_curseforge_for_update(
		folder_name,
		tracked.as_ref(),
		title.as_deref(),
		install.game_version_type_id,
		toc_meta.curseforge_project_id,
	)
	.await
	.ok_or_else(|| {
		let client = install
			.game_version_type_id
			.map(crate::curseforge::catalog::flavor_display_name)
			.unwrap_or_else(|| install.flavor_key.clone());
		format!("未在 CurseForge 找到适用于 {client} 的「{folder_name}」更新包")
	})?;

	let installed_at = tracked.and_then(|t| t.installed_at.clone());
	let suite_folders = crate::wow::suite::suite_folders_for(&addons_path, folder_name);
	let backup_root = addons_path.join(format!(
		".wam-backup-suite-{}-{}",
		folder_name,
		Utc::now().timestamp_millis()
	));
	fs::create_dir_all(&backup_root).map_err(|e| e.to_string())?;

	// Snapshot manifest rows so a failed update can restore metadata too.
	let mut prior_manifest = manifest::load(&addons_path);
	let suite_manifest: Vec<_> = prior_manifest
		.addons
		.iter()
		.filter(|a| suite_folders.iter().any(|f| f == &a.folder_name))
		.cloned()
		.collect();

	// Move aside every suite folder so a failed install can restore the whole package.
	for folder in &suite_folders {
		let target = addons_path.join(folder);
		if target.exists() {
			let dest = backup_root.join(folder);
			fs::rename(&target, &dest).map_err(|e| e.to_string())?;
		}
		let _ = manifest::remove(&addons_path, folder);
	}

	let install_result =
		install_addon_with_progress(install_id, summary, on_progress.clone()).await;
	match install_result {
		Ok(mut installed) => {
			let _ = fs::remove_dir_all(&backup_root);
			if let Some(installed_at) = installed_at {
				let mut manifest = manifest::load(&addons_path);
				for entry in manifest.addons.iter_mut() {
					if installed.related_folders.iter().any(|f| f == &entry.folder_name)
						|| entry.folder_name == installed.folder_name
					{
						entry.installed_at = Some(installed_at.clone());
					}
				}
				manifest::save(&addons_path, &manifest)?;
				installed.installed_at = Some(installed_at);
				let _ = scan::refresh_local_cache(&install);
			}
			Ok(installed)
		}
		Err(err) => {
			// Restore previous suite folders + manifest metadata.
			if let Ok(entries) = fs::read_dir(&backup_root) {
				for entry in entries.flatten() {
					let name = entry.file_name();
					let dest = addons_path.join(&name);
					if dest.exists() {
						let _ = fs::remove_dir_all(&dest);
					}
					let _ = fs::rename(entry.path(), dest);
				}
			}
			let _ = fs::remove_dir_all(&backup_root);
			prior_manifest = manifest::load(&addons_path);
			for entry in suite_manifest {
				if let Some(existing) = prior_manifest
					.addons
					.iter_mut()
					.find(|a| a.folder_name == entry.folder_name)
				{
					*existing = entry;
				} else {
					prior_manifest.addons.push(entry);
				}
			}
			let _ = manifest::save(&addons_path, &prior_manifest);
			let _ = scan::refresh_local_cache(&install);
			Err(err)
		}
	}
}

pub async fn delete_addon(install_id: &str, folder_name: &str) -> Result<(), String> {
	let install = resolve_install(install_id)?;
	let addons_path = PathBuf::from(&install.addons_path);
	let suite_folders = crate::wow::suite::suite_folders_for(&addons_path, folder_name);
	for folder in &suite_folders {
		let target = addons_path.join(folder);
		if target.exists() {
			fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
		}
		manifest::remove(&addons_path, folder)?;
		scan::remove_from_cache(&install, folder)?;
	}
	Ok(())
}

pub fn find_install_in(list: &[WowInstall], install_id: &str) -> Result<WowInstall, String> {
	list.iter()
		.find(|i| i.id == install_id)
		.cloned()
		.ok_or_else(|| "未找到对应的魔兽版本安装".to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::{SystemTime, UNIX_EPOCH};

	fn sample_remote(id: i64, slug: &str, name: &str) -> AddonSummary {
		AddonSummary {
			id,
			slug: slug.into(),
			name: name.into(),
			author: "Author".into(),
			summary: String::new(),
			thumbnail_url: String::new(),
			download_count: 0,
			updated_at: String::new(),
			latest_game_version: String::new(),
			categories: vec![],
			flavors: vec![],
			website_url: String::new(),
			latest_file_id: Some(1),
			latest_version: None,
		}
	}

	#[test]
	fn remote_matches_manual_folder_or_title() {
		let item = sample_remote(42, "DeadlyBossMods", "Deadly Boss Mods");
		assert!(remote_matches_local(
			&item,
			"DeadlyBossMods",
			None,
			Some("Deadly Boss Mods"),
			None
		));
		assert!(remote_matches_local(
			&item,
			"SomeOtherFolder",
			None,
			Some("Deadly Boss Mods"),
			None
		));
		assert!(!remote_matches_local(
			&item,
			"Unrelated",
			None,
			Some("Something Else"),
			None
		));
	}

	#[test]
	fn remote_matches_prefers_explicit_project_id() {
		// Forever PostBox TOC id must not match Retail-only PostBox search hit.
		let retail_postbox = sample_remote(1680332, "postbox", "PostBox");
		assert!(!remote_matches_local(
			&retail_postbox,
			"Postbox",
			None,
			Some("PostBox"),
			Some(1639171)
		));
		let forever_postbox = sample_remote(1639171, "postboxmail", "Postbox");
		assert!(remote_matches_local(
			&forever_postbox,
			"Postbox",
			None,
			Some("PostBox"),
			Some(1639171)
		));
	}

	#[test]
	fn remote_matches_name_fallback_ignores_stale_slug_bind() {
		use crate::types::ManifestAddon;
		let tracked = ManifestAddon {
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
		};
		// Without TOC id, only name/folder matching applies — stale id/slug binds
		// must not force a hit when the remote title/folder do not match.
		let unrelated = sample_remote(1, "some-other-slug", "Totally Different Addon");
		assert!(!remote_matches_local(
			&unrelated,
			"Postbox",
			Some(&tracked),
			Some("PostBox"),
			None
		));
		let by_title = sample_remote(1639171, "postboxmail", "PostBox");
		assert!(remote_matches_local(
			&by_title,
			"Postbox",
			Some(&tracked),
			Some("PostBox"),
			None
		));
	}

	#[test]
	fn remote_matches_forever_postbox_decorated_title() {
		let item = sample_remote(
			1639171,
			"postboxmail",
			"Postbox • A Modern Mailbox & Mail Manager",
		);
		assert!(remote_matches_local(
			&item,
			"Postbox",
			None,
			Some("Postbox"),
			None
		));
		// Retail-only Tibiscui PostBox must not match via slug prefix alone.
		let retail_only = sample_remote(1680332, "postbox", "PostBox");
		assert!(remote_matches_local(
			&retail_only,
			"postbox",
			None,
			Some("PostBox"),
			None
		));
		assert!(!crate::wow::match_remote::names_loosely_equal(
			"postboxmail",
			"postbox"
		));
	}

	#[test]
	fn rejected_staging_folder_leaves_live_addon_intact() {
		let stamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let root = std::env::temp_dir().join(format!("wam-manual-stage-{stamp}"));
		let addons = root.join("AddOns");
		let staging = root.join(".wam-manual-stage");
		let _ = fs::remove_dir_all(&root);
		fs::create_dir_all(addons.join("Foo")).unwrap();
		fs::write(addons.join("Foo/Foo.toc"), "## Interface: 16001\n## Title: LiveFoo\n").unwrap();
		fs::create_dir_all(staging.join("Foo")).unwrap();
		fs::write(
			staging.join("Foo/Foo.toc"),
			"## Interface: 11507\n## Title: ZipFoo\n",
		)
		.unwrap();
		fs::create_dir_all(staging.join("Bar")).unwrap();
		fs::write(
			staging.join("Bar/Bar.toc"),
			"## Interface: 16001\n## Title: ZipBar\n",
		)
		.unwrap();

		// Only promote the compatible folder; rejected Foo stays in staging.
		promote_addon_folder(&staging.join("Bar"), &addons.join("Bar")).unwrap();
		let _ = fs::remove_dir_all(&staging);

		assert_eq!(
			fs::read_to_string(addons.join("Foo/Foo.toc")).unwrap(),
			"## Interface: 16001\n## Title: LiveFoo\n"
		);
		assert!(addons.join("Bar/Bar.toc").is_file());
		assert!(!staging.exists());
		let _ = fs::remove_dir_all(root);
	}
}
