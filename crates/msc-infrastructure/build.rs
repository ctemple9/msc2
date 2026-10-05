//! Release builds embed the tiny helper payload so older archive updaters,
//! which copy only known filenames, cannot omit newly introduced helpers.
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=MSC2_MAP_CAPTURE_HELPERS");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("map_capture_payload.rs");
    let Some(root) = env::var_os("MSC2_MAP_CAPTURE_HELPERS").map(PathBuf::from) else {
        fs::write(output, "pub static HELPERS: &[(&str,&[u8])] = &[];\n").unwrap();
        return;
    };
    let root = root
        .canonicalize()
        .expect("capture helper build directory is unavailable");
    let manifest_path = root.join("helpers.json");
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(&manifest_path).expect("capture helper manifest is missing"),
    )
    .expect("invalid capture helper manifest");
    let helpers = manifest["helpers"].as_array().expect("invalid helper set");
    assert_eq!(
        helpers.len(),
        3,
        "release payload must include Fabric, Forge and NeoForge"
    );
    let mut families = std::collections::BTreeSet::new();
    let mut files = vec![
        "helpers.json".to_string(),
        "LICENSE".into(),
        "DEPENDENCIES.md".into(),
    ];
    for helper in helpers {
        assert!(families.insert(helper["loaderFamily"].as_str().unwrap()));
        let name = helper["file"].as_str().unwrap();
        assert!(
            name.starts_with("msc-map-capture-")
                && name.ends_with(".jar")
                && !name.contains(['/', '\\'])
        );
        let path = root.join(name);
        let bytes = fs::read(&path).expect("capture helper is missing");
        assert!(bytes.len() < 8 * 1024 * 1024);
        assert_eq!(helper["bytes"].as_u64(), Some(bytes.len() as u64));
        assert_eq!(
            helper["sha256"].as_str(),
            Some(format!("{:x}", Sha256::digest(&bytes)).as_str())
        );
        files.push(name.to_string());
    }
    assert_eq!(
        families,
        std::collections::BTreeSet::from(["fabric", "forge", "neoforge"])
    );
    let mut code = "pub static HELPERS: &[(&str,&[u8])] = &[\n".to_string();
    for name in files {
        let path = root.join(&name);
        assert!(path.is_file());
        println!("cargo:rerun-if-changed={}", path.display());
        code.push_str(&format!(
            "({name:?},include_bytes!({:?})),\n",
            path.to_str().expect("build paths must be UTF-8")
        ));
    }
    code.push_str("];\n");
    fs::write(output, code).unwrap();
}
