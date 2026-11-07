pub mod catalog;
pub mod mock;
pub mod web;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use crate::types::{SearchParams, SearchResult};

pub async fn search_addons(params: SearchParams) -> SearchResult {
	match web::search(&params).await {
		Ok(result) => result,
		Err(err) => {
			let mut fallback = mock::search(&params);
			fallback.message = Some(format!(
				"CurseForge 网页请求失败（{}），已切换演示数据。申请官方 API Key 后可替换为稳定接口：https://forms.monday.com/forms/dce5ccb7afda9a1c21dab1a1aa1d84eb?r=use1",
				err
			));
			fallback
		}
	}
}
