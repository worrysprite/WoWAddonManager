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

	parse_response(response).await
}

async fn post_json(api_key: &str, path: &str, body: &Value) -> Result<Value, ApiError> {
	let url = format!("{API_BASE}{path}");
	log::info!("CurseForge POST {url} key={}", redact_key(api_key));
	let response = client()?
		.post(&url)
		.header("x-api-key", api_key)
		.header("Accept", "application/json")
		.header("Content-Type", "application/json")
		.json(body)
		.send()
		.await?;

	parse_response(response).await
}

async fn parse_response(response: reqwest::Response) -> Result<Value, ApiError> {
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
		if let Some(item) = map_mod(entry, None) {
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
		log::debug!("CurseForge 带 classId 搜索为空，尝试不带 classId 重试");
		let query = build_search_query(params, false);
		let json = get_json(api_key, "/v1/mods/search", &query).await?;
		result = parse_search_result(&json, params)?;
	}

	if result.items.is_empty() {
		log::debug!(
			"CurseForge 搜索无结果 total={} page={} search={:?} categories={:?}",
			result.total,
			result.page,
			params.search,
			params.categories
		);
		if result.message.is_none() {
			result.message = Some(if result.total == 0 {
				"官方 API 返回 0 条结果。可清空左侧筛选后重试，或检查 API Key 是否有 WoW 搜索权限。"
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
	preferred_type_id: Option<i64>,
	on_progress: Option<&(dyn Fn(crate::types::AddonJobProgress) + Send + Sync)>,
) -> Result<Vec<u8>, ApiError> {
	if let Some(cb) = on_progress {
		cb(crate::types::AddonJobProgress::download(Some(0), Some(0), None));
	}
	// When installing into a specific client (e.g. Forever), always resolve that
	// flavor's latestFilesIndexes entry — never fall back to a Retail hint fileId.
	let file_id = if preferred_type_id.is_some() {
		latest_file_id(api_key, mod_id, preferred_type_id).await.map_err(|err| {
			log::error!(
				"CurseForge 无适用于当前客户端的文件 mod_id={mod_id} preferred={preferred_type_id:?}: {err}"
			);
			ApiError::Parse(format!(
				"该插件没有适用于当前客户端的 CurseForge 文件（gameVersionTypeId={preferred_type_id:?}）"
			))
		})?
	} else {
		match file_id {
			Some(id) => id,
			None => latest_file_id(api_key, mod_id, None).await?,
		}
	};
	log::info!(
		"CurseForge 下载 mod_id={mod_id} file_id={file_id} preferred_type={preferred_type_id:?}"
	);

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

	let mut response = client()?.get(url).send().await?;
	let status = response.status();
	if !status.is_success() {
		log::error!("CurseForge 下载失败 HTTP {}", status.as_u16());
		return Err(ApiError::Http(status.as_u16(), "下载失败".into()));
	}

	let total = response.content_length();
	let mut received = 0u64;
	let mut bytes = Vec::with_capacity(total.unwrap_or(0) as usize);
	let mut last_pct = 0u32;
	if let Some(cb) = on_progress {
		cb(crate::types::AddonJobProgress::download(
			Some(0),
			Some(0),
			total,
		));
	}
	loop {
		match response.chunk().await? {
			Some(chunk) => {
				received += chunk.len() as u64;
				bytes.extend_from_slice(&chunk);
				if let Some(cb) = on_progress {
					let percent = total.map(|t| {
						if t == 0 {
							100
						} else {
							((received.saturating_mul(100)) / t).min(100) as u32
						}
					});
					let pct = percent.unwrap_or(0);
					// Throttle UI updates to whole-percent steps when size is known.
					if total.is_none() || pct != last_pct || received == total.unwrap_or(u64::MAX) {
						last_pct = pct;
						cb(crate::types::AddonJobProgress::download(
							percent,
							Some(received),
							total,
						));
					}
				}
			}
			None => break,
		}
	}
	if let Some(cb) = on_progress {
		cb(crate::types::AddonJobProgress::download(
			Some(100),
			Some(received),
			total.or(Some(received)),
		));
	}
	log::info!("CurseForge 下载完成 bytes={}", bytes.len());
	Ok(bytes)
}

pub async fn get_mod(
	api_key: &str,
	mod_id: i64,
	preferred_type_id: Option<i64>,
) -> Result<AddonSummary, ApiError> {
	let json = get_json(api_key, &format!("/v1/mods/{mod_id}"), &[]).await?;
	let data = json
		.get("data")
		.ok_or_else(|| ApiError::Parse("缺少 data".into()))?;
	map_mod(data, preferred_type_id)
		.ok_or_else(|| ApiError::Parse(format!("无法解析 mod {mod_id}")))
}

pub async fn get_mods(
	api_key: &str,
	mod_ids: &[i64],
	preferred_type_id: Option<i64>,
) -> Result<Vec<AddonSummary>, ApiError> {
	if mod_ids.is_empty() {
		return Ok(Vec::new());
	}
	let mut items = Vec::new();
	// CurseForge accepts batches; keep chunks modest.
	for chunk in mod_ids.chunks(50) {
		let body = serde_json::json!({
			"modIds": chunk,
			"filterPcOnly": true,
		});
		let json = post_json(api_key, "/v1/mods", &body).await?;
		let data = json
			.get("data")
			.and_then(|v| v.as_array())
			.ok_or_else(|| ApiError::Parse("缺少 data".into()))?;
		for entry in data {
			if let Some(item) = map_mod(entry, preferred_type_id) {
				items.push(item);
			}
		}
	}
	Ok(items)
}

/// Pick the latest file for a client flavor (e.g. Forever=88568) from mod JSON.
pub async fn latest_file_id(
	api_key: &str,
	mod_id: i64,
	preferred_type_id: Option<i64>,
) -> Result<i64, ApiError> {
	let summary = get_mod(api_key, mod_id, preferred_type_id).await?;
	summary
		.latest_file_id
		.ok_or_else(|| ApiError::Parse("无法确定最新文件".into()))
}

fn pick_file_from_indexes(
	indexes: &[Value],
	preferred_type_id: Option<i64>,
) -> (Option<i64>, String) {
	if let Some(want) = preferred_type_id {
		for index in indexes {
			if index.get("gameVersionTypeId").and_then(|v| v.as_i64()) != Some(want) {
				continue;
			}
			if let Some(file_id) = index.get("fileId").and_then(|v| v.as_i64()) {
				let ver = index
					.get("gameVersion")
					.and_then(|v| v.as_str())
					.unwrap_or("")
					.to_string();
				return (Some(file_id), ver);
			}
		}
		// Do not silently fall back to Retail when a flavor was requested.
		return (None, String::new());
	}
	if let Some(index) = indexes.first() {
		let file_id = index.get("fileId").and_then(|v| v.as_i64());
		let ver = index
			.get("gameVersion")
			.and_then(|v| v.as_str())
			.unwrap_or("")
			.to_string();
		return (file_id, ver);
	}
	(None, String::new())
}

fn map_mod(value: &Value, preferred_type_id: Option<i64>) -> Option<AddonSummary> {
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
	let categories = value
		.get("categories")
		.and_then(|v| v.as_array())
		.map(|arr| {
			arr.iter()
				.filter_map(|c| c.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
				.collect::<Vec<_>>()
		})
		.unwrap_or_default();

	let indexes = value
		.get("latestFilesIndexes")
		.and_then(|v| v.as_array())
		.cloned()
		.unwrap_or_default();

	let mut flavors = Vec::new();
	for index in &indexes {
		if let Some(type_id) = index.get("gameVersionTypeId").and_then(|v| v.as_i64()) {
			if let Some(label) = crate::curseforge::catalog::flavor_label(type_id) {
				if !flavors.iter().any(|f| f == label) {
					flavors.push(label.to_string());
				}
			}
		}
	}

	let (picked_file_id, picked_version) = pick_file_from_indexes(&indexes, preferred_type_id);
	let latest_file_id = if preferred_type_id.is_some() {
		picked_file_id
	} else {
		picked_file_id.or_else(|| value.get("mainFileId").and_then(|v| v.as_i64()))
	};
	let latest_game_version = if !picked_version.is_empty() {
		picked_version
	} else if preferred_type_id.is_none() {
		value
			.pointer("/latestFilesIndexes/0/gameVersion")
			.and_then(|v| v.as_str())
			.unwrap_or("")
			.to_string()
	} else {
		String::new()
	};

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
		let item = map_mod(&value, None).unwrap();
		assert_eq!(item.slug, "42");
		assert_eq!(item.name, "Demo");
	}

	#[test]
	fn pick_file_prefers_forever_type_id() {
		let indexes = vec![
			serde_json::json!({
				"fileId": 111,
				"gameVersion": "12.0.0",
				"gameVersionTypeId": 517
			}),
			serde_json::json!({
				"fileId": 222,
				"gameVersion": "1.60.0",
				"gameVersionTypeId": 88568
			}),
		];
		let (id, ver) = pick_file_from_indexes(&indexes, Some(88568));
		assert_eq!(id, Some(222));
		assert_eq!(ver, "1.60.0");
		let (fallback, _) = pick_file_from_indexes(&indexes, None);
		assert_eq!(fallback, Some(111));
	}

	#[test]
	fn pick_file_does_not_fall_back_to_retail_when_flavor_missing() {
		let indexes = vec![serde_json::json!({
			"fileId": 111,
			"gameVersion": "12.0.0",
			"gameVersionTypeId": 517
		})];
		let (id, ver) = pick_file_from_indexes(&indexes, Some(88568));
		assert_eq!(id, None);
		assert!(ver.is_empty());
	}
}
