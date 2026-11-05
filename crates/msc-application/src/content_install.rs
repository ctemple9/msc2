//! Exact, bounded CurseForge dependency review. Resolution has no filesystem
//! effects: the owner can inspect the whole set before any installation starts.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use msc_domain::addon_provider::AddonProviderError;
use msc_domain::content_collection::validate_relative_path;
use msc_domain::identity::{AddOnKind, JavaServerFlavor};
use msc_infrastructure::addon_provider::{AddonTransport, java_catalog as cf};
use msc_infrastructure::secret_store::SecretStore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallFile {
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub file_name: String,
    pub byte_size: u64,
    pub sha1: String,
    pub download_url: Option<String>,
    pub distribution_allowed: Option<bool>,
    pub required_projects: Vec<String>,
    pub incompatible_projects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReview {
    pub provider: String,
    pub files: Vec<InstallFile>,
    pub warnings: Vec<String>,
}

fn invalid(message: impl Into<String>) -> AddonProviderError {
    AddonProviderError::Network(message.into())
}

fn loader(flavor: JavaServerFlavor) -> Option<u32> {
    match flavor.raw_value() {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

fn compatible(file: &cf::Release, flavor: JavaServerFlavor, game_version: &str) -> bool {
    file.is_available
        && file
            .game_versions
            .iter()
            .any(|version| version == game_version)
        && (flavor.add_on_kind() == Some(AddOnKind::Plugin)
            || file
                .game_versions
                .iter()
                .any(|version| version.eq_ignore_ascii_case(flavor.raw_value())))
}

pub(crate) fn dependency_release(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    project: u64,
    flavor: JavaServerFlavor,
    game_version: &str,
    cancelled: &dyn Fn() -> bool,
) -> Result<cf::Release, AddonProviderError> {
    let mut versions = Vec::new();
    let mut offset = 0;
    loop {
        if cancelled() {
            return Err(invalid("Content review cancelled"));
        }
        let page = cf::releases(
            transport,
            secrets,
            project,
            &cf::Query {
                text: "",
                class_id: None,
                game_version: Some(game_version),
                loader_type: loader(flavor),
                offset,
                limit: 50,
            },
        )?;
        if page
            .data
            .iter()
            .any(|file| file.mod_id != project as i64 || file.id <= 0)
        {
            return Err(invalid(
                "Dependency catalog returned a different project identity",
            ));
        }
        let received = page.data.len() as u32;
        versions.extend(
            page.data
                .into_iter()
                .filter(|file| compatible(file, flavor, game_version)),
        );
        offset += received;
        if received == 0 || offset >= page.pagination.total_count {
            break;
        }
        if offset >= 10_000 {
            return Err(invalid("Dependency catalog exceeds its review limit"));
        }
    }
    // Prefer stable releases, then newest date, then exact file ID. Do not
    // depend on provider response order or silently choose a preview first.
    versions.sort_by(|left, right| {
        (left.release_type != 1)
            .cmp(&(right.release_type != 1))
            .then_with(|| right.file_date.cmp(&left.file_date))
            .then_with(|| right.id.cmp(&left.id))
    });
    versions.into_iter().next().ok_or_else(|| {
        invalid(format!(
            "No compatible release for required CurseForge project {project}"
        ))
    })
}

pub fn review_curseforge(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    project_id: u64,
    file_id: u64,
    flavor: JavaServerFlavor,
    game_version: &str,
    cancelled: &dyn Fn() -> bool,
) -> Result<InstallReview, AddonProviderError> {
    review_curseforge_with_pins(
        transport,
        secrets,
        project_id,
        file_id,
        ReviewContext {
            flavor,
            game_version,
            installed: &BTreeMap::new(),
        },
        cancelled,
    )
}

pub struct ReviewContext<'a> {
    pub flavor: JavaServerFlavor,
    pub game_version: &'a str,
    pub installed: &'a BTreeMap<u64, u64>,
}

pub fn review_curseforge_with_pins(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    project_id: u64,
    file_id: u64,
    context: ReviewContext<'_>,
    cancelled: &dyn Fn() -> bool,
) -> Result<InstallReview, AddonProviderError> {
    let flavor = context.flavor;
    let game_version = context.game_version;
    if flavor.add_on_kind().is_none() || game_version.is_empty() {
        return Err(invalid(
            "A mod/plugin server and exact Minecraft version are required",
        ));
    }
    let root = cf::release(transport, secrets, project_id, file_id)?;
    if !compatible(&root, flavor, game_version) {
        return Err(invalid(
            "Selected release does not match the server's Minecraft version and loader",
        ));
    }
    let mut queued = VecDeque::from([(project_id, Some(root))]);
    let mut resolved = BTreeMap::<u64, InstallFile>::new();
    let mut requested = BTreeSet::from([project_id]);
    let mut warnings = Vec::new();
    while let Some((project_id, selected)) = queued.pop_front() {
        if cancelled() {
            return Err(invalid("Content review cancelled"));
        }
        if resolved.len() >= 256 {
            return Err(invalid("Dependency review exceeds 256 projects"));
        }
        let project = cf::project(transport, secrets, project_id)?;
        let expected_class = if flavor.add_on_kind() == Some(AddOnKind::Plugin) {
            5
        } else {
            6
        };
        if project.class_id != Some(expected_class) {
            return Err(invalid(
                "Dependency project is not the selected server's content kind",
            ));
        }
        let file = match selected {
            Some(file) => file,
            None => {
                let pinned = context
                    .installed
                    .get(&project_id)
                    .map(|id| cf::release(transport, secrets, project_id, *id))
                    .transpose()?;
                match pinned.filter(|file| compatible(file, flavor, game_version)) {
                    Some(file) => file,
                    None => dependency_release(
                        transport,
                        secrets,
                        project_id,
                        flavor,
                        game_version,
                        cancelled,
                    )?,
                }
            }
        };
        validate_relative_path(&file.file_name).map_err(invalid)?;
        if file.file_name.contains('/')
            || !file.file_name.to_ascii_lowercase().ends_with(".jar")
            || file.file_length == 0
            || file.file_length > 512 * 1024 * 1024
        {
            return Err(invalid("Selected content is not a bounded mod/plugin JAR"));
        }
        let sha1 = file
            .hashes
            .iter()
            .find(|hash| hash.algo == 1)
            .map(|hash| hash.value.clone())
            .filter(|hash| hash.len() == 40 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(|| invalid("Selected file has no exact supported checksum"))?;
        let mut required = Vec::new();
        let mut incompatible = Vec::new();
        for dependency in file.dependencies {
            if dependency.mod_id <= 0 {
                return Err(invalid("Invalid dependency project identity"));
            }
            let id = dependency.mod_id as u64;
            match dependency.relation_type {
                3 => {
                    required.push(id.to_string());
                    if requested.insert(id) {
                        if requested.len() > 256 {
                            return Err(invalid("Dependency review exceeds 256 projects"));
                        }
                        queued.push_back((id, None));
                    } else {
                        warnings.push(format!("Required project {id} is shared or forms a dependency cycle; included once"));
                    }
                }
                5 => incompatible.push(id.to_string()),
                _ => {}
            }
        }
        resolved.insert(
            project_id,
            InstallFile {
                project_id: project_id.to_string(),
                version_id: file.id.to_string(),
                title: project.name,
                file_name: file.file_name,
                byte_size: file.file_length,
                sha1,
                download_url: file.download_url.filter(|url| !url.is_empty()),
                distribution_allowed: project.allow_mod_distribution,
                required_projects: required,
                incompatible_projects: incompatible,
            },
        );
    }
    let mut names = BTreeSet::new();
    for file in resolved.values() {
        if !names.insert(file.file_name.to_ascii_lowercase()) {
            return Err(invalid(
                "Different required projects have colliding filenames",
            ));
        }
        for incompatible in &file.incompatible_projects {
            if requested.contains(
                &incompatible
                    .parse()
                    .map_err(|_| invalid("Invalid incompatible project"))?,
            ) {
                return Err(invalid(format!(
                    "{} conflicts with required project {incompatible}",
                    file.title
                )));
            }
        }
    }
    Ok(InstallReview {
        provider: "curseforge".into(),
        files: resolved.into_values().collect(),
        warnings,
    })
}

/// Bind the review to the exact server configuration and filesystem that was
/// inspected. Provider resolution alone cannot prove an installation is safe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredInstallReview {
    pub id: String,
    pub owner: String,
    pub server_id: String,
    pub server_dir: String,
    pub minecraft_version: String,
    pub flavor: String,
    pub created_unix: u64,
    pub collection_revision: u64,
    pub baseline: BTreeMap<String, String>,
    #[serde(default)]
    pub replacements: BTreeMap<String, String>,
    pub review: InstallReview,
}

pub fn managed_path(server_dir: &str, relative: &str) -> Result<std::path::PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/"));
    msc_infrastructure::path_safety::safe_path(
        &msc_infrastructure::fs::StdFileSystem,
        std::path::Path::new(server_dir),
        Some(relative),
        &home,
    )
    .map_err(|error| error.to_string())
}

fn review_relative(id: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(id).map_err(|_| "Invalid review identity".to_string())?;
    Ok(format!(".msc2-content/reviews/{id}.json"))
}

pub fn content_baseline(
    server: &msc_domain::app_config_schema::ConfigServer,
) -> Result<BTreeMap<String, String>, String> {
    use sha2::{Digest, Sha512};
    use std::io::Read;
    let kind = server
        .java_flavor
        .add_on_kind()
        .ok_or("Server has no add-ons")?;
    let directory = managed_path(&server.server_dir, kind.folder_name())?;
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(error.to_string()),
    };
    let mut baseline = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Non-Unicode content filename")?;
        let path = managed_path(
            &server.server_dir,
            &format!("{}/{name}", kind.folder_name()),
        )?;
        let metadata =
            std::fs::symlink_metadata(entry.path()).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err("Review cannot follow add-on symlinks".into());
        }
        if !metadata.is_file() {
            continue;
        }
        let mut input = std::fs::File::open(path).map_err(|error| error.to_string())?;
        let mut hash = Sha512::new();
        let mut buffer = [0u8; 128 * 1024];
        loop {
            let count = input.read(&mut buffer).map_err(|error| error.to_string())?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        baseline.insert(name, format!("{:x}", hash.finalize()));
        if baseline.len() > 10_000 {
            return Err("Content inventory exceeds its review limit".into());
        }
    }
    Ok(baseline)
}

