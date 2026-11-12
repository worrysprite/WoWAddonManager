use crate::types::{AddonSummary, SearchParams, SearchResult};
use serde_json::Value;
use thiserror::Error;

const API_BASE: &str = "https://api.curseforge.com";
const WOW_GAME_ID: i32 = 1;
/// WoW "Addons" class/section id from CurseForge categories.
const WOW_ADDONS_CLASS_ID: i32 = 6552;
const USER_AGENT: &str = "WoWAddonManager/0.4 (WorrySprite)";

#[derive(Debug, Error)]
pub enum ApiError {
	#[error("HTTP {0}: {1}")]
	Http(u16, String),
	#[error("解析响应失败: {0}")]
	Parse(String),
	#[error(transparent)]
	Request(#[from] reqwest::Error),
}

fn redact_key(api_key: &str) -> String {
	let chars: Vec<char> = api_key.chars().collect();
	if chars.len() <= 8 {
		return format!("(len={})", chars.len());
	}
	let head: String = chars.iter().take(4).collect();
	let tail: String = chars.iter().rev().take(4).rev().collect();
	format!("{head}…{tail} (len={})", chars.len())
}

fn client() -> Result<reqwest::Client, ApiError> {
	Ok(reqwest::Client::builder()
		.user_agent(USER_AGENT)
		.timeout(std::time::Duration::from_secs(30))
		.build()?)
}

async fn get_json(api_key: &str, path: &str, query: &[(&str, String)]) -> Result<Value, ApiError> {
	let url = format!("{API_BASE}{path}");
	log::info!(
		"CurseForge GET {url}?{} key={}",
		query
			.iter()
			.map(|(k, v)| format!("{k}={v}"))
			.collect::<Vec<_>>()
			.join("&"),
		redact_key(api_key)
	);
	let response = client()?
		.get(&url)
		.header("x-api-key", api_key)
		.header("Accept", "application/json")
		.query(query)
		.send()
		.await?;

	let status = response.status();
	let body = response.text().await?;
	if !status.is_success() {
		let snippet: String = body.chars().take(300).collect();
		log::error!("CurseForge HTTP {} body={}", status.as_u16(), snippet);
		return Err(ApiError::Http(status.as_u16(), snippet));
	}
	log::debug!(
		"CurseForge OK {} body_len={}",
		status.as_u16(),
		body.len()
	);
	serde_json::from_str(&body).map_err(|e| {
		let snippet: String = body.chars().take(200).collect();
		log::error!("CurseForge JSON 解析失败: {e}; body={snippet}");
		ApiError::Parse(e.to_string())
	})
}

fn build_search_query(params: &SearchParams, include_class_id: bool) -> Vec<(&'static str, String)> {
	let page = params.page.unwrap_or(1).max(1);
	let page_size = params.page_size.unwrap_or(20).clamp(1, 50);
	let index = (page - 1) * page_size;

	let mut query: Vec<(&str, String)> = vec![
		("gameId", WOW_GAME_ID.to_string()),
		("pageSize", page_size.to_string()),
		("index", index.to_string()),
		("sortField", sort_field(params.sort_by.as_deref()).to_string()),
		("sortOrder", "desc".into()),
	];
	if include_class_id {
		query.push(("classId", WOW_ADDONS_CLASS_ID.to_string()));
	}

	if let Some(search) = params
		.search
		.as_ref()
		.map(|s| s.trim())
		.filter(|s| !s.is_empty())
	{
		query.push(("searchFilter", search.to_string()));
	}

	if let Some(ids) = &params.game_version_type_ids {
		if let Some(first) = ids.first() {
			query.push(("gameVersionTypeId", first.to_string()));
		}
	}

	query
}

fn parse_search_result(json: &Value, params: &SearchParams) -> Result<SearchResult, ApiError> {
	let page = params.page.unwrap_or(1).max(1);
	let page_size = params.page_size.unwrap_or(20).clamp(1, 50);
	let data = json
		.get("data")
		.and_then(|v| v.as_array())
		.ok_or_else(|| ApiError::Parse("缺少 data".into()))?;

	let mut items = Vec::new();
	let mut skipped = 0usize;
	for entry in data {
		if let Some(item) = map_mod(entry) {
			items.push(item);
		} else {
			skipped += 1;
		}
	}
	if skipped > 0 {
		log::warn!("CurseForge 搜索跳过无法解析的条目: {skipped}");
	}

	if let Some(categories) = &params.categories {
		if !categories.is_empty() {
			let before = items.len();
			items.retain(|item| {
				categories.iter().any(|c| {
					item.categories
						.iter()
						.any(|ic| ic.eq_ignore_ascii_case(c) || slugify(ic) == *c)
				})
			});
			log::info!(
				"CurseForge 分类过滤 categories={categories:?} before={before} after={}",
				items.len()
			);
		}
	}

	let total = json
		.pointer("/pagination/totalCount")
		.and_then(|v| v.as_u64())
		.unwrap_or(items.len() as u64) as u32;

	Ok(SearchResult {
		items,
		page,
		page_size,
		total,
		source: "curseforge-api".to_string(),
		message: None,
	})
}

pub async fn search(api_key: &str, params: &SearchParams) -> Result<SearchResult, ApiError> {
	log::info!(
		"CurseForge 搜索开始 search={:?} page={:?} flavors={:?} categories={:?}",
		params.search,
		params.page,
		params.game_version_type_ids,
		params.categories
	);

	let query = build_search_query(params, true);
	let json = get_json(api_key, "/v1/mods/search", &query).await?;
	let mut result = parse_search_result(&json, params)?;

	// Some keys / responses behave better without classId; retry once if empty.
	if result.items.is_empty() && result.total == 0 {
		log::warn!("CurseForge 带 classId 搜索为空，尝试不带 classId 重试");
		let query = build_search_query(params, false);
		let json = get_json(api_key, "/v1/mods/search", &query).await?;
		result = parse_search_result(&json, params)?;
	}

	if result.items.is_empty() {
		log::warn!(
			"CurseForge 搜索无结果 total={} page={} search={:?} categories={:?}",
			result.total,
			result.page,
			params.search,
			params.categories
		);
		if result.message.is_none() {
			result.message = Some(if result.total == 0 {
				"官方 API 返回 0 条结果。可清空左侧筛选后点「刷新」，或检查 API Key 是否有 WoW 搜索权限。"
					.into()
			} else {
				format!(
					"API 返回了 {total} 条，但当前分类筛选后无可显示项。请清空左侧分类后重试。",
					total = result.total
				)
			});
		}
	} else {
		log::info!(
			"CurseForge 搜索成功 items={} total={}",
			result.items.len(),
			result.total
		);
	}

	Ok(result)
}

pub async fn download_addon_zip(
	api_key: &str,
	mod_id: i64,
	file_id: Option<i64>,
) -> Result<Vec<u8>, ApiError> {
	let file_id = match file_id {
		Some(id) => id,
		None => latest_file_id(api_key, mod_id).await?,
	};
	log::info!("CurseForge 下载 mod_id={mod_id} file_id={file_id}");

	let json = get_json(
		api_key,
		&format!("/v1/mods/{mod_id}/files/{file_id}/download-url"),
		&[],
	)
	.await?;
	let url = json
		.get("data")
		.and_then(|v| v.as_str())
		.ok_or_else(|| ApiError::Parse("缺少 download url".into()))?;

	let response = client()?.get(url).send().await?;
	let status = response.status();
	if !status.is_success() {
		log::error!("CurseForge 下载失败 HTTP {}", status.as_u16());
		return Err(ApiError::Http(status.as_u16(), "下载失败".into()));
	}
	let bytes = response.bytes().await?.to_vec();
	log::info!("CurseForge 下载完成 bytes={}", bytes.len());
	Ok(bytes)
}

async fn latest_file_id(api_key: &str, mod_id: i64) -> Result<i64, ApiError> {
	let json = get_json(api_key, &format!("/v1/mods/{mod_id}"), &[]).await?;
	json.pointer("/data/latestFilesIndexes/0/fileId")
		.or_else(|| json.pointer("/data/mainFileId"))
		.and_then(|v| v.as_i64())
		.ok_or_else(|| ApiError::Parse("无法确定最新文件".into()))
}

fn map_mod(value: &Value) -> Option<AddonSummary> {
	let id = value.get("id")?.as_i64()?;
	let slug = value
		.get("slug")
		.and_then(|v| v.as_str())
		.filter(|s| !s.is_empty())
		.map(|s| s.to_string())
		.unwrap_or_else(|| id.to_string());
	let name = value
		.get("name")
		.and_then(|v| v.as_str())
		.unwrap_or(&slug)
		.to_string();
	let summary = value
		.get("summary")
		.and_then(|v| v.as_str())
		.unwrap_or("")
		.to_string();
	let author = value
		.pointer("/authors/0/name")
		.and_then(|v| v.as_str())
		.unwrap_or("Unknown")
		.to_string();
	let thumbnail_url = value
		.pointer("/logo/thumbnailUrl")
		.or_else(|| value.pointer("/logo/url"))
		.and_then(|v| v.as_str())
		.unwrap_or("")
		.to_string();
	let download_count = value
		.get("downloadCount")
		.and_then(|v| v.as_f64())
		.map(|n| n as i64)
		.unwrap_or(0);
	let updated_at = value
		.get("dateModified")
		.and_then(|v| v.as_str())
		.unwrap_or("")
		.to_string();
	let latest_game_version = value
		.pointer("/latestFilesIndexes/0/gameVersion")
		.and_then(|v| v.as_str())
		.unwrap_or("")
		.to_string();
	let latest_file_id = value
		.pointer("/latestFilesIndexes/0/fileId")
		.or_else(|| value.get("mainFileId"))
		.and_then(|v| v.as_i64());
	let categories = value
		.get("categories")
		.and_then(|v| v.as_array())
		.map(|arr| {
			arr.iter()
				.filter_map(|c| c.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
				.collect::<Vec<_>>()
		})
		.unwrap_or_default();

	let mut flavors = Vec::new();
	if let Some(indexes) = value.get("latestFilesIndexes").and_then(|v| v.as_array()) {
		for index in indexes {
			if let Some(type_id) = index.get("gameVersionTypeId").and_then(|v| v.as_i64()) {
				if let Some(label) = crate::curseforge::catalog::flavor_label(type_id) {
					if !flavors.iter().any(|f| f == label) {
						flavors.push(label.to_string());
					}
				}
			}
		}
	}

	Some(AddonSummary {
		id,
		website_url: format!("https://www.curseforge.com/wow/addons/{slug}"),
		slug,
		name,
		author,
		summary,
		thumbnail_url,
		download_count,
		updated_at,
		latest_game_version,
		categories,
		flavors,
		latest_file_id,
	})
}

fn sort_field(sort_by: Option<&str>) -> i32 {
	// CurseForge SortField: 1=Featured, 2=Popularity, 3=LastUpdated, 4=Name, 5=Author, 6=TotalDownloads, 8=Rating
	match sort_by.unwrap_or("relevancy") {
		"popularity" => 2,
		"lastUpdated" | "date" => 3,
		"name" => 4,
		"totalDownloads" | "downloads" => 6,
		_ => 2,
	}
}

fn slugify(value: &str) -> String {
	value
		.to_lowercase()
		.chars()
		.map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
		.collect::<String>()
		.split('-')
		.filter(|s| !s.is_empty())
		.collect::<Vec<_>>()
		.join("-")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn redact_key_hides_middle() {
		let out = redact_key("$2a$10$abcdefghijklmnop");
		assert!(out.contains('…'));
		assert!(out.contains("len="));
		assert!(!out.contains("abcdefghijklmnop"));
	}

	#[test]
	fn map_mod_falls_back_slug_to_id() {
		let value = serde_json::json!({
			"id": 42,
			"name": "Demo",
			"summary": "x",
			"authors": [{"name": "A"}],
			"downloadCount": 1,
			"dateModified": "2026-01-01",
			"categories": [],
			"latestFilesIndexes": []
		});
		let item = map_mod(&value).unwrap();
		assert_eq!(item.slug, "42");
		assert_eq!(item.name, "Demo");
	}
}
