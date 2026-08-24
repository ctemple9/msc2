use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

const API_MAJOR: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageRequest {
    manifest: String,
    signature_hex: String,
    artifact_directory: PathBuf,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StageResult {
    pub state: &'static str,
    pub release_id: String,
    pub detail: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseManifest {
    release_id: String,
    platform: ReleasePlatform,
    api_major: u32,
    desktop_api_minor: u32,
    agent_api_minor_floor: u32,
    agent_api_minor_ceiling: u32,
    artifacts: Vec<ReleaseArtifact>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ReleasePlatform {
    Macos,
    Windows,
    Linux,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
enum ArtifactKind {
    Desktop,
    Agent,
    Sidecar,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseArtifact {
    kind: ArtifactKind,
    file_name: String,
    sha256: String,
}

pub fn stage(request: StageRequest, data_directory: &Path) -> Result<StageResult, String> {
    stage_with_key(request, data_directory, &release_public_key()?)
}

fn stage_with_key(
    request: StageRequest,
    data_directory: &Path,
    trusted_key: &[u8; 32],
) -> Result<StageResult, String> {
    let manifest: ReleaseManifest = serde_json::from_str(&request.manifest)
        .map_err(|error| format!("Update manifest is invalid JSON: {error}"))?;
    verify_manifest(&manifest, &request.signature_hex, trusted_key)?;

    if manifest.platform == ReleasePlatform::Linux {
        return Ok(StageResult {
            state: "package-manager",
            release_id: manifest.release_id,
            detail: "Linux updates are installed by this distribution's package manager."
                .to_string(),
        });
    }
    if manifest.platform != current_platform()? {
        return Err("This signed release is for a different desktop platform.".to_string());
    }

    let updates_directory = data_directory.join("updates");
    let staged_directory = updates_directory.join(&manifest.release_id);
    if staged_directory.exists() {
        return Err("This release is already staged; it will not be overwritten.".to_string());
    }
    fs::create_dir_all(&updates_directory)
        .map_err(|error| format!("Could not create the update staging directory: {error}"))?;
    let temporary_directory = updates_directory.join(format!(".{}.staging", manifest.release_id));
    if temporary_directory.exists() {
        return Err(
            "A previous staging attempt is still present; inspect it before retrying.".to_string(),
        );
    }
    fs::create_dir(&temporary_directory)
        .map_err(|error| format!("Could not create the temporary staging directory: {error}"))?;

    for artifact in &manifest.artifacts {
        let source = request.artifact_directory.join(&artifact.file_name);
        let destination = temporary_directory.join(&artifact.file_name);
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "Could not stage {} from {}: {error}",
                artifact.file_name,
                source.display()
            )
        })?;
        verify_artifact(&destination, artifact)?;
    }
    fs::write(temporary_directory.join("manifest.json"), &request.manifest)
        .map_err(|error| format!("Could not record the staged manifest: {error}"))?;
    fs::write(
        temporary_directory.join("manifest.sig"),
        &request.signature_hex,
    )
    .map_err(|error| format!("Could not record the staged signature: {error}"))?;
    fs::rename(&temporary_directory, &staged_directory)
        .map_err(|error| format!("Could not finalize the staged update: {error}"))?;

    Ok(StageResult {
        state: "staged",
        release_id: manifest.release_id,
        detail: "Release verified and staged. Installation still requires explicit approval."
            .to_string(),
    })
}

fn verify_manifest(
    manifest: &ReleaseManifest,
    signature_hex: &str,
    trusted_key: &[u8; 32],
) -> Result<(), String> {
    if manifest.release_id.is_empty()
        || !manifest.release_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
    {
        return Err("Update release ID is unsafe.".to_string());
    }
    if manifest.api_major != API_MAJOR {
        return Err("This release requires a different API major version.".to_string());
    }
    if manifest.desktop_api_minor < manifest.agent_api_minor_floor
        || manifest.desktop_api_minor > manifest.agent_api_minor_ceiling
    {
        return Err(
            "This release falls outside its advertised API compatibility window.".to_string(),
        );
    }
    let required = match manifest.platform {
        ReleasePlatform::Macos => &[
            ArtifactKind::Desktop,
            ArtifactKind::Agent,
            ArtifactKind::Sidecar,
        ][..],
        ReleasePlatform::Windows => &[ArtifactKind::Desktop, ArtifactKind::Agent][..],
        ReleasePlatform::Linux => &[][..],
    };
    for kind in required {
        if manifest
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == *kind)
            .count()
            != 1
        {
            return Err(
                "This release does not contain its required coordinated artifact set.".to_string(),
            );
        }
    }
    if manifest.platform == ReleasePlatform::Windows
        && manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.kind == ArtifactKind::Sidecar)
    {
        return Err("Windows native Bedrock must not be packaged as a sidecar.".to_string());
    }
    for artifact in &manifest.artifacts {
        if !safe_file_name(&artifact.file_name) || !is_sha256(&artifact.sha256) {
            return Err("This release contains an unsafe artifact identity.".to_string());
        }
    }
    let payload = serde_json::to_vec(manifest)
        .map_err(|error| format!("Could not canonicalize the update manifest: {error}"))?;
    let signature = Signature::from_slice(&hex_bytes(signature_hex, 64)?)
        .map_err(|_| "Update manifest signature is invalid.".to_string())?;
    let key = VerifyingKey::from_bytes(trusted_key)
        .map_err(|_| "The embedded release key is invalid.".to_string())?;
    key.verify(&payload, &signature)
        .map_err(|_| "Update manifest signature did not verify.".to_string())
}

