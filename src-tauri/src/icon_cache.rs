//! Local thumbnail cache under %AppData%/wow-addon-manager/icons.
//! Keeps JSON metadata small (paths only) while avoiding repeat CDN downloads.

use crate::types::{AddonSummary, InstalledAddon};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

const USER_AGENT: &str = "WoWAddonManager/1.0 (WorrySprite)";

pub fn icons_dir() -> Option<PathBuf> {
	let dir = dirs::config_dir()?.join("wow-addon-manager").join("icons");
	fs::create_dir_all(&dir).ok()?;
	Some(dir)
}

fn is_remote_url(url: &str) -> bool {
	let lower = url.trim().to_ascii_lowercase();
	lower.starts_with("http://") || lower.starts_with("https://")
}

fn is_local_file(url: &str) -> bool {
	let path = Path::new(url);
	path.is_absolute() && path.is_file()
}

fn cache_stem(mod_id: Option<i64>, url: &str) -> String {
	if let Some(id) = mod_id {
		return format!("cf-{id}");
	}
	let mut hasher = DefaultHasher::new();
	url.trim().hash(&mut hasher);
	format!("u-{:x}", hasher.finish())
}

fn extension_from_url_or_content_type(url: &str, content_type: Option<&str>) -> &'static str {
	if let Some(ct) = content_type.map(|s| s.to_ascii_lowercase()) {
		if ct.contains("image/jpeg") || ct.contains("image/jpg") {
			return "jpg";
		}
		if ct.contains("image/webp") {
			return "webp";
		}
		if ct.contains("image/gif") {
			return "gif";
		}
		if ct.contains("image/png") {
			return "png";
		}
	}
	let path = url.split('?').next().unwrap_or(url);
	let ext = Path::new(path)
		.extension()
		.and_then(|e| e.to_str())
		.unwrap_or("")
		.to_ascii_lowercase();
	match ext.as_str() {
		"jpg" | "jpeg" => "jpg",
		"webp" => "webp",
		"gif" => "gif",
		_ => "png",
	}
}

fn existing_cached_file(stem: &str) -> Option<PathBuf> {
	let dir = icons_dir()?;
	for ext in ["png", "jpg", "jpeg", "webp", "gif"] {
		let path = dir.join(format!("{stem}.{ext}"));
		if path.is_file() {
			return Some(path);
		}
	}
	None
}

/// Ensure `url` is available locally. Returns an absolute filesystem path string.
pub async fn ensure_cached(mod_id: Option<i64>, url: &str) -> Option<String> {
	let url = url.trim();
	if url.is_empty() {
		return None;
	}
	if is_local_file(url) {
		return Some(url.to_string());
	}
	if !is_remote_url(url) {
		return None;
	}

	let stem = cache_stem(mod_id, url);
	if let Some(existing) = existing_cached_file(&stem) {
		return Some(existing.to_string_lossy().into_owned());
	}

	let dir = icons_dir()?;
	let client = reqwest::Client::builder()
		.user_agent(USER_AGENT)
		.timeout(std::time::Duration::from_secs(20))
		.build()
		.ok()?;
	let response = match client.get(url).send().await {
		Ok(resp) => resp,
		Err(err) => {
			log::debug!("icon download failed url={url} err={err}");
			return None;
		}
	};
	if !response.status().is_success() {
		log::debug!(
			"icon download HTTP {} url={url}",
			response.status().as_u16()
		);
		return None;
	}
	let content_type = response
		.headers()
		.get(reqwest::header::CONTENT_TYPE)
		.and_then(|v| v.to_str().ok())
		.map(|s| s.to_string());
	let bytes = match response.bytes().await {
		Ok(b) => b,
		Err(err) => {
			log::debug!("icon body failed url={url} err={err}");
			return None;
		}
	};
	if bytes.is_empty() {
		return None;
	}
	let ext = extension_from_url_or_content_type(url, content_type.as_deref());
	let path = dir.join(format!("{stem}.{ext}"));
	if let Err(err) = fs::write(&path, &bytes) {
		log::warn!("icon write failed path={} err={err}", path.display());
		return None;
	}
	log::debug!(
		"icon cached stem={stem} bytes={} path={}",
		bytes.len(),
		path.display()
	);
	Some(path.to_string_lossy().into_owned())
}

pub async fn localize_installed_thumbnails(addons: &mut [InstalledAddon]) {
	for addon in addons.iter_mut() {
		let Some(url) = addon.thumbnail_url.clone() else {
			continue;
		};
		if !is_remote_url(&url) {
			continue;
		}
		if let Some(local) = ensure_cached(addon.curseforge_id, &url).await {
			addon.thumbnail_url = Some(local);
		}
	}
}

pub async fn localize_summary_thumbnails(items: &mut [AddonSummary]) {
	for item in items.iter_mut() {
		if !is_remote_url(&item.thumbnail_url) {
			continue;
		}
		if let Some(local) = ensure_cached(Some(item.id), &item.thumbnail_url).await {
			item.thumbnail_url = local;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cache_stem_prefers_mod_id() {
		assert_eq!(cache_stem(Some(42), "https://cdn.example/a.png"), "cf-42");
		assert!(cache_stem(None, "https://cdn.example/a.png").starts_with("u-"));
	}

	#[test]
	fn extension_detects_jpeg() {
		assert_eq!(
			extension_from_url_or_content_type("https://x/a", Some("image/jpeg")),
			"jpg"
		);
		assert_eq!(
			extension_from_url_or_content_type("https://x/a.webp?x=1", None),
			"webp"
		);
	}
}
