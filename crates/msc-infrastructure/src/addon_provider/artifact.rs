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

fn validate_download_url(
    provider: &str,
    url: &url::Url,
    redirect: bool,
) -> Result<(), AddonProviderError> {
    let host = url.host_str().unwrap_or("");
    let trusted = match provider {
        "modrinth" => host == "cdn.modrinth.com",
        "curseforge" => host == "forgecdn.net" || host.ends_with(".forgecdn.net"),
        _ => false,
    };
    if !trusted
        || url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|port| port != 443)
        || url.fragment().is_some()
        || (!redirect && url.query().is_some())
        || url.query_pairs().any(|(key, _)| {
            matches!(
                key.to_ascii_lowercase().as_str(),
                "api-key" | "x-api-key" | "apikey"
            )
        })
    {
        return Err(AddonProviderError::InvalidDirectUrl);
    }
    Ok(())
}
fn redirect_url(
    provider: &str,
    current: &url::Url,
    location: &str,
) -> Result<url::Url, AddonProviderError> {
    let next = current
        .join(location)
        .map_err(|_| failure("Invalid provider redirect URL"))?;
    validate_download_url(provider, &next, true)?;
    Ok(next)
}
fn needs_curseforge_key(url: &url::Url) -> bool {
    // CurseForge documents API-key authentication for edge.forgecdn.net.
    // A redirected storage endpoint never inherits a custom credential header.
    url.host_str() == Some("edge.forgecdn.net")
}

/// Write a new file in agent-owned staging. Every redirect is validated before
/// issuing its request, and credentials are recomputed for the approved auth
/// endpoint rather than blindly forwarded by the HTTP client.
pub fn acquire(
    artifact: &Artifact,
    secrets: &dyn SecretStore,
    destination: &Path,
    max_bytes: u64,
    cancelled: &dyn Fn() -> bool,
) -> Result<u64, AddonProviderError> {
    let initial_url =
        url::Url::parse(&artifact.url).map_err(|_| failure("Invalid provider download URL"))?;
    validate_download_url(artifact.provider, &initial_url, false)?;
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
            let deadline = std::time::Instant::now() + Duration::from_secs(30 * 60);
            let mut current_url = initial_url;
            let mut redirects = 0;
            let mut response = loop {
                if cancelled() {
                    return Err(failure("Artifact acquisition cancelled"));
                }
                let remaining = deadline
                    .checked_duration_since(std::time::Instant::now())
                    .ok_or_else(|| failure("Provider artifact request timed out"))?;
                let mut request = agent
                    .get(current_url.as_str())
                    .header("User-Agent", USER_AGENT);
                if artifact.provider == "curseforge" && needs_curseforge_key(&current_url) {
                    let key = super::curseforge_api_key(secrets)?;
                    request = request.header("x-api-key", &key);
                }
                let response = request
                    .config()
                    .timeout_global(Some(remaining))
                    .build()
                    .call()
                    .map_err(|_| failure("Provider artifact request failed"))?;
                let status = response.status().as_u16();
                if matches!(status, 301 | 302 | 303 | 307 | 308) {
                    if redirects >= 5 {
                        return Err(failure("Provider artifact exceeded five redirects"));
                    }
                    let location = response
                        .headers()
                        .get("location")
                        .and_then(|header| header.to_str().ok())
                        .ok_or_else(|| {
                            failure("Provider artifact redirect has no valid destination")
                        })?;
                    current_url = redirect_url(artifact.provider, &current_url, location)?;
                    redirects += 1;
                    continue;
                }
                if artifact.provider == "curseforge" {
                    msc_domain::addon_provider::ensure_curseforge_ok(status)?;
                } else {
                    msc_domain::addon_provider::ensure_modrinth_ok(status)?;
                }
                break response;
            };
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

#[cfg(test)]
mod tests {
    use super::*;
    // Essential: the 302 fix must allow CDN handoff without forwarding API
    // keys to storage or permitting a redirect to another origin. Pure URL
    // inputs, no network/files/timing; expected <1ms. Not run by default.
    #[test]
    fn artifact_redirects_preserve_origin_and_credential_boundaries() {
        let edge = url::Url::parse("https://edge.forgecdn.net/files/123/456/mod.jar").unwrap();
        assert!(needs_curseforge_key(&edge));
        let storage = redirect_url(
            "curseforge",
            &edge,
            "https://mediafilez.forgecdn.net/files/123/456/mod.jar",
        )
        .unwrap();
        assert!(!needs_curseforge_key(&storage));
        assert_eq!(
            redirect_url("curseforge", &storage, "../456/other.jar")
                .unwrap()
                .path(),
            "/files/123/456/other.jar"
        );
        for target in [
            "https://forgecdn.net.example.com/mod.jar",
            "https://example.com/mod.jar",
            "http://edge.forgecdn.net/mod.jar",
            "https://user:secret@edge.forgecdn.net/mod.jar",
            "https://edge.forgecdn.net:8443/mod.jar",
            "https://mediafilez.forgecdn.net/mod.jar?api-key=secret",
        ] {
            assert!(redirect_url("curseforge", &edge, target).is_err());
        }
        assert!(
            redirect_url(
                "modrinth",
                &url::Url::parse("https://cdn.modrinth.com/a.jar").unwrap(),
                edge.as_str()
            )
            .is_err()
        );
    }
}
