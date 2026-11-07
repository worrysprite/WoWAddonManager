use crate::types::{AddonSummary, SearchParams, SearchResult};
use serde_json::Value;
use thiserror::Error;

const API_BASE: &str = "https://api.curseforge.com";
const WOW_GAME_ID: i32 = 1;
const WOW_ADDONS_CLASS_ID: i32 = 6552;
const USER_AGENT: &str = "WoWAddonManager/0.1 (WorrySprite)";

#[derive(Debug, Error)]
pub enum ApiError {
	#[error("HTTP {0}: {1}")]
	Http(u16, String),
	#[error("解析响应失败: {0}")]
	Parse(String),
	#[error(transparent)]
	Request(#[from] reqwest::Error),
}

fn client() -> Result<reqwest::Client, ApiError> {
	Ok(reqwest::Client::builder()
		.user_agent(USER_AGENT)
		.timeout(std::time::Duration::from_secs(30))
		.build()?)
}

async fn get_json(api_key: &str, path: &str, query: &[(&str, String)]) -> Result<Value, ApiError> {
	let response = client()?
		.get(format!("{API_BASE}{path}"))
		.header("x-api-key", api_key)
		.header("Accept", "application/json")
		.query(query)
		.send()
		.await?;

	let status = response.status();
	let body = response.text().await?;
	if !status.is_success() {
		return Err(ApiError::Http(
			status.as_u16(),
			body.chars().take(200).collect(),
		));
	}
	serde_json::from_str(&body).map_err(|e| ApiError::Parse(e.to_string()))
}

pub async fn search(api_key: &str, params: &SearchParams) -> Result<SearchResult, ApiError> {
	let page = params.page.unwrap_or(1).max(1);
	let page_size = params.page_size.unwrap_or(20).clamp(1, 50);
	let index = (page - 1) * page_size;

	let mut query: Vec<(&str, String)> = vec![
		("gameId", WOW_GAME_ID.to_string()),
		("classId", WOW_ADDONS_CLASS_ID.to_string()),
		("pageSize", page_size.to_string()),
		("index", index.to_string()),
		("sortField", sort_field(params.sort_by.as_deref()).to_string()),
		("sortOrder", "desc".into()),
	];

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

	let json = get_json(api_key, "/v1/mods/search", &query).await?;
	let data = json
		.get("data")
		.and_then(|v| v.as_array())
		.ok_or_else(|| ApiError::Parse("缺少 data".into()))?;

	let mut items = Vec::new();
	for entry in data {
		if let Some(item) = map_mod(entry) {
			items.push(item);
		}
	}

	if let Some(categories) = &params.categories {
		if !categories.is_empty() {
			items.retain(|item| {
				categories.iter().any(|c| {
					item.categories
						.iter()
						.any(|ic| ic.eq_ignore_ascii_case(c) || slugify(ic) == *c)
				})
			});
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

pub async fn download_addon_zip(
	api_key: &str,
	mod_id: i64,
	file_id: Option<i64>,
) -> Result<Vec<u8>, ApiError> {
	let file_id = match file_id {
		Some(id) => id,
		None => latest_file_id(api_key, mod_id).await?,
	};

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
		return Err(ApiError::Http(status.as_u16(), "下载失败".into()));
	}
	Ok(response.bytes().await?.to_vec())
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
	let slug = value.get("slug")?.as_str()?.to_string();
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
