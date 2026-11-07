use crate::types::{AddonSummary, SearchParams, SearchResult};
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use thiserror::Error;

const SEARCH_BASE: &str = "https://www.curseforge.com/wow/search";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 WoWAddonManager/0.1";

#[derive(Debug, Error)]
pub enum WebError {
	#[error("HTTP {0}")]
	Http(String),
	#[error("Cloudflare challenge")]
	Cloudflare,
	#[error("无法解析搜索结果")]
	Parse,
	#[error(transparent)]
	Request(#[from] reqwest::Error),
}

pub async fn search(params: &SearchParams) -> Result<SearchResult, WebError> {
	let page = params.page.unwrap_or(1).max(1);
	let page_size = params.page_size.unwrap_or(20).clamp(1, 50);
	let url = build_search_url(params, page, page_size);

	let client = reqwest::Client::builder()
		.user_agent(USER_AGENT)
		.redirect(reqwest::redirect::Policy::limited(10))
		.timeout(std::time::Duration::from_secs(20))
		.build()?;

	let response = client
		.get(&url)
		.header("Accept", "text/html,application/xhtml+xml,application/json")
		.header("Accept-Language", "en-US,en;q=0.9")
		.send()
		.await?;

	let status = response.status();
	let body = response.text().await?;

	if status.as_u16() == 403 || body.contains("Just a moment...") || body.contains("cf-mitigated")
	{
		return Err(WebError::Cloudflare);
	}
	if !status.is_success() {
		return Err(WebError::Http(format!("{status}")));
	}

	if let Some(result) = parse_next_data(&body, page, page_size) {
		return Ok(result);
	}
	if let Some(result) = parse_html_cards(&body, page, page_size) {
		return Ok(result);
	}

	Err(WebError::Parse)
}

pub fn build_search_url(params: &SearchParams, page: u32, page_size: u32) -> String {
	let mut query = vec![
		("class".to_string(), "addons".to_string()),
		("page".to_string(), page.to_string()),
		("pageSize".to_string(), page_size.to_string()),
		(
			"sortBy".to_string(),
			params
				.sort_by
				.clone()
				.unwrap_or_else(|| "relevancy".to_string()),
		),
	];

	if let Some(search) = params.search.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
		query.push(("search".to_string(), search.to_string()));
	}
	if let Some(categories) = &params.categories {
		if !categories.is_empty() {
			query.push(("categories".to_string(), categories.join(",")));
		}
	}
	if let Some(ids) = &params.game_version_type_ids {
		if !ids.is_empty() {
			query.push((
				"gameVersionTypeId".to_string(),
				ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","),
			));
		}
	}

	let qs = query
		.into_iter()
		.map(|(k, v)| format!("{k}={}", urlencoding::encode(&v)))
		.collect::<Vec<_>>()
		.join("&");
	format!("{SEARCH_BASE}?{qs}")
}

fn parse_next_data(body: &str, page: u32, page_size: u32) -> Option<SearchResult> {
	let re = Regex::new(r#"<script id="__NEXT_DATA__" type="application/json">(.*?)</script>"#)
		.ok()?;
	let caps = re.captures(body)?;
	let json: Value = serde_json::from_str(caps.get(1)?.as_str()).ok()?;

	let candidates = [
		json.pointer("/props/pageProps/results"),
		json.pointer("/props/pageProps/data/results"),
		json.pointer("/props/pageProps/searchResults"),
		json.pointer("/props/pageProps/projects"),
	];

	for candidate in candidates.into_iter().flatten() {
		if let Some(result) = parse_json_results(candidate, page, page_size) {
			return Some(result);
		}
	}

	fn walk(value: &Value, page: u32, page_size: u32) -> Option<SearchResult> {
		match value {
			Value::Array(arr) if !arr.is_empty() && arr.iter().any(|v| v.get("slug").is_some() || v.get("name").is_some()) => {
				parse_json_results(value, page, page_size)
			}
			Value::Object(map) => {
				for (key, child) in map {
					if matches!(key.as_str(), "results" | "data" | "hits" | "projects" | "addons") {
						if let Some(found) = parse_json_results(child, page, page_size) {
							return Some(found);
						}
					}
					if let Some(found) = walk(child, page, page_size) {
						return Some(found);
					}
				}
				None
			}
			Value::Array(arr) => {
				for child in arr {
					if let Some(found) = walk(child, page, page_size) {
						return Some(found);
					}
				}
				None
			}
			_ => None,
		}
	}

	walk(&json, page, page_size)
}

fn parse_json_results(value: &Value, page: u32, page_size: u32) -> Option<SearchResult> {
	let items_value = if value.is_array() {
		value
	} else {
		value
			.get("data")
			.or_else(|| value.get("hits"))
			.or_else(|| value.get("results"))
			.or_else(|| value.get("projects"))?
	};

	let arr = items_value.as_array()?;
	if arr.is_empty() {
		return None;
	}

	let mut items = Vec::new();
	for entry in arr {
		if let Some(item) = map_json_addon(entry) {
			items.push(item);
		}
	}
	if items.is_empty() {
		return None;
	}

	let total = value
		.get("totalCount")
		.or_else(|| value.get("total"))
		.or_else(|| value.get("pagination").and_then(|p| p.get("totalCount")))
		.and_then(|v| v.as_u64())
		.unwrap_or(items.len() as u64) as u32;

	Some(SearchResult {
		items,
		page,
		page_size,
		total,
		source: "curseforge-web".to_string(),
		message: None,
	})
}

fn map_json_addon(value: &Value) -> Option<AddonSummary> {
	let id = value
		.get("id")
		.or_else(|| value.get("projectId"))
		.and_then(|v| v.as_i64())?;
	let slug = value
		.get("slug")
		.and_then(|v| v.as_str())
		.unwrap_or("")
		.to_string();
	if slug.is_empty() {
		return None;
	}
	let name = value
		.get("name")
		.or_else(|| value.get("title"))
		.and_then(|v| v.as_str())
		.unwrap_or(&slug)
		.to_string();
	let summary = value
		.get("summary")
		.or_else(|| value.get("description"))
		.or_else(|| value.get("shortDescription"))
		.and_then(|v| v.as_str())
		.unwrap_or("")
		.to_string();
	let author = value
		.pointer("/authors/0/name")
		.and_then(|v| v.as_str())
		.or_else(|| value.pointer("/author/name").and_then(|v| v.as_str()))
		.or_else(|| value.get("author").and_then(|v| v.as_str()))
		.unwrap_or("Unknown")
		.to_string();
	let thumbnail_url = value
		.pointer("/logo/thumbnailUrl")
		.or_else(|| value.pointer("/avatar/thumbnailUrl"))
		.or_else(|| value.get("thumbnailUrl"))
		.and_then(|v| v.as_str())
		.unwrap_or("")
		.to_string();
	let download_count = value
		.get("downloadCount")
		.or_else(|| value.pointer("/downloadCount"))
		.and_then(|v| v.as_i64())
		.unwrap_or(0);
	let updated_at = value
		.get("dateModified")
		.or_else(|| value.get("updatedAt"))
		.and_then(|v| v.as_str())
		.unwrap_or("")
		.to_string();
	let latest_game_version = value
		.pointer("/latestFilesIndexes/0/gameVersion")
		.or_else(|| value.pointer("/gameVersions/0"))
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
	let latest_file_id = value
		.pointer("/latestFilesIndexes/0/fileId")
		.or_else(|| value.pointer("/latestFiles/0/id"))
		.and_then(|v| v.as_i64());

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
		flavors: Vec::new(),
		latest_file_id,
	})
}

