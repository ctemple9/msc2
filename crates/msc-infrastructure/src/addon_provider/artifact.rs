//! Bounded-memory artifact acquisition. The caller supplies metadata resolved
//! from a pinned provider release, never a URL supplied by the API client.
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha512};

use super::{AddonProviderError, USER_AGENT};
use crate::secret_store::SecretStore;

pub struct Artifact {
    pub provider: &'static str,
    pub project_id: String,
    pub version_id: String,
    pub file_name: String,
    pub url: String,
    pub size: u64,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

fn failure(message: impl Into<String>) -> AddonProviderError {
    AddonProviderError::Network(message.into())
}

pub fn resolve(
    transport: &dyn super::AddonTransport,
    secrets: &dyn SecretStore,
    provider: &str,
    project_id: &str,
    version_id: &str,
) -> Result<Artifact, AddonProviderError> {
    match provider {
        "curseforge" => {
            let project_id = project_id
                .parse::<u64>()
                .map_err(|_| failure("Invalid CurseForge project ID"))?;
            let file_id = version_id
                .parse::<u64>()
                .map_err(|_| failure("Invalid CurseForge file ID"))?;
            super::java_catalog::project(transport, secrets, project_id)?;
            let release = super::java_catalog::release(transport, secrets, project_id, file_id)?;
            if !release.is_available {
                return Err(failure("Selected file is unavailable"));
            }
            let url = match release.download_url {
                Some(url) if !url.is_empty() => url,
                _ => return Err(failure("Selected file requires a manual download")),
            };
            Ok(Artifact {
                provider: "curseforge",
                project_id: project_id.to_string(),
                version_id: file_id.to_string(),
                file_name: release.file_name,
                url,
                size: release.file_length,
                sha1: release
                    .hashes
                    .iter()
                    .find(|hash| hash.algo == 1)
                    .map(|hash| hash.value.clone()),
                sha512: None,
            })
        }
        "modrinth" => {
            let version = super::modrinth_version(transport, version_id)?;
            if version.project_id != project_id {
                return Err(failure("Selected version belongs to a different project"));
            }
            let file = msc_domain::addon_provider::modrinth_primary_file(&version.files)
                .ok_or_else(|| failure("Selected release has no downloadable file"))?;
            Ok(Artifact {
                provider: "modrinth",
                project_id: version.project_id,
                version_id: version.id,
                file_name: file.filename.clone(),
                url: file.url.clone(),
                size: file.size,
                sha1: file.hashes.get("sha1").cloned(),
                sha512: file.hashes.get("sha512").cloned(),
            })
        }
        _ => Err(failure("Unknown content provider")),
    }
}

/// Write a new file in an agent-controlled staging directory. No redirect
/// carries credentials, and no partial output is published under destination.
pub fn acquire(
    artifact: &Artifact,
    secrets: &dyn SecretStore,
    destination: &Path,
    max_bytes: u64,
    cancelled: &dyn Fn() -> bool,
) -> Result<u64, AddonProviderError> {
    let uri: ureq::http::Uri = artifact
        .url
        .parse()
        .map_err(|_| failure("Invalid provider download URL"))?;
    let host = uri.host().unwrap_or("").to_ascii_lowercase();
    let trusted = match artifact.provider {
        "modrinth" => host == "cdn.modrinth.com",
        "curseforge" => host == "forgecdn.net" || host.ends_with(".forgecdn.net"),
        _ => false,
    };
    if !trusted
        || uri.scheme_str() != Some("https")
        || uri
            .authority()
            .is_some_and(|value| value.as_str().contains('@'))
        || uri.port_u16().is_some_and(|port| port != 443)
        || uri.query().is_some()
    {
        return Err(AddonProviderError::InvalidDirectUrl);
    }
    msc_domain::content_collection::validate_relative_path(&artifact.file_name).map_err(failure)?;
    if artifact.file_name.contains('/') || artifact.size == 0 || artifact.size > max_bytes {
        return Err(failure("Invalid provider artifact name or byte size"));
    }
    for (hash, length) in [(&artifact.sha1, 40), (&artifact.sha512, 128)] {
        if hash.as_ref().is_some_and(|hash| {
            hash.len() != length || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err(failure("Invalid provider checksum"));
        }
    }
    if artifact.sha1.is_none() && artifact.sha512.is_none() {
        return Err(failure("Selected file has no supported integrity checksum"));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| failure("Missing staging directory"))?;
    std::fs::create_dir_all(parent).map_err(|_| failure("Cannot create staging directory"))?;
    if fs2::available_space(parent).map_err(|_| failure("Cannot determine staging capacity"))?
        < artifact.size
    {
        return Err(failure("Insufficient staging capacity"));
    }
    let temp = destination.with_extension("part");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut output = options
        .open(&temp)
        .map_err(|_| failure("Cannot reserve staging file"))?;
    let result =
        (|| {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(30 * 60)))
                .max_redirects(0)
                .https_only(true)
                .http_status_as_error(false)
                .build()
                .into();
            let mut request = agent.get(&artifact.url).header("User-Agent", USER_AGENT);
            if artifact.provider == "curseforge" {
                let key = super::curseforge_api_key(secrets)?;
                request = request.header("x-api-key", &key);
            }
            let mut response = request
                .call()
                .map_err(|_| failure("Provider artifact request failed"))?;
            let status = response.status().as_u16();
            if artifact.provider == "curseforge" {
                msc_domain::addon_provider::ensure_curseforge_ok(status)?;
            } else {
                msc_domain::addon_provider::ensure_modrinth_ok(status)?;
            }
            let mut input = response.body_mut().as_reader();
            let mut sha1 = sha1::Sha1::new();
            let mut sha512 = Sha512::new();
            let mut buffer = [0u8; 128 * 1024];
            let mut received = 0u64;
            loop {
                if cancelled() {
                    return Err(failure("Artifact acquisition cancelled"));
                }
                let count = input
                    .read(&mut buffer)
                    .map_err(|_| failure("Provider download interrupted"))?;
                if count == 0 {
                    break;
                }
                received += count as u64;
                if received > artifact.size || received > max_bytes {
                    return Err(failure("Provider file exceeded its declared size"));
                }
                sha1.update(&buffer[..count]);
                sha512.update(&buffer[..count]);
                output
                    .write_all(&buffer[..count])
                    .map_err(|_| failure("Could not write staged artifact"))?;
            }
            if received != artifact.size
                || artifact.sha1.as_ref().is_some_and(|hash| {
                    !hash.eq_ignore_ascii_case(&format!("{:x}", sha1.finalize()))
                })
                || artifact.sha512.as_ref().is_some_and(|hash| {
                    !hash.eq_ignore_ascii_case(&format!("{:x}", sha512.finalize()))
                })
            {
                return Err(failure(
                    "Provider artifact failed size or checksum verification",
                ));
            }
            output
                .sync_all()
                .map_err(|_| failure("Could not sync staged artifact"))?;
            if cancelled() {
                return Err(failure("Artifact acquisition cancelled"));
            }
            // hard_link publishes without overwriting an existing destination.
            std::fs::hard_link(&temp, destination)
                .map_err(|_| failure("Could not publish staged artifact"))?;
            Ok(received)
        })();
    drop(output);
    let _ = std::fs::remove_file(temp);
    result
}
