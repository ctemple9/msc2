//! Named, scriptable adapters for the shared content contract. Complex per-file
//! choices live in bounded JSON request files rather than implicit CLI defaults.
use super::*;
use clap::ValueEnum;
use serde_json::{Value, json};
#[derive(Debug, Clone, ValueEnum)]
pub enum Provider {
    Modrinth,
    Curseforge,
}
impl Provider {
    fn name(&self) -> &str {
        match self {
            Self::Modrinth => "modrinth",
            Self::Curseforge => "curseforge",
        }
    }
}
#[derive(Debug, Clone, ValueEnum)]
pub enum ReadKind {
    Collection,
    Identify,
    Updates,
    Review,
    PackReview,
    RetainedPack,
    Publications,
    Publication,
    PublicationReview,
    Datapacks,
    MrpackEligibility,
    ExportDestination,
}
#[derive(Debug, Clone, ValueEnum)]
pub enum Action {
    InstallReview,
    Install,
    InstallManualAdmit,
    InstallManualComplete,
    Source,
    UpdateReview,
    Review,
    Apply,
    Recover,
    PlayerAdmit,
    PackUpdateReview,
    PackUpdate,
    PackReplacementReview,
    PackReplacement,
    PackManualAdmit,
    PackManualComplete,
    RetainedRetry,
    RetainedManualAdmit,
    RetainedManualComplete,
    PublicationReview,
    Publish,
}
impl Action {
    fn suffix(&self, id: Option<&str>) -> Result<String, CliError> {
        let child = |prefix: &str, tail: &str| -> Result<String, CliError> {
            let id = id.ok_or_else(|| CliError::usage("--id is required for this action"))?;
            Ok(format!("{prefix}/{}/{tail}", encode_uri_component(id)))
        };
        Ok(match self {
            Self::InstallReview => "install-review".into(),
            Self::Install => "install".into(),
            Self::InstallManualAdmit => "install-manual/admit".into(),
            Self::InstallManualComplete => "install-manual/complete".into(),
            Self::Source => "source".into(),
            Self::UpdateReview => "update-review".into(),
            Self::Review => "review".into(),
            Self::Apply => "apply".into(),
            Self::Recover => "recovery-review".into(),
            Self::PlayerAdmit => "player-files/admit".into(),
            Self::PackUpdateReview => "pack-update-review".into(),
            Self::PackUpdate => "pack-update".into(),
            Self::PackReplacementReview => "pack-replacement-review".into(),
            Self::PackReplacement => "pack-replacement".into(),
            Self::PackManualAdmit => child("pack-reviews", "manual/admit")?,
            Self::PackManualComplete => child("pack-reviews", "manual/complete")?,
            Self::RetainedRetry => child("retained-packs", "retry")?,
            Self::RetainedManualAdmit => child("retained-packs", "manual/admit")?,
            Self::RetainedManualComplete => child("retained-packs", "manual/complete")?,
            Self::PublicationReview => "publication-reviews".into(),
            Self::Publish => "publications".into(),
        })
    }
    fn capability(&self) -> &str {
        match self {
            Self::InstallReview
            | Self::Install
            | Self::InstallManualAdmit
            | Self::InstallManualComplete => "reviewedInstalls",
            Self::Source => "sourceIdentification",
            Self::UpdateReview => "reviewedUpdates",
            Self::Review | Self::Apply | Self::PlayerAdmit => "playerContentManagement",
            Self::Recover => "playerContentRecovery",
            Self::PackUpdateReview
            | Self::PackUpdate
            | Self::PackManualAdmit
            | Self::PackManualComplete => "reviewedPackUpdates",
            Self::PackReplacementReview | Self::PackReplacement => "reviewedPackReplacement",
            Self::RetainedRetry | Self::RetainedManualAdmit | Self::RetainedManualComplete => {
                "retainedPackRecovery"
            }
            Self::PublicationReview | Self::Publish => "contentPublications",
        }
    }
}
#[derive(Debug, Clone, Subcommand)]
pub enum ContentCommand {
    /// Search either provider; modpacks show all setups for compatibility review.
    Search {
        query: String,
        #[arg(long, value_enum)]
        provider: Provider,
        #[arg(long, default_value = "mod")]
        kind: String,
        #[arg(long)]
        game_version: Option<String>,
        #[arg(long)]
        loader: Option<String>,
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    /// Read project metadata and exact releases before acquisition/install.
    Inspect {
        project_id: String,
        #[arg(long, value_enum)]
        provider: Provider,
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    /// Acquire the exact provider pack into purpose-bound staging.
    Acquire {
        project_id: String,
        version_id: String,
        #[arg(long, value_enum)]
        provider: Provider,
        #[arg(long, required = true)]
        confirm: bool,
        #[arg(long)]
        no_wait: bool,
    },
    /// Read content/review/publication metadata for an explicit server ID.
    Get {
        #[arg(long)]
        server: String,
        #[arg(long, value_enum)]
        kind: ReadKind,
        #[arg(long)]
        id: Option<String>,
        /// Relative export filename when kind is export-destination.
        #[arg(long)]
        path: Option<String>,
        #[arg(long, default_value = "players")]
        audience: String,
    },
    /// Submit explicit per-item choices from a JSON file (or - for stdin).
    Request {
        #[arg(long)]
        server: String,
        #[arg(long, value_enum)]
        action: Action,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        file: PathBuf,
        #[arg(long, required = true)]
        confirm: bool,
        #[arg(long)]
        no_wait: bool,
        /// Upload a local file to the returned purpose-bound admission.
        #[arg(long)]
        upload: Option<PathBuf>,
    },
    /// Upload a pack archive for a later update/replacement/recovery review.
    StagePack { path: PathBuf },
    /// Export an existing frozen publication onto this agent host.
    Export {
        #[arg(long)]
        server: String,
        #[arg(long)]
        publication: String,
        #[arg(long, default_value = "players")]
        audience: String,
        /// ZIP path relative to this server's exports directory.
        #[arg(long)]
        destination: String,
        #[arg(long)]
        include_mrpack: bool,
        #[arg(long)]
        allow_incomplete: bool,
        /// Existing exact SHA512 returned by get --kind export-destination.
        #[arg(long)]
        replace_sha512: Option<String>,
        #[arg(long, required = true)]
        confirm: bool,
        #[arg(long)]
        no_wait: bool,
    },
}
fn scoped(server: &str, suffix: &str) -> String {
    format!(
        "/v1/servers/{}/content/{suffix}",
        encode_uri_component(server)
    )
}
async fn capability(client: &ApiClient, name: &str) -> Result<(), CliError> {
    let caps: Value = client.get_json("/v1/capabilities").await?;
    let catalog = &caps["contentCatalog"];
    let supported = catalog[name].as_bool() == Some(true)
        || catalog[name]
            .as_array()
            .is_some_and(|values| !values.is_empty());
    if !supported {
        return Err(CliError::usage(format!(
            "This agent does not advertise {name}; update it before using this command"
        )));
    }
    Ok(())
}
async fn output(
    client: &ApiClient,
    value: Value,
    _json_mode: bool,
    no_wait: bool,
) -> Result<(), CliError> {
    if let Some(id) = value["operationId"].as_str() {
        finish_operation(client, true, no_wait, Some(id.into()), "content action").await
    } else {
        print_json(&value)
    }
}
async fn request_file(path: &Path) -> Result<Value, CliError> {
    use std::io::Read;
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut bytes = Vec::new();
        let input: Box<dyn Read> = if path == Path::new("-") {
            Box::new(std::io::stdin())
        } else {
            Box::new(std::fs::File::open(path).map_err(|e| CliError::usage(e.to_string()))?)
        };
        input
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| CliError::usage(e.to_string()))?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(CliError::usage("Content request exceeds 8 MiB"));
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|e| CliError::usage(e.to_string()))?;
        if !value.is_object() {
            return Err(CliError::usage("Content request must be a JSON object"));
        }
        Ok(value)
    })
    .await
    .map_err(|e| CliError::internal(e.to_string()))?
}
pub(super) async fn run(common: CommonArgs, command: ContentCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
    match command {
        ContentCommand::Search {
            query,
            provider,
            kind,
            game_version,
            loader,
            offset,
        } => {
            let mut path = format!(
                "/v1/catalog/{}/search?q={}&kind={}&offset={offset}",
                provider.name(),
                encode_uri_component(&query),
                encode_uri_component(&kind)
            );
            for (key, value) in [("gameVersion", game_version), ("loader", loader)] {
                if let Some(value) = value {
                    path.push_str(&format!("&{key}={}", encode_uri_component(&value)));
                }
            }
            print_json(&client.get_json::<Value>(&path).await?)
        }
        ContentCommand::Inspect {
            project_id,
            provider,
            offset,
        } => {
            let base = if matches!(provider, Provider::Curseforge) {
                "/v1/catalog/curseforge/projects"
            } else {
                "/v1/catalog/projects"
            };
            let path = format!("{base}/{}", encode_uri_component(&project_id));
            let project: Value = client.get_json(&path).await?;
            let versions: Value = client
                .get_json(&format!("{path}/versions?offset={offset}"))
                .await?;
            print_json(&json!({"provider":provider.name(),"project":project,"versions":versions}))
        }
        ContentCommand::Acquire {
            project_id,
            version_id,
            provider,
            confirm: _,
            no_wait,
        } => {
            capability(&client, "packAcquisition").await?;
            let value:Value=client.post_json("/v1/catalog/modpacks/acquire",&json!({"provider":provider.name(),"projectId":project_id,"versionId":version_id})).await?;
            output(&client, value, common.json, no_wait).await
        }
        ContentCommand::StagePack { path } => {
            capability(&client, "packAcquisition").await?;
            let id = stage_file_upload(
                &client,
                &path,
                StagedUploadPurposeDto::ModpackArchive,
                None,
                None,
                None,
            )
            .await?;
            print_json(&json!({"stagedUploadId":id}))
        }
        ContentCommand::Get {
            server,
            kind,
            id,
            path,
            audience,
        } => {
            let need_id = || {
                id.as_deref()
                    .map(encode_uri_component)
                    .ok_or_else(|| CliError::usage("--id is required for this resource"))
            };
            let suffix = match kind {
                ReadKind::Collection => "collection".into(),
                ReadKind::Identify => "identify".into(),
                ReadKind::Updates => "updates".into(),
                ReadKind::Review => format!("reviews/{}", need_id()?),
                ReadKind::PackReview => format!("pack-reviews/{}", need_id()?),
                ReadKind::RetainedPack => format!("retained-packs/{}", need_id()?),
                ReadKind::Publications => "publications".into(),
                ReadKind::Publication => format!("publications/{}", need_id()?),
                ReadKind::PublicationReview => format!("publication-reviews/{}", need_id()?),
                ReadKind::Datapacks => format!("publication-datapacks/{}", need_id()?),
                ReadKind::MrpackEligibility => format!(
                    "publications/{}/mrpack-eligibility?audience={}",
                    need_id()?,
                    encode_uri_component(&audience)
                ),
                ReadKind::ExportDestination => format!(
                    "export-destination?path={}",
                    encode_uri_component(
                        &path.ok_or_else(|| CliError::usage("--path is required"))?
                    )
                ),
            };
            print_json(&client.get_json::<Value>(&scoped(&server, &suffix)).await?)
        }
        ContentCommand::Request {
            server,
            action,
            id,
            file,
            confirm: _,
            no_wait,
            upload,
        } => {
            capability(&client, action.capability()).await?;
            if upload.is_some()
                && !matches!(
                    action,
                    Action::InstallManualAdmit
                        | Action::PlayerAdmit
                        | Action::PackManualAdmit
                        | Action::RetainedManualAdmit
                )
            {
                return Err(CliError::usage(
                    "--upload is only valid for an admission action",
                ));
            }
            let body = request_file(&file).await?;
            let value: Value = client
                .post_json(&scoped(&server, &action.suffix(id.as_deref())?), &body)
                .await?;
            if let Some(path) = upload {
                let id = value["stagedUploadId"].as_str().ok_or_else(|| {
                    CliError::internal("Admission did not return a staging identity")
                })?;
                let route = value["uploadPath"].as_str().ok_or_else(|| {
                    CliError::internal("Admission did not return an upload route")
                })?;
                if uuid::Uuid::parse_str(id).is_err() || route != format!("/v1/staged-uploads/{id}")
                {
                    return Err(CliError::internal(
                        "Admission returned an unrelated upload route",
                    ));
                }
                match client.put_file(route, &path).await {
                    Ok(uploaded) => print_json(&uploaded),
                    Err(mut error) => {
                        if client.cancel_staged(id).await.is_err() {
                            error
                                .message
                                .push_str(" Staging cleanup could not be confirmed.");
                        }
                        Err(error)
                    }
                }
            } else {
                output(&client, value, common.json, no_wait).await
            }
        }
        ContentCommand::Export {
            server,
            publication,
            audience,
            destination,
            include_mrpack,
            allow_incomplete,
            replace_sha512,
            confirm: _,
            no_wait,
        } => {
            capability(&client, "contentHostSaves").await?;
            if include_mrpack {
                capability(&client, "contentMrpackExports").await?;
            }
            let value:Value=client.post_json(&scoped(&server,"exports"),&json!({
                "publicationId":publication,"audience":audience,"includeMrpack":include_mrpack,"allowIncomplete":allow_incomplete,
                "hostDestination":{"relativePath":destination,"replaceSha512":replace_sha512}
            })).await?;
            output(&client, value, common.json, no_wait).await
        }
    }
}
