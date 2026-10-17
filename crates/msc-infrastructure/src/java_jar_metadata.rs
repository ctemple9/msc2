//! Offline identity from the selected server JAR, never dependency versions.
use std::{io::Read, path::Path};

#[derive(Default, Debug, PartialEq)]
pub struct JavaJarMetadata {
    pub minecraft_version: Option<String>,
    pub build: Option<String>,
}

fn version(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        && value.bytes().next().is_some_and(|b| b.is_ascii_digit()))
    .then(|| value.to_owned())
}

/// Read only small known metadata members. Paperclip's own manifest describes
/// the launcher, while root version.json describes the Minecraft server.
/// Unknown/malformed metadata must not turn an existing JAR into a missing one.
pub fn inspect(path: &Path) -> JavaJarMetadata {
    let Some(mut jar) = crate::source_tree::open(path, false)
        .ok()
        .and_then(|file| zip::ZipArchive::new(file).ok())
    else {
        return JavaJarMetadata::default();
    };
    inspect_archive(&mut jar)
}

fn inspect_archive<R: Read + std::io::Seek>(jar: &mut zip::ZipArchive<R>) -> JavaJarMetadata {
    let mut result = JavaJarMetadata::default();
    if jar.len() > 100_000 {
        return result;
    }
    fn text<R: Read + std::io::Seek>(jar: &mut zip::ZipArchive<R>, name: &str) -> Option<String> {
        let entry = jar.by_name(name).ok()?;
        if entry.size() > 64 * 1024 {
            return None;
        }
        let mut value = String::new();
        entry.take(64 * 1024 + 1).read_to_string(&mut value).ok()?;
        (value.len() <= 64 * 1024).then_some(value)
    }
    if let Some(value) = text(jar, "version.json")
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
    {
        result.minecraft_version = value.get("id").and_then(|id| id.as_str()).and_then(version);
    }
    if let Some(manifest) = text(jar, "META-INF/MANIFEST.MF") {
        // Main manifest attributes end at the first blank line; named package
        // sections cannot supply the server's version/build.
        for line in manifest.lines().take_while(|line| !line.is_empty()) {
            if let Some(value) = line.strip_prefix("Build-Number: ") {
                result.build = value.trim().parse::<u64>().ok().map(|n| n.to_string());
            }
            if result.minecraft_version.is_none() {
                if let Some(value) = line.strip_prefix("Minecraft-Version: ") {
                    result.minecraft_version = version(value);
                } else if let Some(value) = line.strip_prefix("Implementation-Version: ") {
                    // Modern Paper: 1.21.4-123-hash. Require a numeric build;
                    // arbitrary library or loader versions are not Minecraft.
                    let mut parts = value.split('-');
                    if let (Some(mc), Some(build)) = (parts.next(), parts.next())
                        && mc.contains('.')
                        && let Ok(build) = build.parse::<u64>()
                    {
                        result.minecraft_version = version(mc);
                        result.build.get_or_insert_with(|| build.to_string());
                    }
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn inspect_entries(entries: &[(&str, &str)]) -> JavaJarMetadata {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, text) in entries {
            writer
                .start_file(
                    *name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(text.as_bytes()).unwrap();
        }
        let cursor = writer.finish().unwrap();
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        inspect_archive(&mut archive)
    }

    // Essential: the reported imported Paperclip JAR has no manifest version.
    // Tiny in-memory ZIP metadata, no server process, network, or timing dependency.
    #[test]
    fn paperclip_version_comes_from_server_metadata_not_bundled_libraries() {
        let metadata = inspect_entries(&[
            (
                "META-INF/MANIFEST.MF",
                "Manifest-Version: 1.0\r\nMain-Class: io.papermc.paperclip.Main\r\n\r\n",
            ),
            ("version.json", r#"{"id":"26.3","java_version":25}"#),
            ("META-INF/maven/library/pom.properties", "version=1.5"),
        ]);
        assert_eq!(metadata.minecraft_version.as_deref(), Some("26.3"));
        assert_eq!(metadata.build, None);
        let unknown = inspect_entries(&[("META-INF/maven/library/pom.properties", "version=1.5")]);
        assert_eq!(unknown, JavaJarMetadata::default());
    }

    #[test]
    fn server_manifest_build_and_metadata_ceiling_are_respected() {
        let metadata = inspect_entries(&[(
            "META-INF/MANIFEST.MF",
            "Implementation-Version: 26.3-147-ddb45eb\r\nBuild-Number: 147\r\n\r\n",
        )]);
        assert_eq!(metadata.minecraft_version.as_deref(), Some("26.3"));
        assert_eq!(metadata.build.as_deref(), Some("147"));
        let oversized = "x".repeat(65 * 1024);
        assert_eq!(
            inspect_entries(&[("version.json", &oversized)]),
            JavaJarMetadata::default()
        );
    }
}
