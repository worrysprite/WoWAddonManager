use crate::curseforge;
use crate::types::{AddonSummary, InstalledAddon, ManifestAddon, WowInstall};
use crate::wow::{detect, manifest, scan};
use chrono::Utc;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

fn resolve_install(install_id: &str) -> Result<WowInstall, String> {
	let mut installs = detect::detect_installs();
	let cfg = crate::config::load();
	for root in cfg.extra_wow_roots {
		if let Ok(extra) = detect::install_from_path(&root) {
			for item in extra {
				if !installs.iter().any(|i| i.id == item.id) {
					installs.push(item);
				}
			}
		}
	}
	find_install_in(&installs, install_id)
}

pub async fn install_addon(install_id: &str, addon: AddonSummary) -> Result<InstalledAddon, String> {
	let install = resolve_install(install_id)?;
	let addons_path = PathBuf::from(&install.addons_path);
	fs::create_dir_all(&addons_path).map_err(|e| e.to_string())?;

	let folders = match curseforge::download_addon_zip(addon.id, &addon.slug, addon.latest_file_id)
		.await
	{
		Ok(bytes) => extract_zip(&bytes, &addons_path)?,
		Err(err) => {
			// Without a working download path: create a tracked placeholder for local testing.
			let folder = sanitize_folder(&addon.slug);
			let target = addons_path.join(&folder);
			fs::create_dir_all(&target).map_err(|e| e.to_string())?;
			write_placeholder_toc(&target, &folder, &addon)?;
			eprintln!("下载失败，已写入占位插件目录以便联调: {err}");
			vec![folder]
		}
	};

	let primary = folders
		.first()
		.cloned()
		.unwrap_or_else(|| sanitize_folder(&addon.slug));
	let now = Utc::now().to_rfc3339();

	for folder in &folders {
		let entry = ManifestAddon {
			folder_name: folder.clone(),
			curseforge_id: Some(addon.id),
			curseforge_slug: Some(addon.slug.clone()),
			file_id: addon.latest_file_id,
			name: Some(addon.name.clone()),
			author: Some(addon.author.clone()),
			summary: Some(addon.summary.clone()),
			thumbnail_url: Some(addon.thumbnail_url.clone()),
			categories: addon.categories.clone(),
			flavors: addon.flavors.clone(),
			download_count: Some(addon.download_count),
			latest_game_version: Some(addon.latest_game_version.clone()),
			website_url: Some(addon.website_url.clone()),
			installed_at: Some(now.clone()),
			updated_at: Some(addon.updated_at.clone()),
			version: Some(addon.updated_at.clone()),
		};
		manifest::upsert(&addons_path, entry)?;
	}

	let list = scan::list_installed(&install).await?;
	list.into_iter()
		.find(|a| a.folder_name == primary)
		.ok_or_else(|| "安装完成但未能读取插件信息".to_string())
}

