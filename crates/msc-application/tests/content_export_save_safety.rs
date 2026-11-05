//! Essential P24.18: host overwrite consent must identify existing bytes, and
//! cancelled/stale saves must leave them untouched. Native transfer coverage
//! does not exercise agent-host destinations. Tiny local files, expected <50ms.
use msc_application::content_export_saving::{self as saves, HostDestination};
use msc_domain::app_config_schema::ConfigServer;
#[test]
fn host_save_requires_current_overwrite_consent_and_preserves_cancelled_destination() {
    let root = std::env::temp_dir().join(format!("msc-export-save-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("exports")).unwrap();
    let server = ConfigServer::new(
        "save",
        "Save",
        root.to_str().unwrap(),
        "server.jar",
        1.0,
        2.0,
    );
    let path = root.join("exports/pack.zip");
    std::fs::write(&path, b"existing").unwrap();
    let inspected = saves::inspect(&server, "pack.zip".into()).unwrap();
    let mut request = HostDestination {
        relative_path: "pack.zip".into(),
        replace_sha512: None,
    };
    assert!(saves::validate(&server, &request).is_err());
    request.replace_sha512 = Some(inspected["existingSha512"].as_str().unwrap().into());
    let staged = root.join("exports/.new.zip");
    std::fs::write(&staged, b"new").unwrap();
    assert!(saves::commit(&server, &request, &staged, &|| true).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"existing");
    std::fs::write(&path, b"changed").unwrap();
    assert!(saves::commit(&server, &request, &staged, &|| false).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"changed");
    request.relative_path = "../server.jar".into();
    assert!(saves::target(&server, &request).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
