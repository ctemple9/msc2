//! Port of the inspect-side fixtures in `fixtures/transfer-package/`
//! (P5.12/P5.14): `inspectTransferPackage(at:)`. Also covers the
//! path-traversal / absolute-path / symlink-escape rejection the format
//! doc calls out as new Rust-side hardening with no MSC 1 fixture of its
//! own (MSC 1's real implementation shells out to `/usr/bin/unzip -o`,
//! which has none of this).

use msc_application::transfer::{
    TransferExportRequest, TransferExportServerInput, TransferInspectError, export_server_transfer,
    inspect_transfer_package,
};
use msc_domain::app_config_schema::ConfigServer;
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn load_fixture(case: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/transfer-package")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()))
}

struct TempPaths {
    package_path: PathBuf,
    staging_root: PathBuf,
}

impl TempPaths {
    fn new(name: &str) -> Self {
        let base = std::env::temp_dir().join(format!(
            "msc2-transfer-inspect-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        Self {
            package_path: base.join("package.msctransfer"),
            staging_root: base.join("staging"),
        }
    }
}

impl Drop for TempPaths {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.package_path.parent().unwrap());
    }
}

fn write_package(path: &Path, entries: &[(&str, &[u8], bool)]) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    for (name, bytes, is_symlink) in entries {
        if *is_symlink {
            // `unix_permissions` on `start_file` only carries the
            // permission bits, not the file-type nibble — proved against
            // the real crate before writing this: `start_file` with
            // `unix_permissions(0o120777)` reads back as a *regular* file
            // with mode 0o100777, not a symlink. `add_symlink` is the
            // crate's real API for a symlink entry.
            zip.add_symlink(
                *name,
                std::str::from_utf8(bytes).unwrap(),
                SimpleFileOptions::default(),
            )
            .unwrap();
        } else {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
    }
    zip.finish().unwrap();
}

#[test]
fn newer_format_version_is_rejected_and_staging_removed() {
    let fixture = load_fixture("newer-unsupported-format-rejected");
    let paths = TempPaths::new("newer-format");
    let manifest_bytes = serde_json::to_vec_pretty(&fixture["input"]["manifest"]).unwrap();
    write_package(
        &paths.package_path,
        &[("manifest.json", &manifest_bytes, false)],
    );

    let err = inspect_transfer_package(&paths.package_path, &paths.staging_root, &[], &[])
        .expect_err("newer formatVersion must be rejected");

    assert_eq!(
        err,
        TransferInspectError::UnsupportedFormatVersion { found: 3 }
    );
    assert_eq!(
        err.to_string(),
        fixture["expected"]["error_message"].as_str().unwrap()
    );
    assert!(
        !paths.staging_root.exists(),
        "staging must be removed on failure"
    );
}

