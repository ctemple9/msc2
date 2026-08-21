//! Port of `fixtures/plugin-source-resolution/`'s 16 cases (P8.4) against
//! `msc_domain::plugin_source` (P8.10).

mod support;

use msc_domain::plugin_source::*;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("plugin-source-resolution/{case}.json")))
}

fn detected_str(source: Option<PluginSourceType>) -> Option<&'static str> {
    source.map(PluginSourceType::as_str)
}

#[test]
fn plugin_source_resolution_detect_blank_url_returns_nil() {
    let fixture = load("detect-blank-url-returns-nil");
    let url = fixture.input["url"].as_str().unwrap();
    assert_eq!(detected_str(detect(url)), None);
    assert_eq!(fixture.expected["detected_source"].as_str(), None);
}

#[test]
fn plugin_source_resolution_detect_direct_by_http_scheme() {
    let fixture = load("detect-direct-by-http-scheme");
    let url = fixture.input["url"].as_str().unwrap();
    assert_eq!(
        detected_str(detect(url)),
        fixture.expected["detected_source"].as_str()
    );
}

#[test]
fn plugin_source_resolution_detect_direct_by_jar_suffix_without_scheme() {
    let fixture = load("detect-direct-by-jar-suffix-without-scheme");
    let url = fixture.input["url"].as_str().unwrap();
    assert_eq!(
        detected_str(detect(url)),
        fixture.expected["detected_source"].as_str()
    );
}

#[test]
fn plugin_source_resolution_detect_github_url() {
    let fixture = load("detect-github-url");
    let url = fixture.input["url"].as_str().unwrap();
    assert_eq!(
        detected_str(detect(url)),
        fixture.expected["detected_source"].as_str()
    );
}

#[test]
fn plugin_source_resolution_detect_hangar_url() {
    let fixture = load("detect-hangar-url");
    let url = fixture.input["url"].as_str().unwrap();
    assert_eq!(
        detected_str(detect(url)),
        fixture.expected["detected_source"].as_str()
    );
}

#[test]
fn plugin_source_resolution_detect_modrinth_url() {
    let fixture = load("detect-modrinth-url");
    let url = fixture.input["url"].as_str().unwrap();
    assert_eq!(
        detected_str(detect(url)),
        fixture.expected["detected_source"].as_str()
    );
}

#[test]
fn plugin_source_resolution_detect_is_case_insensitive_and_trims_whitespace() {
    let fixture = load("detect-is-case-insensitive-and-trims-whitespace");
    let url = fixture.input["url"].as_str().unwrap();
    assert_eq!(
        detected_str(detect(url)),
        fixture.expected["detected_source"].as_str()
    );
}

#[test]
fn plugin_source_resolution_detect_unrecognized_url_returns_nil() {
    let fixture = load("detect-unrecognized-url-returns-nil");
    let url = fixture.input["url"].as_str().unwrap();
    assert_eq!(detected_str(detect(url)), None);
    assert_eq!(fixture.expected["detected_source"].as_str(), None);
}

#[test]
fn plugin_source_resolution_parse_github_basic_owner_repo() {
    let fixture = load("parse-github-basic-owner-repo");
    let url = fixture.input["url"].as_str().unwrap();
    let (owner, repo) = parse_github(url).unwrap();
    assert_eq!(owner, fixture.expected["owner"].as_str().unwrap());
    assert_eq!(repo, fixture.expected["repo"].as_str().unwrap());
}

#[test]
fn plugin_source_resolution_parse_github_extra_path_segments_ignored() {
    let fixture = load("parse-github-extra-path-segments-ignored");
    let url = fixture.input["url"].as_str().unwrap();
    let (owner, repo) = parse_github(url).unwrap();
    assert_eq!(owner, fixture.expected["owner"].as_str().unwrap());
    assert_eq!(repo, fixture.expected["repo"].as_str().unwrap());
}

#[test]
fn plugin_source_resolution_parse_github_missing_repo_segment_returns_nil() {
    let fixture = load("parse-github-missing-repo-segment-returns-nil");
    let url = fixture.input["url"].as_str().unwrap();
    assert!(parse_github(url).is_none());
    assert!(fixture.expected["result"].is_null());
}

#[test]
fn plugin_source_resolution_parse_hangar_basic_author_slug() {
    let fixture = load("parse-hangar-basic-author-slug");
    let url = fixture.input["url"].as_str().unwrap();
    let (author, slug) = parse_hangar(url).unwrap();
    assert_eq!(author, fixture.expected["author"].as_str().unwrap());
    assert_eq!(slug, fixture.expected["slug"].as_str().unwrap());
}

#[test]
fn plugin_source_resolution_parse_hangar_missing_slug_returns_nil() {
    let fixture = load("parse-hangar-missing-slug-returns-nil");
    let url = fixture.input["url"].as_str().unwrap();
    assert!(parse_hangar(url).is_none());
    assert!(fixture.expected["result"].is_null());
}

#[test]
fn plugin_source_resolution_parse_modrinth_basic_slug_any_project_kind_segment() {
    let fixture = load("parse-modrinth-basic-slug-any-project-kind-segment");
    let url = fixture.input["url"].as_str().unwrap();
    let slug = parse_modrinth(url).unwrap();
    assert_eq!(slug, fixture.expected["slug"].as_str().unwrap());

    // The doc-comment-implied project-kind check never actually runs --
    // any second segment parses identically, per the fixture's own note.
    assert_eq!(
        parse_modrinth("https://modrinth.com/anything/luckperms"),
        Some("luckperms".to_string())
    );
}

#[test]
fn plugin_source_resolution_parse_modrinth_missing_slug_returns_nil() {
    let fixture = load("parse-modrinth-missing-slug-returns-nil");
    let url = fixture.input["url"].as_str().unwrap();
    assert!(parse_modrinth(url).is_none());
    assert!(fixture.expected["result"].is_null());
}

#[test]
fn plugin_source_resolution_strip_scheme_strips_http_and_https_prefix_case_insensitively() {
    // The fixture's third literal input ("github.com/a/b (no scheme)")
    // embeds a human-readable annotation inside the JSON string itself,
    // and its paired "expected" entry ("github.com/a/b") doesn't match
    // that literal -- a P8.4 fixture-authoring slip, not a real case (see
    // rolling-plan.md's P8.10 report). Exercised here against a genuinely
    // schemeless string instead, which is what the case is actually
    // meant to prove: `strip_scheme` returns it byte-for-byte unchanged.
    let fixture = load("strip-scheme-strips-http-and-https-prefix-case-insensitively");
    let inputs: Vec<String> = fixture.input["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(strip_scheme(&inputs[0]), "github.com/a/b");
    assert_eq!(strip_scheme(&inputs[1]), "github.com/a/b");

    let schemeless = "github.com/a/b";
    assert_eq!(strip_scheme(schemeless), schemeless);
}
