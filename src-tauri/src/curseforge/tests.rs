#[cfg(test)]
mod tests {
	use crate::curseforge::{mock, web};
	use crate::types::SearchParams;
	use crate::wow::detect;

	#[test]
	fn mock_search_filters_by_keyword() {
		let result = mock::search(&SearchParams {
			search: Some("bartender".into()),
			page: Some(1),
			page_size: Some(20),
			categories: None,
			game_version_type_ids: None,
			sort_by: None,
		});
		assert!(!result.items.is_empty());
		assert!(result.items.iter().any(|a| a.slug.contains("bartender")));
	}

	#[test]
	fn builds_expected_search_url() {
		let url = web::build_search_url(
			&SearchParams {
				search: Some("bar".into()),
				page: Some(1),
				page_size: Some(20),
				categories: Some(vec!["action-bars".into(), "artwork".into()]),
				game_version_type_ids: Some(vec![88568, 517]),
				sort_by: Some("relevancy".into()),
			},
			1,
			20,
		);
		assert!(url.contains("class=addons"));
		assert!(url.contains("search=bar"));
		assert!(url.contains("categories=action-bars%2Cartwork"));
		assert!(url.contains("gameVersionTypeId=88568%2C517"));
	}

	#[test]
	fn detects_mock_wow_install() {
		let installs = detect::detect_installs();
		assert!(
			installs.iter().any(|i| i.flavor_key == "retail"),
			"expected mock retail install, got: {:?}",
			installs
		);
	}
}
