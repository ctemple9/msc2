use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=MSC2_MSI_PAYLOAD_DIR");
    let root = std::env::var_os("MSC2_MSI_PAYLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../clients/desktop-web/src-tauri/target/package/agent")
        });
    let mut hashes = BTreeMap::new();
    for name in [
        "msc.exe",
        "vantage.exe",
        "bedrock-map.exe",
        "VANTAGE-LICENSE.txt",
    ] {
        let path = root.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = std::fs::read(&path).unwrap_or_else(|error| {
            panic!(
                "stage the Windows agent payload before building its lifecycle helper: {}: {error}",
                path.display()
            )
        });
        hashes.insert(name, format!("{:x}", Sha256::digest(&bytes)));
    }
    let json = serde_json::to_string(&hashes).unwrap();
    std::fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("payload.rs"),
        format!("const PAYLOAD_HASHES: &str = {json:?};\n"),
    )
    .unwrap();
}
