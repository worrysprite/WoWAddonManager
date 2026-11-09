#[cfg(test)]
mod tests {
	use crate::curseforge::web;
	use crate::types::SearchParams;
	use crate::wow::detect;

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

	#[test]
	fn extracts_manual_zip_addon() {
		use std::io::Write;
		use zip::write::SimpleFileOptions;

		let dir = tempfile_dir();
		let zip_path = dir.join("DemoZipAddon.zip");
		{
			let file = std::fs::File::create(&zip_path).unwrap();
			let mut zip = zip::ZipWriter::new(file);
			let opts = SimpleFileOptions::default();
			zip.start_file("DemoZipAddon/DemoZipAddon.toc", opts).unwrap();
			zip.write_all(b"## Interface: 120100\n## Title: Demo Zip Addon\n## Author: Tester\n").unwrap();
			zip.finish().unwrap();
		}
		let addons = dir.join("AddOns");
		let folders = crate::wow::archive::extract_archive(&zip_path, &addons).unwrap();
		assert_eq!(folders, vec!["DemoZipAddon".to_string()]);
		assert!(addons.join("DemoZipAddon/DemoZipAddon.toc").exists());
	}

	fn tempfile_dir() -> std::path::PathBuf {
		let dir = std::env::temp_dir().join(format!("wam-test-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&dir);
		std::fs::create_dir_all(&dir).unwrap();
		dir
	}
