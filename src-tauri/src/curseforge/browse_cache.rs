use crate::types::{SearchParams, SearchResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

const CACHE_TTL: Duration = Duration::from_secs(30 * 60);
const MAX_ENTRIES: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct BrowseCacheFile {
	version: u32,
	entries: BTreeMap<String, BrowseCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowseCacheEntry {
	saved_at: String,
	has_api_key: bool,
	result: SearchResult,
}

static MEMORY: Mutex<Option<BrowseCacheFile>> = Mutex::new(None);

fn cache_path() -> Option<PathBuf> {
	let dir = dirs::config_dir()?.join("wow-addon-manager");
	fs::create_dir_all(&dir).ok()?;
	Some(dir.join("browse-cache.json"))
}

fn load_file() -> BrowseCacheFile {
	if let Ok(guard) = MEMORY.lock() {
		if let Some(cache) = guard.as_ref() {
			return cache.clone();
		}
	}
	let loaded: BrowseCacheFile = cache_path()
		.and_then(|path| fs::read_to_string(path).ok())
		.and_then(|content| serde_json::from_str(&content).ok())
		.unwrap_or_default();
	if let Ok(mut guard) = MEMORY.lock() {
		*guard = Some(loaded.clone());
	}
	loaded
}

fn persist(cache: &BrowseCacheFile) {
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

pub fn cache_key(params: &SearchParams, has_api_key: bool) -> String {
	let mut categories = params.categories.clone().unwrap_or_default();
	categories.sort();
	categories.dedup();
	let mut flavors = params.game_version_type_ids.clone().unwrap_or_default();
	flavors.sort_unstable();
	flavors.dedup();
	let payload = serde_json::json!({
		"search": params.search.as_deref().unwrap_or("").trim().to_lowercase(),
		"page": params.page.unwrap_or(1).max(1),
		"pageSize": params.page_size.unwrap_or(20).clamp(1, 50),
		"categories": categories,
		"flavors": flavors,
		"sortBy": params.sort_by.as_deref().unwrap_or("relevancy"),
		"hasApiKey": has_api_key,
	});
	let raw = payload.to_string();
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	raw.hash(&mut hasher);
	format!("{:016x}", hasher.finish())
}

pub fn get(params: &SearchParams, has_api_key: bool) -> Option<SearchResult> {
	let key = cache_key(params, has_api_key);
	let cache = load_file();
	let entry = cache.entries.get(&key)?;
	if entry.has_api_key != has_api_key {
		return None;
	}
	let saved = DateTime::parse_from_rfc3339(&entry.saved_at).ok()?;
	let age = Utc::now().signed_duration_since(saved.with_timezone(&Utc));
	if age.to_std().unwrap_or(Duration::MAX) > CACHE_TTL {
		return None;
	}
	// Return cached payload as-is; do not surface cache mechanics in the UI.
	Some(entry.result.clone())
}

pub fn put(params: &SearchParams, has_api_key: bool, result: &SearchResult) {
	// Skip caching hard failures so a later retry can recover.
	if result.source.contains("error") {
		return;
	}
	let key = cache_key(params, has_api_key);
	let mut cache = load_file();
	cache.version = 1;
	cache.entries.insert(
		key,
		BrowseCacheEntry {
			saved_at: Utc::now().to_rfc3339(),
			has_api_key,
			result: result.clone(),
		},
	);
	trim_old_entries(&mut cache);
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

fn trim_old_entries(cache: &mut BrowseCacheFile) {
	if cache.entries.len() <= MAX_ENTRIES {
		return;
	}
	let mut ordered: Vec<_> = cache
		.entries
		.iter()
		.map(|(k, v)| (k.clone(), v.saved_at.clone()))
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

	#[test]
	fn cache_key_is_stable_for_same_params() {
		let params = SearchParams {
			search: Some("  BarTender ".into()),
			page: Some(1),
			page_size: Some(20),
			categories: Some(vec!["artwork".into(), "action-bars".into()]),
			game_version_type_ids: Some(vec![517, 88568]),
			sort_by: Some("relevancy".into()),
		};
		let a = cache_key(&params, true);
		let mut shuffled = params.clone();
		shuffled.categories = Some(vec!["action-bars".into(), "artwork".into()]);
		shuffled.game_version_type_ids = Some(vec![88568, 517]);
		let b = cache_key(&shuffled, true);
		assert_eq!(a, b);
		assert_ne!(a, cache_key(&params, false));
	}
}