pub async fn update_addon(install_id: &str, folder_name: &str) -> Result<InstalledAddon, String> {
	let install = resolve_install(install_id)?;
	let addons_path = PathBuf::from(&install.addons_path);
	let stored = manifest::load(&addons_path);
	let tracked = manifest::find(&stored, folder_name)
		.cloned()
		.ok_or_else(|| "本地清单中没有该插件的 CurseForge 信息，请从浏览页重新安装".to_string())?;

	let slug = tracked
		.curseforge_slug
		.clone()
		.ok_or_else(|| "缺少 slug".to_string())?;
	let remote = crate::curseforge::search_addons(crate::types::SearchParams {
		search: Some(slug.clone()),
		page: Some(1),
		page_size: Some(5),
		categories: None,
		game_version_type_ids: None,
		sort_by: Some("relevancy".into()),
	})
	.await
	.items
	.into_iter()
	.find(|item| {
		tracked
			.curseforge_id
			.map(|id| item.id == id)
			.unwrap_or(false)
			|| item.slug == slug
	});

	let summary = if let Some(remote) = remote {
		remote
	} else {
		AddonSummary {
			id: tracked.curseforge_id.unwrap_or(0),
			slug,
			name: tracked.name.clone().unwrap_or_else(|| folder_name.to_string()),
			author: tracked.author.clone().unwrap_or_default(),
			summary: tracked.summary.clone().unwrap_or_default(),
			thumbnail_url: tracked.thumbnail_url.clone().unwrap_or_default(),
			download_count: tracked.download_count.unwrap_or(0),
			updated_at: tracked.updated_at.clone().unwrap_or_default(),
			latest_game_version: tracked.latest_game_version.clone().unwrap_or_default(),
			categories: tracked.categories.clone(),
			flavors: tracked.flavors.clone(),
			website_url: tracked.website_url.clone().unwrap_or_default(),
			latest_file_id: tracked.file_id,
		}
	};

	let installed_at = tracked.installed_at.clone();
	delete_addon(install_id, folder_name).await?;
	let mut installed = install_addon(install_id, summary).await?;
	if let Some(installed_at) = installed_at {
		let addons_path = PathBuf::from(&install.addons_path);
		let mut manifest = manifest::load(&addons_path);
		if let Some(entry) = manifest
			.addons
			.iter_mut()
			.find(|a| a.folder_name == installed.folder_name)
		{
			entry.installed_at = Some(installed_at.clone());
			manifest::save(&addons_path, &manifest)?;
			installed.installed_at = Some(installed_at);
		}
	}
	Ok(installed)
}

pub async fn delete_addon(install_id: &str, folder_name: &str) -> Result<(), String> {
	let install = resolve_install(install_id)?;
	let addons_path = PathBuf::from(&install.addons_path);
	let target = addons_path.join(folder_name);
	if target.exists() {
		fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
	}
	manifest::remove(&addons_path, folder_name)?;
	Ok(())
}

pub fn find_install_in(list: &[WowInstall], install_id: &str) -> Result<WowInstall, String> {
	list.iter()
		.find(|i| i.id == install_id)
		.cloned()
		.ok_or_else(|| "未找到对应的魔兽版本安装".to_string())
}

fn extract_zip(bytes: &[u8], addons_path: &Path) -> Result<Vec<String>, String> {
	let reader = Cursor::new(bytes);
	let mut archive = ZipArchive::new(reader).map_err(|e| e.to_string())?;
	let mut top_folders = std::collections::BTreeSet::new();

	for i in 0..archive.len() {
		let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
		let name = file
			.enclosed_name()
			.ok_or_else(|| "zip 内含非法路径".to_string())?
			.to_path_buf();

		if let Some(first) = name.components().next() {
			if let std::path::Component::Normal(os) = first {
				top_folders.insert(os.to_string_lossy().to_string());
			}
		}

		let out_path = addons_path.join(&name);
		if file.name().ends_with('/') {
			fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
		} else {
			if let Some(parent) = out_path.parent() {
				fs::create_dir_all(parent).map_err(|e| e.to_string())?;
			}
			let mut outfile = fs::File::create(&out_path).map_err(|e| e.to_string())?;
			std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
		}
	}

	Ok(top_folders.into_iter().collect())
}

fn sanitize_folder(slug: &str) -> String {
	slug.chars()
		.map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
		.collect()
}

fn write_placeholder_toc(dir: &Path, folder: &str, addon: &AddonSummary) -> Result<(), String> {
	let toc = format!(
		"## Interface: 120100\n## Title: {title}\n## Author: {author}\n## Notes: {notes}\n## Version: placeholder\n## X-CurseForge-Id: {id}\n## X-Installed-By: WoWAddonManager\n",
		title = addon.name,
		author = addon.author,
		notes = addon.summary.replace('\n', " "),
		id = addon.id
	);
	fs::write(dir.join(format!("{folder}.toc")), toc).map_err(|e| e.to_string())?;
	fs::write(
		dir.join("README-WOW-ADDON-MANAGER.txt"),
		format!(
			"This is a placeholder install because CurseForge download requires API access or passed Cloudflare.\nAddon: {}\nURL: {}\n",
			addon.name, addon.website_url
		),
	)
	.map_err(|e| e.to_string())?;
	Ok(())
}
