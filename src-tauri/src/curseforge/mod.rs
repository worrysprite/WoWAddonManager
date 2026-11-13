pub mod api;
pub mod browse_cache;
pub mod catalog;
pub mod web;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use crate::config;
use crate::types::{AddonSummary, SearchParams, SearchResult};
use std::collections::HashMap;

pub async fn search_addons(params: SearchParams, force_refresh: bool) -> SearchResult {
	let has_api_key = config::api_key().is_some();
	if !force_refresh {
		if let Some(cached) = browse_cache::get(&params, has_api_key) {
			return cached;
		}
	}

	let result = fetch_search(params.clone()).await;
	browse_cache::put(&params, has_api_key, &result);
	result
}

async fn fetch_search(params: SearchParams) -> SearchResult {
	if let Some(api_key) = config::api_key() {
		log::info!("搜索走官方 API（已配置 Key，len={}）", api_key.len());
		match api::search(&api_key, &params).await {
			Ok(result) => result,
			Err(err) => {
				log::error!("CurseForge API 搜索失败: {err}");
				SearchResult {
					items: Vec::new(),
					page: params.page.unwrap_or(1).max(1),
					page_size: params.page_size.unwrap_or(20).clamp(1, 50),
					total: 0,
					source: "curseforge-api-error".to_string(),
					message: Some(format!(
						"CurseForge API 请求失败：{}。请检查设置中的 API Key 是否有效。",
						err
					)),
				}
			}
		}
	} else {
		log::info!("搜索走网页 GET（未配置 API Key）");
		match web::search(&params).await {
			Ok(result) => result,
			Err(err) => {
				log::error!("CurseForge 网页搜索失败: {err}");
				SearchResult {
					items: Vec::new(),
					page: params.page.unwrap_or(1).max(1),
					page_size: params.page_size.unwrap_or(20).clamp(1, 50),
					total: 0,
					source: "curseforge-web-error".to_string(),
					message: Some(format!(
						"CurseForge 网页请求失败：{}。可在设置中填写 API Key 使用官方接口。",
						err
					)),
				}
			}
		}
	}
}

pub async fn download_addon_zip(
	mod_id: i64,
	slug: &str,
	file_id: Option<i64>,
) -> Result<Vec<u8>, String> {
	if let Some(api_key) = config::api_key() {
		api::download_addon_zip(&api_key, mod_id, file_id)
			.await
			.map_err(|e| e.to_string())
	} else {
		web::download_addon_zip(slug, file_id)
			.await
			.map_err(|e| e.to_string())
	}
}

/// Fetch one mod by CurseForge id (official API only).
pub async fn get_addon(mod_id: i64) -> Option<AddonSummary> {
	let api_key = config::api_key()?;
	match api::get_mod(&api_key, mod_id).await {
		Ok(item) => Some(item),
		Err(err) => {
			log::warn!("CurseForge get mod {mod_id} failed: {err}");
			None
		}
	}
}

/// Batch-fetch mods by id. Returns a map keyed by CurseForge mod id.
pub async fn get_addons_by_ids(mod_ids: &[i64]) -> HashMap<i64, AddonSummary> {
	let mut out = HashMap::new();
	if mod_ids.is_empty() {
		return out;
	}
	let Some(api_key) = config::api_key() else {
		return out;
	};
	let mut unique = mod_ids.to_vec();
	unique.sort_unstable();
	unique.dedup();
	match api::get_mods(&api_key, &unique).await {
		Ok(items) => {
			for item in items {
				out.insert(item.id, item);
			}
		}
		Err(err) => {
			log::warn!(
				"CurseForge batch get mods failed count={} err={err}",
				unique.len()
			);
		}
	}
	out
}