#[test]
fn older_format_version_is_not_rejected() {
    let paths = TempPaths::new("older-format");
    let manifest = serde_json::json!({
        "formatVersion": 1,
        "appConfigVersion": 1,
        "createdAt": "2020-01-01T00:00:00Z",
        "sourceMachineName": "Old Mac",
        "servers": []
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    write_package(
        &paths.package_path,
        &[("manifest.json", &manifest_bytes, false)],
    );

    let inspection = inspect_transfer_package(&paths.package_path, &paths.staging_root, &[], &[])
        .expect("formatVersion 1 <= 2 is accepted");
    assert_eq!(inspection.manifest.format_version, 1);
    assert!(paths.staging_root.exists());
}

#[test]
fn missing_manifest_json_is_rejected_and_staging_removed() {
    let paths = TempPaths::new("missing-manifest");
    write_package(
        &paths.package_path,
        &[("servers/readme.txt", b"not a manifest", false)],
    );

    let err = inspect_transfer_package(&paths.package_path, &paths.staging_root, &[], &[])
        .expect_err("a package with no manifest.json must fail");

    assert_eq!(err, TransferInspectError::MissingManifest);
    assert!(!paths.staging_root.exists());
}

#[test]
fn malformed_manifest_json_is_rejected_and_staging_removed() {
    let paths = TempPaths::new("malformed-manifest");
    write_package(
        &paths.package_path,
        &[("manifest.json", b"{not valid json", false)],
    );

    let err = inspect_transfer_package(&paths.package_path, &paths.staging_root, &[], &[])
        .expect_err("malformed manifest.json must fail");

    assert!(matches!(err, TransferInspectError::Decode(_)));
    assert!(!paths.staging_root.exists());
}

#[test]
fn path_traversal_entry_is_rejected_and_staging_removed() {
    let paths = TempPaths::new("path-traversal");
    write_package(
        &paths.package_path,
        &[
            ("manifest.json", b"{}", false),
            ("../escaped.txt", b"evil", false),
        ],
    );

    let err = inspect_transfer_package(&paths.package_path, &paths.staging_root, &[], &[])
        .expect_err("a path-traversal entry must be rejected");

    assert!(matches!(err, TransferInspectError::UnsafeEntry(_)));
    assert!(!paths.staging_root.exists());
    assert!(
        !paths
            .staging_root
            .parent()
            .unwrap()
            .join("escaped.txt")
            .exists()
    );
}

#[test]
fn absolute_path_entry_is_rejected_and_staging_removed() {
    let paths = TempPaths::new("absolute-path");
    write_package(
        &paths.package_path,
        &[
            ("manifest.json", b"{}", false),
            ("/etc/escaped.txt", b"evil", false),
        ],
    );

    let err = inspect_transfer_package(&paths.package_path, &paths.staging_root, &[], &[])
        .expect_err("an absolute-path entry must be rejected, not silently relativized");

    assert!(matches!(err, TransferInspectError::UnsafeEntry(_)));
    assert!(!paths.staging_root.exists());
}

#[test]
fn symlink_entry_is_rejected_and_staging_removed() {
    let paths = TempPaths::new("symlink-escape");
    write_package(
        &paths.package_path,
        &[
            ("manifest.json", b"{}", false),
            ("servers/smp/link", b"/etc", true),
        ],
    );

    let err = inspect_transfer_package(&paths.package_path, &paths.staging_root, &[], &[])
        .expect_err("a symlink entry must be rejected");

    assert!(matches!(err, TransferInspectError::UnsafeEntry(_)));
    assert!(!paths.staging_root.exists());
}

/// End-to-end with P5.13's export: exports a real Bedrock server whose
/// `bedrockPort` collides with a caller-supplied "already in use" port,
/// then inspects the resulting package and checks the exact conflict
/// string against `bedrock-worlds-export.json`'s `inspect_port_conflict`.
#[test]
fn bedrock_port_conflict_detected_end_to_end() {
    let fixture = load_fixture("bedrock-worlds-export");
    let export_dir = std::env::temp_dir().join(format!(
        "msc2-transfer-inspect-export-src-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&export_dir);
    std::fs::create_dir_all(export_dir.join("worlds")).unwrap();

    let mut server = ConfigServer::decode(&fixture["input"]["server"]).unwrap();
    server.server_dir = export_dir.to_string_lossy().into_owned();

    let request = TransferExportRequest {
        servers: vec![TransferExportServerInput {
            server,
            paper_mc_version: None,
            paper_build: None,
        }],
        created_at: "2026-01-01T00:00:00Z".to_string(),
        source_machine_name: "Test Mac".to_string(),
        app_config_version: 1,
    };

    let paths = TempPaths::new("bedrock-conflict");
    {
        let file = std::fs::File::create(&paths.package_path).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        export_server_transfer(&request, &mut writer).expect("export succeeds");
    }

    let existing_bedrock_ports = fixture["input"]["inspect_context"]["existing_bedrock_ports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect::<Vec<_>>();

    let inspection = inspect_transfer_package(
        &paths.package_path,
        &paths.staging_root,
        &[],
        &existing_bedrock_ports,
    )
    .expect("a well-formed package inspects successfully");

    assert_eq!(inspection.conflicts.len(), 1);
    assert_eq!(
        inspection.conflicts[0].message,
        fixture["expected"]["inspect_port_conflict"]["detail"]
            .as_str()
            .unwrap()
    );

    // The manifest and its bundled files really landed on disk under
    // staging_root, not just in memory — the "extract into a temporary
    // staging root" half of this step.
    let folder = &inspection.manifest.servers[0].folder_name;
    assert!(inspection.staging_root.join("manifest.json").is_file());
    assert!(
        inspection
            .staging_root
            .join(format!("servers/{folder}/worlds"))
            .is_dir()
    );

    let _ = std::fs::remove_dir_all(&export_dir);
}