fn verify_artifact(path: &Path, artifact: &ReleaseArtifact) -> Result<(), String> {
    let actual = sha256_file(path)?;
    if !actual.eq_ignore_ascii_case(&artifact.sha256) {
        return Err(format!(
            "{} did not match its signed SHA-256 digest.",
            artifact.file_name
        ));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("Could not read staged artifact {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            format!("Could not hash staged artifact {}: {error}", path.display())
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn safe_file_name(value: &str) -> bool {
    let path = Path::new(value);
    path.components().count() == 1
        && matches!(path.components().next(), Some(Component::Normal(_)))
        && !value.is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn hex_bytes(value: &str, expected_bytes: usize) -> Result<Vec<u8>, String> {
    if value.len() != expected_bytes * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Update manifest signature has an invalid encoding.".to_string());
    }
    (0..value.len())
        .step_by(2)
        .map(|offset| {
            u8::from_str_radix(&value[offset..offset + 2], 16)
                .map_err(|_| "Invalid signature.".to_string())
        })
        .collect()
}

fn release_public_key() -> Result<[u8; 32], String> {
    let value = option_env!("MSC2_RELEASE_PUBLIC_KEY_HEX")
        .ok_or_else(|| "This desktop package has no configured release-signing key.".to_string())?;
    hex_bytes(value, 32)?
        .try_into()
        .map_err(|_| "This desktop package has an invalid release-signing key.".to_string())
}

fn current_platform() -> Result<ReleasePlatform, String> {
    match std::env::consts::OS {
        "macos" => Ok(ReleasePlatform::Macos),
        "windows" => Ok(ReleasePlatform::Windows),
        _ => Err("This desktop platform uses its package manager for updates.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signed_manifest() -> (ReleaseManifest, String, [u8; 32]) {
        let manifest = ReleaseManifest {
            release_id: "2026.8.24".to_string(),
            platform: ReleasePlatform::Macos,
            api_major: 1,
            desktop_api_minor: 4,
            agent_api_minor_floor: 2,
            agent_api_minor_ceiling: 5,
            artifacts: vec![
                ReleaseArtifact {
                    kind: ArtifactKind::Desktop,
                    file_name: "MSC-2.pkg".to_string(),
                    sha256: "a".repeat(64),
                },
                ReleaseArtifact {
                    kind: ArtifactKind::Agent,
                    file_name: "agent.tar.zst".to_string(),
                    sha256: "b".repeat(64),
                },
                ReleaseArtifact {
                    kind: ArtifactKind::Sidecar,
                    file_name: "sidecar.zip".to_string(),
                    sha256: "c".repeat(64),
                },
            ],
        };
        let key = SigningKey::from_bytes(&[9; 32]);
        let signature = key.sign(&serde_json::to_vec(&manifest).expect("manifest serializes"));
        (
            manifest,
            signature
                .to_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            key.verifying_key().to_bytes(),
        )
    }

    #[test]
    fn coordinated_update_accepts_a_signed_compatible_release_set() {
        let (manifest, signature, key) = signed_manifest();
        assert!(verify_manifest(&manifest, &signature, &key).is_ok());
    }

    #[test]
    fn coordinated_update_rejects_missing_sidecar_or_skew() {
        let (mut manifest, signature, key) = signed_manifest();
        manifest.artifacts.pop();
        assert!(verify_manifest(&manifest, &signature, &key).is_err());

        let (mut manifest, signature, key) = signed_manifest();
        manifest.desktop_api_minor = 6;
        assert!(verify_manifest(&manifest, &signature, &key).is_err());
    }

    #[test]
    fn coordinated_update_rejects_untrusted_signature_and_path_escape() {
        let (mut manifest, signature, key) = signed_manifest();
        manifest.artifacts[0].file_name = "../MSC-2.pkg".to_string();
        assert!(verify_manifest(&manifest, &signature, &key).is_err());

        let (manifest, signature, _) = signed_manifest();
        assert!(verify_manifest(&manifest, &signature, &[1; 32]).is_err());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn coordinated_update_stages_only_a_verified_immutable_release_set() {
        let root = std::env::temp_dir().join(format!("msc2-update-test-{}", std::process::id()));
        let artifacts = root.join("artifacts");
        let data = root.join("data");
        fs::create_dir_all(&artifacts).expect("test artifacts directory exists");
        fs::write(artifacts.join("MSC-2.pkg"), b"desktop").expect("desktop fixture writes");
        fs::write(artifacts.join("agent.tar.zst"), b"agent").expect("agent fixture writes");
        fs::write(artifacts.join("sidecar.zip"), b"sidecar").expect("sidecar fixture writes");

        let (mut manifest, _, _) = signed_manifest();
        for artifact in &mut manifest.artifacts {
            artifact.sha256 =
                sha256_file(&artifacts.join(&artifact.file_name)).expect("fixture hashes");
        }
        let signing_key = SigningKey::from_bytes(&[4; 32]);
        let manifest_json = serde_json::to_string(&manifest).expect("manifest serializes");
        let signature = signing_key.sign(manifest_json.as_bytes());
        let request = StageRequest {
            manifest: manifest_json,
            signature_hex: signature
                .to_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            artifact_directory: artifacts,
        };

        let result = stage_with_key(request, &data, &signing_key.verifying_key().to_bytes())
            .expect("signed set stages");
        assert_eq!(result.state, "staged");
        let staged = data.join("updates/2026.8.24");
        assert_eq!(
            fs::read(staged.join("agent.tar.zst")).expect("agent staged"),
            b"agent"
        );
        assert!(staged.join("manifest.sig").is_file());
        assert!(!data.join("configuration").exists());
        let _ = fs::remove_dir_all(root);
    }
}