pub fn store_review(record: &StoredInstallReview) -> Result<(), String> {
    let path = managed_path(&record.server_dir, &review_relative(&record.id)?)?;
    let parent = path.parent().ok_or("Missing content directory")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let encoded = serde_json::to_vec(record).map_err(|error| error.to_string())?;
    msc_infrastructure::atomic_write::atomic_write(
        &msc_infrastructure::fs::StdFileSystem,
        &path,
        &encoded,
    )
    .map_err(|error| error.to_string())
}

pub fn load_review(
    server: &msc_domain::app_config_schema::ConfigServer,
    review_id: &str,
    owner: &str,
) -> Result<StoredInstallReview, String> {
    let path = managed_path(&server.server_dir, &review_relative(review_id)?)?;
    if std::fs::metadata(&path)
        .map_err(|error| error.to_string())?
        .len()
        > 4 * 1024 * 1024
    {
        return Err("Stored review exceeds its metadata limit".into());
    }
    let record: StoredInstallReview =
        serde_json::from_slice(&std::fs::read(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    if record.owner != owner
        || record.id != review_id
        || record.server_id != server.id
        || record.server_dir != server.server_dir
        || record.flavor != server.java_flavor.raw_value()
        || record.collection_revision
            != server
                .content_collection
                .as_ref()
                .map_or(0, |collection| collection.revision)
        || server.minecraft_version.as_deref() != Some(record.minecraft_version.as_str())
        || now.saturating_sub(record.created_unix) > 30 * 60
    {
        return Err("Review identity, server configuration or expiry changed".into());
    }
    if content_baseline(server)? != record.baseline {
        return Err("Content changed since review; inspect it again".into());
    }
    Ok(record)
}

pub fn staged_file_path(
    record: &StoredInstallReview,
    file: &InstallFile,
) -> Result<std::path::PathBuf, String> {
    review_relative(&record.id)?;
    let project = file
        .project_id
        .parse::<u64>()
        .map_err(|_| "Invalid staged project identity")?;
    let version = file
        .version_id
        .parse::<u64>()
        .map_err(|_| "Invalid staged release identity")?;
    managed_path(
        &record.server_dir,
        &format!(
            ".msc2-content/install/{}/{project}-{version}.jar",
            record.id
        ),
    )
}

fn verify_file(path: &std::path::Path, expected: &InstallFile) -> Result<(), String> {
    use sha1::{Digest, Sha1};
    use std::io::Read;
    let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() != expected.byte_size {
        return Err("Staged content is not the expected file size".into());
    }
    let mut input = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hash = Sha1::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let count = input.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if !expected
        .sha1
        .eq_ignore_ascii_case(&format!("{:x}", hash.finalize()))
    {
        return Err("Staged content does not match the reviewed release checksum".into());
    }
    msc_infrastructure::archive::list_entry_names(path).map_err(|error| error.to_string())?;
    Ok(())
}

/// Stage all available downloads. Missing restricted files are returned as
/// identities for the D-027 upload checkpoint, not skipped as a successful install.
pub fn stage_files(
    record: &StoredInstallReview,
    secrets: &dyn SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<String>, String> {
    let mut pending = Vec::new();
    for file in &record.review.files {
        if cancelled() {
            return Err("Content installation cancelled".into());
        }
        let path = staged_file_path(record, file)?;
        if path.try_exists().map_err(|error| error.to_string())? {
            verify_file(&path, file)?;
            continue;
        }
        let Some(url) = file.download_url.as_ref() else {
            pending.push(file.version_id.clone());
            continue;
        };
        let artifact = msc_infrastructure::addon_provider::artifact::Artifact {
            provider: "curseforge",
            project_id: file.project_id.clone(),
            version_id: file.version_id.clone(),
            file_name: file.file_name.clone(),
            url: url.clone(),
            size: file.byte_size,
            sha1: Some(file.sha1.clone()),
            sha512: None,
        };
        msc_infrastructure::addon_provider::artifact::acquire(
            &artifact,
            secrets,
            &path,
            512 * 1024 * 1024,
            cancelled,
        )
        .map_err(|error| error.to_string())?;
        verify_file(&path, file)?;
    }
    Ok(pending)
}

/// A manual file stays outside the running server until every dependency is
/// ready. Validate exact bytes even when browser filenames have changed.
pub fn accept_manual(
    record: &StoredInstallReview,
    version_id: &str,
    staged_upload: &std::path::Path,
    uploaded_name: &str,
) -> Result<(), String> {
    let file = record
        .review
        .files
        .iter()
        .find(|file| file.version_id == version_id)
        .ok_or("No reviewed file matches this manual upload")?;
    if !crate::curseforge_manual::filename_matches(&file.file_name, uploaded_name, false) {
        return Err("Uploaded filename does not match the reviewed file".into());
    }
    verify_file(staged_upload, file)?;
    let destination = staged_file_path(record, file)?;
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    // Publish without overwriting another ready dependency or accepting bytes
    // changed while the upload was being inspected.
    let temp = destination.with_extension("manual-part");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut output = options.open(&temp).map_err(|error| error.to_string())?;
    let result = (|| {
        use std::io::Read;
        let input = std::fs::File::open(staged_upload).map_err(|error| error.to_string())?;
        std::io::copy(&mut input.take(file.byte_size + 1), &mut output)
            .map_err(|error| error.to_string())?;
        output.sync_all().map_err(|error| error.to_string())?;
        verify_file(&temp, file)?;
        std::fs::hard_link(&temp, &destination).map_err(|error| error.to_string())?;
        Ok(())
    })();
    drop(output);
    let _ = std::fs::remove_file(temp);
    result
}

fn activation_marker(server_dir: &str) -> Result<std::path::PathBuf, String> {
    managed_path(server_dir, ".msc2-content/install-active.json")
}

/// Recover a previously interrupted activation. Only files absent in the
/// reviewed baseline and still matching the exact reviewed bytes may be removed.
/// Edited files are preserved and leave recovery blocked for owner inspection.
pub fn recover_activation(
    server: &msc_domain::app_config_schema::ConfigServer,
) -> Result<(), String> {
    let marker = activation_marker(&server.server_dir)?;
    if !marker.try_exists().map_err(|error| error.to_string())? {
        return Ok(());
    }
    use msc_infrastructure::fs::FileSystem;
    let bytes = msc_infrastructure::fs::StdFileSystem
        .read_bounded(&marker, 4 * 1024 * 1024)
        .map_err(|error| error.to_string())?;
    let record: StoredInstallReview =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if record.server_id != server.id || record.server_dir != server.server_dir {
        return Err("Content recovery belongs to a different server".into());
    }
    let committed = server
        .content_collection
        .as_ref()
        .and_then(|collection| collection.extra.get("lastInstallReview"))
        .and_then(serde_json::Value::as_str)
        == Some(record.id.as_str());
    if !committed {
        let flavor = JavaServerFlavor::from_raw_value(&record.flavor)
            .ok_or("Invalid recovery server flavor")?;
        let kind = flavor
            .add_on_kind()
            .ok_or("Invalid recovery content kind")?;
        for file in &record.review.files {
            validate_relative_path(&file.file_name)?;
            if file.file_name.contains('/') {
                return Err("Unsafe recovery filename".into());
            }
            let temporary = staged_file_path(&record, file)?.with_extension("activation");
            if temporary.try_exists().map_err(|error| error.to_string())? {
                std::fs::remove_file(temporary).map_err(|error| error.to_string())?;
            }
            if record.baseline.contains_key(&file.file_name) {
                let replacing = record.replacements.get(&file.file_name) == Some(&file.file_name);
                if !replacing
                    || !rollback_path(&record, &file.file_name)?
                        .try_exists()
                        .map_err(|error| error.to_string())?
                {
                    continue;
                }
            }
            let destination = managed_path(
                &record.server_dir,
                &format!("{}/{}", kind.folder_name(), file.file_name),
            )?;
            if destination
                .try_exists()
                .map_err(|error| error.to_string())?
            {
                if record
                    .baseline
                    .get(&file.file_name)
                    .is_some_and(|hash| verify_original(&destination, hash).is_ok())
                {
                    continue;
                }
                verify_file(&destination, file).map_err(|_| {
                    format!(
                        "Content recovery preserved changed file {}; inspect it before retrying",
                        file.file_name
                    )
                })?;
                std::fs::remove_file(destination).map_err(|error| error.to_string())?;
            }
        }
    }
    if !committed {
        let flavor =
            JavaServerFlavor::from_raw_value(&record.flavor).ok_or("Invalid recovery flavor")?;
        let folder = flavor
            .add_on_kind()
            .ok_or("Invalid recovery content kind")?
            .folder_name();
        for original in record.replacements.keys() {
            let backup = rollback_path(&record, original)?;
            if !backup.try_exists().map_err(|error| error.to_string())? {
                continue;
            }
            verify_original(
                &backup,
                record
                    .baseline
                    .get(original)
                    .ok_or("Missing original identity")?,
            )?;
            let destination = managed_path(&record.server_dir, &format!("{folder}/{original}"))?;
            if destination
                .try_exists()
                .map_err(|error| error.to_string())?
            {
                verify_original(&destination, &record.baseline[original])?;
            } else {
                std::fs::hard_link(&backup, &destination).map_err(|error| error.to_string())?;
            }
            std::fs::remove_file(backup).map_err(|error| error.to_string())?;
        }
    }
    std::fs::remove_file(marker).map_err(|error| error.to_string())
}

fn rollback_path(
    record: &StoredInstallReview,
    original: &str,
) -> Result<std::path::PathBuf, String> {
    validate_relative_path(original)?;
    if original.contains('/') {
        return Err("Unsafe original filename".into());
    }
    review_relative(&record.id)?;
    managed_path(
        &record.server_dir,
        &format!(".msc2-content/install/{}/rollback/{original}", record.id),
    )
}

fn verify_original(path: &std::path::Path, expected_sha512: &str) -> Result<(), String> {
    let identity = msc_infrastructure::addon_provider::fingerprint::identify(path)?;
    if identity.sha512 != expected_sha512 {
        return Err("Original file changed; recovery preserved it for inspection".into());
    }
    Ok(())
}

/// The caller holds server operation exclusivity and ensures it is stopped.
/// persist_collection saves the new config before the recovery marker is cleared.
pub fn activate(
    server: &msc_domain::app_config_schema::ConfigServer,
    record: &StoredInstallReview,
    cancelled: &dyn Fn() -> bool,
    persist_collection: &mut dyn FnMut(
        msc_domain::content_collection::ContentCollection,
    ) -> Result<(), String>,
) -> Result<(), String> {
    use msc_domain::content_collection::{ContentAudience, ContentEntry, ContentSource};
    recover_activation(server)?;
    if record.collection_revision
        != server
            .content_collection
            .as_ref()
            .map_or(0, |collection| collection.revision)
    {
        return Err("Content metadata changed after review".into());
    }
    if content_baseline(server)? != record.baseline {
        return Err("Content changed after review".into());
    }
    let kind = server
        .java_flavor
        .add_on_kind()
        .ok_or("Server has no add-ons")?;
    let mut targets = BTreeSet::new();
    for (original, replacement) in &record.replacements {
        validate_relative_path(original)?;
        validate_relative_path(replacement)?;
        if original.contains('/')
            || replacement.contains('/')
            || !record
                .review
                .files
                .iter()
                .any(|file| file.file_name == *replacement)
            || !targets.insert(replacement)
        {
            return Err("Invalid or ambiguous replacement plan".into());
        }
        let expected = record
            .baseline
            .get(original)
            .ok_or("Original file is absent from review")?;
        verify_original(
            &managed_path(
                &record.server_dir,
                &format!("{}/{original}", kind.folder_name()),
            )?,
            expected,
        )?;
    }
    for file in &record.review.files {
        validate_relative_path(&file.file_name)?;
        if file.file_name.contains('/') {
            return Err("Unsafe reviewed filename".into());
        }
        verify_file(&staged_file_path(record, file)?, file)?;
        let destination = managed_path(
            &server.server_dir,
            &format!("{}/{}", kind.folder_name(), file.file_name),
        )?;
        if record.baseline.keys().any(|name| {
            name.eq_ignore_ascii_case(&file.file_name)
                && name != &file.file_name
                && !record.replacements.contains_key(name)
        }) {
            return Err("A reviewed filename collides with existing content".into());
        }
        if destination
            .try_exists()
            .map_err(|error| error.to_string())?
            && record.replacements.get(&file.file_name) != Some(&file.file_name)
        {
            verify_file(&destination, file)?;
        }
    }
    let marker = activation_marker(&server.server_dir)?;
    let bytes = serde_json::to_vec(record).map_err(|error| error.to_string())?;
    msc_infrastructure::atomic_write::atomic_write(
        &msc_infrastructure::fs::StdFileSystem,
        &marker,
        &bytes,
    )
    .map_err(|error| error.to_string())?;
    let result = (|| {
        for original in record.replacements.keys() {
            if cancelled() {
                return Err("Content activation cancelled".into());
            }
            let backup = rollback_path(record, original)?;
            if let Some(parent) = backup.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let original_path = managed_path(
                &server.server_dir,
                &format!("{}/{original}", kind.folder_name()),
            )?;
            // Copy-free rollback storage retains the original inode without
            // ever overwriting another backup or a later owner edit.
            std::fs::hard_link(&original_path, &backup).map_err(|error| error.to_string())?;
            verify_original(&backup, &record.baseline[original])?;
            verify_original(&original_path, &record.baseline[original])?;
            std::fs::remove_file(original_path).map_err(|error| error.to_string())?;
        }
        for file in &record.review.files {
            if cancelled() {
                return Err("Content activation cancelled".into());
            }
            if record.baseline.contains_key(&file.file_name)
                && !record.replacements.contains_key(&file.file_name)
            {
                continue;
            }
            let destination = managed_path(
                &server.server_dir,
                &format!("{}/{}", kind.folder_name(), file.file_name),
            )?;
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let source = staged_file_path(record, file)?;
            let temporary = source.with_extension("activation");
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            let mut output = options
                .open(&temporary)
                .map_err(|error| error.to_string())?;
            let copied = (|| {
                use std::io::Read;
                let input = std::fs::File::open(&source).map_err(|error| error.to_string())?;
                std::io::copy(&mut input.take(file.byte_size + 1), &mut output)
                    .map_err(|error| error.to_string())?;
                output.sync_all().map_err(|error| error.to_string())?;
                verify_file(&temporary, file)?;
                std::fs::hard_link(&temporary, &destination).map_err(|error| error.to_string())?;
                Ok::<(), String>(())
            })();
            drop(output);
            let _ = std::fs::remove_file(&temporary);
            copied?;
        }
        if cancelled() {
            return Err("Content activation cancelled".into());
        }
        let installed = content_baseline(server)?;
        let mut collection = server.content_collection.clone().unwrap_or_default();
        collection.revision = collection
            .revision
            .checked_add(1)
            .ok_or("Content revision overflow")?;
        let previous_entries = collection.current_files.clone();
        for original in record.replacements.keys() {
            collection
                .current_files
                .remove(&format!("{}/{original}", kind.folder_name()));
        }
        for file in &record.review.files {
            let path = format!("{}/{}", kind.folder_name(), file.file_name);
            let old_name = record
                .replacements
                .iter()
                .find(|(_, target)| *target == &file.file_name)
                .map(|(original, _)| original);
            let previous = previous_entries
                .get(&path)
                .or_else(|| {
                    old_name.and_then(|name| {
                        previous_entries.get(&format!("{}/{name}", kind.folder_name()))
                    })
                })
                .cloned();
            let same_bytes = previous
                .as_ref()
                .is_some_and(|entry| installed.get(&file.file_name) == Some(&entry.sha512));
            collection.current_files.insert(
                path.clone(),
                ContentEntry {
                    path,
                    sha512: installed
                        .get(&file.file_name)
                        .ok_or("Installed file missing")?
                        .clone(),
                    byte_size: file.byte_size,
                    audience: previous
                        .as_ref()
                        .map_or(ContentAudience::Server, |entry| entry.audience),
                    optional: previous.as_ref().is_some_and(|entry| entry.optional),
                    source: Some(ContentSource {
                        provider: "curseforge".into(),
                        project_id: file.project_id.clone(),
                        version_id: file.version_id.clone(),
                    }),
                    redistribution_evidence: previous
                        .as_ref()
                        .filter(|_| same_bytes)
                        .and_then(|entry| entry.redistribution_evidence.clone()),
                    classification_evidence: previous
                        .as_ref()
                        .filter(|_| same_bytes)
                        .map_or_else(Vec::new, |entry| entry.classification_evidence.clone()),
                    unresolved_reasons: previous.as_ref().filter(|_| same_bytes).map_or_else(
                        || vec!["Player requirements have not been reviewed".into()],
                        |entry| entry.unresolved_reasons.clone(),
                    ),
                    extra: {
                        let mut extra = previous.map_or_else(BTreeMap::new, |entry| entry.extra);
                        extra.insert(
                            "requiredProjects".into(),
                            serde_json::json!(file.required_projects),
                        );
                        extra.insert(
                            "incompatibleProjects".into(),
                            serde_json::json!(file.incompatible_projects),
                        );
                        extra.insert(
                            "providerDistributionAllowed".into(),
                            serde_json::json!(file.distribution_allowed),
                        );
                        extra
                    },
                },
            );
        }
        collection.extra.insert(
            "lastInstallReview".into(),
            serde_json::Value::String(record.id.clone()),
        );
        collection.validate()?;
        persist_collection(collection)?;
        Ok::<(), String>(())
    })();
    if let Err(error) = result {
        return match recover_activation(server) {
            Ok(()) => Err(error),
            Err(recovery) => Err(format!("{error}; recovery needs attention: {recovery}")),
        };
    }
    std::fs::remove_file(marker)
        .map_err(|error| format!("Content committed; recovery marker cleanup pending: {error}"))
}