fn parse_html_cards(body: &str, page: u32, page_size: u32) -> Option<SearchResult> {
	let document = Html::parse_document(body);
	let link_sel = Selector::parse(r#"a[href*="/wow/addons/"]"#).ok()?;
	let mut items = Vec::new();
	let mut seen = std::collections::HashSet::new();

	for element in document.select(&link_sel) {
		let href = element.value().attr("href").unwrap_or("");
		let slug = href
			.trim_start_matches("https://www.curseforge.com")
			.trim_start_matches("/wow/addons/")
			.split('/')
			.next()
			.unwrap_or("")
			.to_string();
		if slug.is_empty() || !seen.insert(slug.clone()) {
			continue;
		}
		let name = element.text().collect::<String>().trim().to_string();
		if name.is_empty() || name.len() < 2 {
			continue;
		}
		items.push(AddonSummary {
			id: (seen.len() as i64) * -1,
			slug: slug.clone(),
			name,
			author: "Unknown".to_string(),
			summary: String::new(),
			thumbnail_url: String::new(),
			download_count: 0,
			updated_at: String::new(),
			latest_game_version: String::new(),
			categories: Vec::new(),
			flavors: Vec::new(),
			website_url: format!("https://www.curseforge.com/wow/addons/{slug}"),
			latest_file_id: None,
		});
		if items.len() >= page_size as usize {
			break;
		}
	}

	if items.is_empty() {
		None
	} else {
		Some(SearchResult {
			total: items.len() as u32,
			items,
			page,
			page_size,
			source: "curseforge-web-html".to_string(),
			message: Some("已从网页 HTML 粗解析结果，字段可能不完整。".to_string()),
		})
	}
}

pub async fn download_addon_zip(slug: &str, file_id: Option<i64>) -> Result<Vec<u8>, WebError> {
	let client = reqwest::Client::builder()
		.user_agent(USER_AGENT)
		.redirect(reqwest::redirect::Policy::limited(10))
		.timeout(std::time::Duration::from_secs(60))
		.build()?;

	let urls = match file_id {
		Some(id) => vec![
			format!("https://www.curseforge.com/wow/addons/{slug}/download/{id}/file"),
			format!("https://www.curseforge.com/wow/addons/{slug}/download/{id}"),
		],
		None => vec![format!(
			"https://www.curseforge.com/wow/addons/{slug}/download"
		)],
	};

	let mut last_err = WebError::Http("no download url".into());
	for url in urls {
		match client.get(&url).send().await {
			Ok(resp) => {
				if resp.status().is_success() {
					let bytes = resp.bytes().await?;
					if bytes.len() > 64 {
						return Ok(bytes.to_vec());
					}
					last_err = WebError::Http("empty download".into());
				} else {
					last_err = WebError::Http(format!("download {}", resp.status()));
				}
			}
			Err(err) => last_err = WebError::Request(err),
		}
	}
	Err(last_err)
}
