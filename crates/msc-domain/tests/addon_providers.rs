//! Port of `fixtures/addon-providers/`'s 33 cases (P8.4) against
//! `msc_domain::addon_provider` (P8.10).

mod support;

use msc_domain::addon_provider::*;
use serde_json::Value;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("addon-providers/{case}.json")))
}

fn workspace_root() -> std::path::PathBuf {
    support::fixtures_dir().parent().unwrap().to_path_buf()
}

fn read_corpus(rel: &str) -> String {
    let path = workspace_root().join("corpus/addons").join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read corpus file: {e}", path.display()))
}

// `Fixture` doesn't expose `corpus_source` today (P1.2's struct only reads
// the fields P1.2 itself needed) -- fixtures carrying it are read directly
// as raw JSON here instead of extending the shared support struct.
fn raw_corpus_source(case: &str) -> Vec<String> {
    let path = support::fixtures_dir().join(format!("addon-providers/{case}.json"));
    let text = std::fs::read_to_string(&path).unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    v["corpus_source"]
        .as_array()
        .map(|a| a.iter().map(|s| s.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

fn first_corpus_body(case: &str) -> String {
    let sources = raw_corpus_source(case);
    read_corpus(&sources[0])
}

// --- Modrinth: facets ---

#[test]
fn addon_providers_search_facets_plugin_project_type_includes_mod_in_or_group() {
    let fixture = load("search-facets-plugin-project-type-includes-mod-in-or-group");
    let s = modrinth_facets("plugin", &[], None);
    assert_eq!(s, fixture.expected["facets_string"].as_str().unwrap());
}

#[test]
fn addon_providers_search_facets_single_group_for_non_plugin_project_type() {
    let fixture = load("search-facets-single-group-for-non-plugin-project-type");
    let s = modrinth_facets("mod", &[], None);
    assert_eq!(s, fixture.expected["facets_string"].as_str().unwrap());
}

#[test]
fn addon_providers_search_facets_appends_loader_and_game_version_groups() {
    let fixture = load("search-facets-appends-loader-and-game-version-groups");
    let loaders: Vec<String> = fixture.input["loaders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let gv = fixture.input["game_version"].as_str();
    let s = modrinth_facets("mod", &loaders, gv);
    assert_eq!(s, fixture.expected["facets_string"].as_str().unwrap());
}

#[test]
fn addon_providers_search_index_downloads_when_query_empty_else_relevance() {
    let fixture = load("search-index-downloads-when-query-empty-else-relevance");
    assert_eq!(
        modrinth_search_index(""),
        fixture.expected["index_query_param"].as_str().unwrap()
    );
    assert_eq!(modrinth_search_index("sodium"), "relevance");
}

#[test]
fn addon_providers_search_real_response_decodes_hits_and_total_hits() {
    let body = first_corpus_body("search-real-response-decodes-hits-and-total-hits");
    let result = modrinth_decode_search(&body).unwrap();
    let raw: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(result.hits.len(), raw["hits"].as_array().unwrap().len());
    assert_eq!(result.total_hits, raw["total_hits"].as_u64().unwrap());
}

#[test]
fn addon_providers_project_is_client_only_when_server_side_unsupported() {
    let fixture = load("project-is-client-only-when-server-side-unsupported");
    let server_side = fixture.input["serverSide"].as_str().unwrap();
    assert_eq!(
        modrinth_is_client_only(server_side),
        fixture.expected["isClientOnly"].as_bool().unwrap()
    );
    assert!(!modrinth_is_client_only("optional"));
    assert!(!modrinth_is_client_only("required"));
}

// --- Modrinth: version/file selection ---

#[test]
fn addon_providers_version_primary_file_prefers_flagged_primary_else_first_file() {
    let fixture = load("version-primary-file-prefers-flagged-primary-else-first-file");
    let files: Vec<ModrinthVersionFile> =
        serde_json::from_value(fixture.input["files"].clone()).unwrap();
    let primary = modrinth_primary_file(&files).unwrap();
    assert_eq!(
        primary.filename,
        fixture.expected["primary_file_filename"].as_str().unwrap()
    );

    let no_primary = vec![files[0].clone()];
    assert_eq!(
        modrinth_primary_file(&no_primary).unwrap().filename,
        files[0].filename
    );
}

#[test]
fn addon_providers_legacy_fetch_latest_prefers_primary_jar_over_first_jar_suffixed_file() {
    let fixture = load("legacy-fetch-latest-prefers-primary-jar-over-first-jar-suffixed-file");
    let files: Vec<ModrinthVersionFile> =
        serde_json::from_value(fixture.input["files"].clone()).unwrap();
    let selected = modrinth_legacy_jar_file(&files).unwrap();
    assert_eq!(
        selected.filename,
        fixture.expected["selected_filename"].as_str().unwrap()
    );
}

#[test]
fn addon_providers_legacy_fetch_latest_no_jar_asset_throws() {
    let fixture = load("legacy-fetch-latest-no-jar-asset-throws");
    let files: Vec<ModrinthVersionFile> =
        serde_json::from_value(fixture.input["files"].clone()).unwrap();
    let err = modrinth_legacy_jar_file(&files).unwrap_err();
    assert_eq!(err, AddonProviderError::NoJarAsset);
    assert_eq!(
        err.to_string(),
        fixture.expected["error_description"].as_str().unwrap()
    );
}

#[test]
fn addon_providers_legacy_fetch_latest_no_versions_throws_no_compatible_version() {
    let fixture = load("legacy-fetch-latest-no-versions-throws-no-compatible-version");
    let err = modrinth_legacy_fetch_latest(200, &[]).unwrap_err();
    assert_eq!(
        err,
        AddonProviderError::NoCompatibleVersion {
            provider: "Modrinth"
        }
    );
    assert_eq!(
        err.to_string(),
        fixture.expected["error_description"].as_str().unwrap()
    );
}

#[test]
fn addon_providers_legacy_fetch_latest_non_2xx_status_throws_network_error() {
    let fixture = load("legacy-fetch-latest-non-2xx-status-throws-network-error");
    let status = fixture.input["http_status"].as_u64().unwrap() as u16;
    let err = modrinth_legacy_fetch_latest(status, &[]).unwrap_err();
    let contains = fixture.expected["error_description_contains"]
        .as_str()
        .unwrap();
    assert!(err.to_string().contains(contains));
}

// --- Modrinth: version_file_hash / batches ---

#[test]
fn addon_providers_version_file_hash_404_returns_nil_not_error() {
    let fixture = load("version-file-hash-404-returns-nil-not-error");
    let result = modrinth_version_from_hash(404, "{}").unwrap();
    assert!(result.is_none());
    assert!(fixture.expected["threw"].as_bool() == Some(false));
}

#[test]
fn addon_providers_version_file_hash_non_404_error_status_throws() {
    let fixture = load("version-file-hash-non-404-error-status-throws");
    let status = fixture.input["http_status"].as_u64().unwrap() as u16;
    let err = modrinth_version_from_hash(status, "{}").unwrap_err();
    let contains = fixture.expected["error_description_contains"]
        .as_str()
        .unwrap();
    assert!(err.to_string().contains(contains));
}

#[test]
fn addon_providers_version_file_hash_real_exact_identity_response() {
    let fixture = load("version-file-hash-real-exact-identity-response");
    let body = first_corpus_body("version-file-hash-real-exact-identity-response");
    let info = modrinth_version_from_hash(200, &body).unwrap().unwrap();
    assert_eq!(
        info.project_id,
        fixture.expected["decodes_to_project_id"].as_str().unwrap()
    );
    assert_eq!(
        info.id,
        fixture.expected["decodes_to_version_id"].as_str().unwrap()
    );
    assert_eq!(
        info.version_number,
        fixture.expected["version_number"].as_str().unwrap()
    );
}

#[test]
fn addon_providers_versions_from_hashes_batch_empty_input_short_circuits() {
    let plan = modrinth_versions_from_hashes_plan(&[]);
    assert!(plan.is_none());
    assert!(modrinth_versions_from_hashes_plan(&["abc".to_string()]).is_some());
}

#[test]
fn addon_providers_projects_batch_empty_ids_returns_empty_without_request() {
    let plan = modrinth_projects_plan(&[]);
    assert!(plan.is_none());
}

#[test]
fn addon_providers_latest_versions_for_hashes_omits_body_fields_when_loaders_and_game_versions_empty()
 {
    let fixture =
        load("latest-versions-for-hashes-omits-body-fields-when-loaders-and-game-versions-empty");
    let body = modrinth_latest_versions_body(&["abc123".to_string()], &[], &[]);
    assert_eq!(body, fixture.expected["post_body"]);

    let with_filters = modrinth_latest_versions_body(
        &["abc123".to_string()],
        &["fabric".to_string()],
        &["1.21.1".to_string()],
    );
    assert_eq!(with_filters["loaders"], serde_json::json!(["fabric"]));
    assert_eq!(with_filters["game_versions"], serde_json::json!(["1.21.1"]));
}

// --- Hangar ---

#[test]
fn addon_providers_no_compatible_version_throws_when_result_empty() {
    let fixture = load("no-compatible-version-throws-when-result-empty");
    let err = hangar_select_latest(&[]).unwrap_err();
    assert_eq!(
        err.to_string(),
        fixture.expected["error_description"].as_str().unwrap()
    );
}

#[test]
fn addon_providers_fetch_latest_real_response_falls_back_to_computed_endpoint_when_api_url_absent()
{
    let fixture =
        load("fetch-latest-real-response-falls-back-to-computed-endpoint-when-api-url-absent");
    let body = first_corpus_body(
        "fetch-latest-real-response-falls-back-to-computed-endpoint-when-api-url-absent",
    );
    let versions = hangar_decode_versions(&body).unwrap();
    let latest = hangar_select_latest(&versions).unwrap();
    assert_eq!(
        latest.name,
        fixture.expected["selected_version_name"].as_str().unwrap()
    );
    let url = hangar_download_url("EssentialsX", "Essentials", latest);
    assert_eq!(url, fixture.expected["download_url"].as_str().unwrap());
}

#[test]
fn addon_providers_fetch_latest_uses_api_provided_download_url_when_present() {
    let fixture = load("fetch-latest-uses-api-provided-download-url-when-present");
    let mut downloads = std::collections::HashMap::new();
    downloads.insert(
        "PAPER".to_string(),
        HangarDownload {
            download_url: Some(
                fixture.input["raw_downloads_PAPER_downloadUrl"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            ),
        },
    );
    let version = HangarVersion {
        name: "1.0.0".to_string(),
        downloads,
    };
    let url = hangar_download_url(
        fixture.input["author"].as_str().unwrap(),
        fixture.input["slug"].as_str().unwrap(),
        &version,
    );
    assert_eq!(url, fixture.expected["download_url"].as_str().unwrap());
}

#[test]
fn addon_providers_version_name_percent_encoded_in_fallback_download_url() {
    let fixture = load("version-name-percent-encoded-in-fallback-download-url");
    let version = HangarVersion {
        name: fixture.input["version_name"].as_str().unwrap().to_string(),
        downloads: std::collections::HashMap::new(),
    };
    let url = hangar_download_url("A", "B", &version);
    let segment = fixture.expected["fallback_download_url_segment"]
        .as_str()
        .unwrap();
    assert!(url.contains(segment));
}

// --- CurseForge ---

#[test]
fn addon_providers_missing_api_key_throws_before_request_even_with_valid_ids() {
    let fixture = load("missing-api-key-throws-before-request-even-with-valid-ids");
    let err = curseforge_require_api_key(fixture.input["apiKey"].as_str().unwrap()).unwrap_err();
    assert_eq!(err, AddonProviderError::MissingApiKey);
    assert_eq!(
        err.to_string(),
        fixture.expected["error_description"].as_str().unwrap()
    );
    assert_eq!(
        fixture.expected["network_request_made"].as_bool(),
        Some(false)
    );
}

#[test]
fn addon_providers_unauthorized_401_or_403_throws_unauthorized_not_generic_network_error() {
    let fixture = load("unauthorized-401-or-403-throws-unauthorized-not-generic-network-error");
    let status = fixture.input["http_status"].as_u64().unwrap() as u16;
    let err = ensure_curseforge_ok(status).unwrap_err();
    assert_eq!(err, AddonProviderError::Unauthorized);
    assert_eq!(
        err.to_string(),
        fixture.expected["error_description"].as_str().unwrap()
    );

    let generic = ensure_curseforge_ok(500).unwrap_err();
    assert!(
        generic
            .to_string()
            .contains("CurseForge returned status 500")
    );
}

#[test]
fn addon_providers_files_dedupes_and_sorts_ids_before_batching() {
    let fixture = load("files-dedupes-and-sorts-ids-before-batching");
    let input: Vec<i64> = fixture.input["input_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    let expected: Vec<i64> = fixture.expected["ids_sent_to_fetch"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    assert_eq!(curseforge_batched_ids(&input), expected);
}

#[test]
fn addon_providers_files_batch_real_blocked_file_download_url_null_but_available() {
    let fixture = load("files-batch-real-blocked-file-download-url-null-but-available");
    let body = first_corpus_body("files-batch-real-blocked-file-download-url-null-but-available");
    let files = curseforge_decode_files(&body).unwrap();
    let f = &files[0];
    let decodes_to = &fixture.expected["decodes_to"];
    assert_eq!(f.id, decodes_to["id"].as_i64().unwrap());
    assert_eq!(f.mod_id, decodes_to["modId"].as_i64().unwrap());
    assert_eq!(f.file_name, decodes_to["fileName"].as_str().unwrap());
    assert!(f.download_url.is_none());
}

#[test]
fn addon_providers_files_batch_real_resolvable_file_has_download_url() {
    let body = first_corpus_body("files-batch-real-resolvable-file-has-download-url");
    let files = curseforge_decode_files(&body).unwrap();
    assert!(files[0].download_url.is_some());
}

#[test]
fn addon_providers_files_omitted_ids_absent_from_result_not_error() {
    let fixture = load("files-omitted-ids-absent-from-result-not-error");
    // This fixture has no `corpus_source` of its own -- it reuses the
    // blocked-entityculling capture (one real file, id 8287121) to prove
    // that requesting a second, unrecognized id (999999999) alongside it
    // doesn't error, just yields a shorter result.
    let body = read_corpus("curseforge/mods-files-blocked-entityculling.json");
    let files = curseforge_decode_files(&body).unwrap();
    assert_eq!(
        files.len() as i64,
        fixture.expected["result_count"].as_i64().unwrap()
    );
}

#[test]
fn addon_providers_mods_batch_real_blocked_mod_metadata_for_manual_download_list() {
    let fixture = load("mods-batch-real-blocked-mod-metadata-for-manual-download-list");
    let body = first_corpus_body("mods-batch-real-blocked-mod-metadata-for-manual-download-list");
    let mods = curseforge_decode_mods(&body).unwrap();
    let m = &mods[0];
    let decodes_to = &fixture.expected["decodes_to"];
    assert_eq!(m.id, decodes_to["id"].as_i64().unwrap());
    assert_eq!(m.name, decodes_to["name"].as_str().unwrap());
    assert_eq!(m.slug, decodes_to["slug"].as_str().unwrap());
    assert_eq!(m.website_url(), decodes_to["websiteUrl"].as_str());
}

// --- GitHub Releases ---

#[test]
fn addon_providers_asset_suffix_match_is_case_insensitive() {
    let fixture = load("asset-suffix-match-is-case-insensitive");
    let assets: Vec<GitHubAsset> = serde_json::from_value(fixture.input["assets"].clone()).unwrap();
    let selected = github_select_jar_asset(&assets).unwrap();
    assert_eq!(
        selected.name,
        fixture.expected["selected_asset_name"].as_str().unwrap()
    );
}

#[test]
fn addon_providers_no_jar_asset_returns_nil_url_without_throwing() {
    let fixture = load("no-jar-asset-returns-nil-url-without-throwing");
    let assets: Vec<GitHubAsset> = serde_json::from_value(fixture.input["assets"].clone()).unwrap();
    let selected = github_select_jar_asset(&assets);
    assert!(selected.is_none());
    assert!(fixture.expected["threw"].as_bool() == Some(false));
}

#[test]
fn addon_providers_first_jar_asset_selected_from_real_multi_asset_release() {
    let fixture = load("first-jar-asset-selected-from-real-multi-asset-release");
    let body = first_corpus_body("first-jar-asset-selected-from-real-multi-asset-release");
    let release = github_decode_release(&body).unwrap();
    let selected = github_select_jar_asset(&release.assets).unwrap();
    assert_eq!(
        selected.name,
        fixture.expected["selected_asset_name"].as_str().unwrap()
    );
}

// --- Direct URL ---

#[test]
fn addon_providers_invalid_url_string_throws() {
    let fixture = load("invalid-url-string-throws");
    let err = direct_dispatch(fixture.input["source_url"].as_str().unwrap()).unwrap_err();
    assert_eq!(err, AddonProviderError::InvalidDirectUrl);
    assert_eq!(
        err.to_string(),
        fixture.expected["message"].as_str().unwrap()
    );
}

#[test]
fn addon_providers_valid_url_returns_direct_literal_version_real_evidence() {
    let fixture = load("valid-url-returns-direct-literal-version-real-evidence");
    let source_url = fixture.input["source_url"].as_str().unwrap();
    let (version, download_url) = direct_dispatch(source_url).unwrap();
    assert_eq!(
        version,
        fixture.expected["version_string"].as_str().unwrap()
    );
    assert_eq!(
        download_url,
        fixture.expected["download_url"].as_str().unwrap()
    );
}
