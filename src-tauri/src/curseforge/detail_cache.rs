use crate::types::AddonDetail;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_ENTRIES: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DetailCacheFile {
	version: u32,
	entries: BTreeMap<String, DetailCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetailCacheEntry {
	/// Updated on every successful read so eviction follows LRU order.
	accessed_at: String,
	detail: AddonDetail,
}

static MEMORY: Mutex<Option<DetailCacheFile>> = Mutex::new(None);

fn cache_path() -> Option<PathBuf> {
	let dir = dirs::config_dir()?.join("wow-addon-manager");
	fs::create_dir_all(&dir).ok()?;
	Some(dir.join("detail-cache.json"))
}

fn entry_key(mod_id: i64) -> String {
	mod_id.to_string()
}

fn load_file() -> DetailCacheFile {
	if let Ok(guard) = MEMORY.lock() {
		if let Some(cache) = guard.as_ref() {
			return cache.clone();
		}
	}
	let loaded: DetailCacheFile = cache_path()
		.and_then(|path| fs::read_to_string(path).ok())
		.and_then(|content| serde_json::from_str(&content).ok())
		.unwrap_or_default();
	if let Ok(mut guard) = MEMORY.lock() {
		*guard = Some(loaded.clone());
	}
	loaded
}

fn persist(cache: &DetailCacheFile) {
	if let Ok(mut guard) = MEMORY.lock() {
		*guard = Some(cache.clone());
	}
	let Some(path) = cache_path() else {
		return;
	};
	if let Ok(content) = serde_json::to_string_pretty(cache) {
		let _ = fs::write(path, content);
	}
}

pub fn get(mod_id: i64) -> Option<AddonDetail> {
	let key = entry_key(mod_id);
	let mut cache = load_file();
	let entry = cache.entries.get_mut(&key)?;
	entry.accessed_at = Utc::now().to_rfc3339();
	let detail = entry.detail.clone();
	persist(&cache);
	Some(detail)
}

pub fn put(mod_id: i64, detail: &AddonDetail) {
	let key = entry_key(mod_id);
	let mut cache = load_file();
	cache.version = 1;
	cache.entries.insert(
		key,
		DetailCacheEntry {
			accessed_at: Utc::now().to_rfc3339(),
			detail: detail.clone(),
		},
	);
	trim_lru(&mut cache);
	persist(&cache);
}

pub fn invalidate() {
	if let Ok(mut guard) = MEMORY.lock() {
		*guard = None;
	}
	if let Some(path) = cache_path() {
		let _ = fs::remove_file(path);
	}
}

fn trim_lru(cache: &mut DetailCacheFile) {
	if cache.entries.len() <= MAX_ENTRIES {
		return;
	}
	let mut ordered: Vec<_> = cache
		.entries
		.iter()
		.map(|(k, v)| (k.clone(), v.accessed_at.clone()))
		.collect();
	ordered.sort_by(|a, b| a.1.cmp(&b.1));
	let remove_count = cache.entries.len().saturating_sub(MAX_ENTRIES);
	for (key, _) in ordered.into_iter().take(remove_count) {
		cache.entries.remove(&key);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::{AddonScreenshot, AddonSummary};

	fn sample_detail(id: i64) -> AddonDetail {
		AddonDetail {
			addon: AddonSummary {
				id,
				slug: format!("addon-{id}"),
				name: format!("Addon {id}"),
				author: "Tester".into(),
				summary: String::new(),
				thumbnail_url: String::new(),
				download_count: 0,
				updated_at: String::new(),
				latest_game_version: String::new(),
				categories: Vec::new(),
				flavors: Vec::new(),
				website_url: String::new(),
				latest_file_id: None,
				latest_version: None,
			},
			description_html: format!("<p>{id}</p>"),
			screenshots: vec![AddonScreenshot {
				id: 1,
				title: String::new(),
				url: "https://example.com/a.png".into(),
				thumbnail_url: "https://example.com/a-t.png".into(),
			}],
		}
	}

	#[test]
	fn trim_lru_keeps_most_recently_accessed() {
		use chrono::DateTime;
		let mut cache = DetailCacheFile {
			version: 1,
			entries: BTreeMap::new(),
		};
		for id in 1..=105 {
			let accessed = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
				.unwrap()
				.with_timezone(&Utc)
				+ chrono::Duration::seconds(id);
			cache.entries.insert(
				entry_key(id),
				DetailCacheEntry {
					accessed_at: accessed.to_rfc3339(),
					detail: sample_detail(id),
				},
			);
		}
		trim_lru(&mut cache);
		assert_eq!(cache.entries.len(), MAX_ENTRIES);
		assert!(!cache.entries.contains_key("1"));
		assert!(!cache.entries.contains_key("5"));
		assert!(cache.entries.contains_key("6"));
		assert!(cache.entries.contains_key("105"));
	}
}
