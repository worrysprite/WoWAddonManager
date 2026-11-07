use crate::curseforge;
use crate::types::{AddonSummary, InstalledAddon, ManifestAddon, WowInstall};
use crate::wow::{archive, detect, manifest, scan};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

fn resolve_install(install_id: &str) -> Result<WowInstall, String> {
	let cfg = crate::config::load();
	let report = detect::detect_with_extra_roots(&cfg.extra_wow_roots);
	find_install_in(&report.installs, install_id)
}

pub async fn install_addon(install_id: &str, addon: AddonSummary) -> Result<InstalledAddon, String> {
	let install = resolve_install(install_id)?;
	let addons_path = PathBuf::from(&install.addons_path);
	fs::create_dir_all(&addons_path).map_err(|e| e.to_string())?;

	let folders = match curseforge::download_addon_zip(addon.id, &addon.slug, addon.latest_file_id)
		.await
	{
		Ok(bytes) => archive::extract_zip_bytes(&bytes, &addons_path)?,
		Err(err) => {
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
			install_source: Some("curseforge".into()),
			archive_name: None,
		};
		manifest::upsert(&addons_path, entry)?;
	}

	let list = scan::refresh_local_cache(&install)?;
	list.into_iter()
		.find(|a| a.folder_name == primary)
		.ok_or_else(|| "安装完成但未能读取插件信息".to_string())
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

	let folders = archive::extract_archive(&archive, &addons_path)?;
	let now = Utc::now().to_rfc3339();
	let archive_name = archive
		.file_name()
		.and_then(|s| s.to_str())
		.unwrap_or("archive")
		.to_string();

	for folder in &folders {
		let (title, author, notes, version) = archive::read_toc_meta(&addons_path, folder);
		let entry = ManifestAddon {
			folder_name: folder.clone(),
			curseforge_id: None,
			curseforge_slug: None,
			file_id: None,
			name: title.or_else(|| Some(folder.clone())),
			author,
			summary: notes.or_else(|| Some(format!("手动安装自 {archive_name}"))),
			thumbnail_url: None,
			categories: vec!["Manual".into()],
			flavors: Vec::new(),
			download_count: None,
			latest_game_version: None,
			website_url: None,
			installed_at: Some(now.clone()),
			updated_at: Some(now.clone()),
			version,
			install_source: Some("manual".into()),
			archive_name: Some(archive_name.clone()),
		};
		manifest::upsert(&addons_path, entry)?;
	}

	let list = scan::refresh_local_cache(&install)?;
	let installed = list
		.into_iter()
		.filter(|a| folders.iter().any(|f| f == &a.folder_name))
		.collect::<Vec<_>>();
	if installed.is_empty() {
		Err("安装完成但未能读取插件信息".into())
	} else {
		Ok(installed)
	}
}

pub async fn update_addon(install_id: &str, folder_name: &str) -> Result<InstalledAddon, String> {
	let install = resolve_install(install_id)?;
	let addons_path = PathBuf::from(&install.addons_path);
	let stored = manifest::load(&addons_path);
	let tracked = manifest::find(&stored, folder_name)
		.cloned()
		.ok_or_else(|| "本地清单中没有该插件的 CurseForge 信息，请从浏览页重新安装".to_string())?;

	if tracked.install_source.as_deref() == Some("manual") || tracked.curseforge_slug.is_none() {
		return Err("手动安装的插件不支持在线更新，请重新选择压缩包安装".into());
	}

	let slug = tracked
		.curseforge_slug
		.clone()
		.ok_or_else(|| "缺少 slug".to_string())?;
	let remote = crate::curseforge::search_addons(
		crate::types::SearchParams {
			search: Some(slug.clone()),
			page: Some(1),
			page_size: Some(5),
			categories: None,
			game_version_type_ids: None,
			sort_by: Some("relevancy".into()),
		},
		false,
	)
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
		let _ = scan::refresh_local_cache(&install);
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
	scan::remove_from_cache(&install, folder_name)?;
	Ok(())
}

pub fn find_install_in(list: &[WowInstall], install_id: &str) -> Result<WowInstall, String> {
	list.iter()
		.find(|i| i.id == install_id)
		.cloned()
		.ok_or_else(|| "未找到对应的魔兽版本安装".to_string())
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
