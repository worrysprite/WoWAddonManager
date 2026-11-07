pub mod api;
pub mod browse_cache;
pub mod catalog;
pub mod mock;
pub mod web;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use crate::config;
use crate::types::{SearchParams, SearchResult};

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
		match api::search(&api_key, &params).await {
			Ok(result) => result,
			Err(err) => SearchResult {
				items: Vec::new(),
				page: params.page.unwrap_or(1).max(1),
				page_size: params.page_size.unwrap_or(20).clamp(1, 50),
				total: 0,
				source: "curseforge-api-error".to_string(),
				message: Some(format!(
					"CurseForge API 请求失败：{}。请检查设置中的 API Key 是否有效。",
					err
				)),
			},
		}
	} else {
		match web::search(&params).await {
			Ok(result) => result,
			Err(err) => {
				let mut fallback = mock::search(&params);
				fallback.message = Some(format!(
					"CurseForge 网页请求失败（{}），已切换演示数据。可在设置中填写 API Key 使用官方接口。",
					err
				));
				fallback
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
