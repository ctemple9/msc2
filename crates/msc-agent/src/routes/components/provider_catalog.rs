//! Additive provider-specific endpoints leave legacy Modrinth contracts intact.
use super::*;
use msc_api::dto::{
    ProviderCatalogProjectDto, ProviderCatalogReleaseDto, ProviderCatalogSearchDto,
    ProviderCatalogVersionsDto,
};
use provider::java_catalog as cf;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct QueryDto {
    #[serde(default)]
    q: String,
    #[serde(default = "default_kind")]
    kind: String,
    game_version: Option<String>,
    loader: Option<String>,
    #[serde(default)]
    offset: u32,
}
fn default_kind() -> String {
    "mod".into()
}

impl QueryDto {
    fn provider_query(&self) -> Result<cf::Query<'_>, AddonProviderError> {
        let loader = match self.loader.as_deref() {
            None | Some("") => None,
            Some("forge") => Some(1),
            Some("fabric") => Some(4),
            Some("quilt") => Some(5),
            Some("neoforge") => Some(6),
            Some(_) => {
                return Err(AddonProviderError::Network(
                    "Unsupported CurseForge loader".into(),
                ));
            }
        };
        Ok(cf::Query {
            text: &self.q,
            class_id: None,
            game_version: self.game_version.as_deref(),
            loader_type: loader,
            offset: self.offset,
            limit: 50,
        })
    }
}

fn project_item(project: cf::Project, kind: String) -> CatalogItemDto {
    CatalogItemDto {
        project_id: project.id.to_string(),
        slug: project.slug,
        title: project.name,
        description: project.summary,
        author: project
            .authors
            .into_iter()
            .map(|author| author.name)
            .collect::<Vec<_>>()
            .join(", "),
        downloads: project.download_count as i64,
        icon_url: project.logo.map(|logo| logo.url),
        is_client_only: false,
        project_type: kind,
    }
}

pub(super) async fn search(
    Extension(_credential): Extension<AuthenticatedCredential>,
    Query(query): Query<QueryDto>,
) -> Response {
    let result = tokio::task::spawn_blocking(move || {
        let secrets = production_secret_store().map_err(|_| AddonProviderError::MissingApiKey)?;
        let transport = HttpTransport::new();
        let aliases: &[&str] = match query.kind.as_str() {
            "mod" => &["mc-mods", "mods"],
            "plugin" => &["bukkit-plugins", "plugins"],
            "modpack" => &["modpacks"],
            "resourcepack" => &["texture-packs", "resource-packs"],
            "shader" => &["shaders"],
            "datapack" => &["data-packs", "datapacks"],
            _ => {
                return Err(AddonProviderError::Network(
                    "Unknown content category".into(),
                ));
            }
        };
        let classes = cf::classes(&transport, secrets.as_ref())?;
        let Some(class) = classes
            .iter()
            .find(|class| aliases.contains(&class.slug.as_str()))
        else {
            return Err(AddonProviderError::Network(format!(
                "CurseForge does not expose a {} class",
                query.kind
            )));
        };
        let mut provider_query = query.provider_query()?;
        // Discovery of modpacks must include incompatible packs for owner review.
        if query.kind == "modpack" {
            provider_query.game_version = None;
            provider_query.loader_type = None;
        }
        provider_query.class_id = Some(class.id);
        let page = cf::search(&transport, secrets.as_ref(), &provider_query)?;
        Ok(ProviderCatalogSearchDto {
            provider: "curseforge".into(),
            results: page
                .data
                .into_iter()
                .map(|project| project_item(project, query.kind.clone()))
                .collect(),
            offset: page.pagination.index,
            total_count: page.pagination.total_count,
        })
    })
    .await;
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        Ok(Err(error)) => provider_error(error),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "catalog_failed",
            "Catalog worker failed",
        ),
    }
}

pub(super) async fn project(
    Extension(_credential): Extension<AuthenticatedCredential>,
    AxumPath(project_id): AxumPath<u64>,
) -> Response {
    let result = tokio::task::spawn_blocking(move || {
        let secrets = production_secret_store().map_err(|_| AddonProviderError::MissingApiKey)?;
        let transport = HttpTransport::new();
        let project = cf::project(&transport, secrets.as_ref(), project_id)?;
        let body = cf::description(&transport, secrets.as_ref(), project_id)?;
        Ok::<_, AddonProviderError>(ProviderCatalogProjectDto {
            provider: "curseforge".into(),
            distribution_allowed: project.allow_mod_distribution,
            project: CatalogProjectDetailDto {
                project_id: project.id.to_string(),
                slug: project.slug,
                title: project.name,
                description: project.summary,
                body,
                icon_url: project.logo.map(|logo| logo.url),
                downloads: project.download_count as i64,
                server_side: "unknown".into(),
                ..Default::default()
            },
        })
    })
    .await;
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        Ok(Err(error)) => provider_error(error),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "catalog_failed",
            "Catalog worker failed",
        ),
    }
}

pub(super) async fn versions(
    Extension(_credential): Extension<AuthenticatedCredential>,
    AxumPath(project_id): AxumPath<u64>,
    Query(query): Query<QueryDto>,
) -> Response {
    let result = tokio::task::spawn_blocking(move || {
        let secrets = production_secret_store().map_err(|_| AddonProviderError::MissingApiKey)?;
        let transport = HttpTransport::new();
        cf::project(&transport, secrets.as_ref(), project_id)?;
        let page = cf::releases(
            &transport,
            secrets.as_ref(),
            project_id,
            &query.provider_query()?,
        )?;
        let versions = page
            .data
            .into_iter()
            .map(|release| {
                let sha1 = release
                    .hashes
                    .iter()
                    .find(|hash| hash.algo == 1)
                    .map(|hash| hash.value.clone());
                let download = release.download_url.filter(|url| !url.is_empty());
                ProviderCatalogReleaseDto {
                    available: release.is_available,
                    server_pack: release.is_server_pack.unwrap_or(false),
                    sha1,
                    manual_download_required: download.is_none(),
                    version: CatalogVersionDto {
                        id: release.id.to_string(),
                        project_id: release.mod_id.to_string(),
                        name: release.display_name,
                        version_number: release.file_name.clone(),
                        version_type: match release.release_type {
                            1 => "release",
                            2 => "beta",
                            3 => "alpha",
                            _ => "unknown",
                        }
                        .into(),
                        loaders: release
                            .game_versions
                            .iter()
                            .filter_map(|version| match version.as_str() {
                                "Forge" => Some("forge".into()),
                                "NeoForge" => Some("neoforge".into()),
                                "Fabric" => Some("fabric".into()),
                                "Quilt" => Some("quilt".into()),
                                _ => None,
                            })
                            .collect(),
                        game_versions: release.game_versions,
                        date_published: Some(release.file_date),
                        dependencies: release
                            .dependencies
                            .into_iter()
                            .map(|dependency| CatalogVersionDependencyDto {
                                project_id: Some(dependency.mod_id.to_string()),
                                version_id: None,
                                dependency_type: match dependency.relation_type {
                                    3 => "required",
                                    2 => "optional",
                                    5 => "incompatible",
                                    6 => "embedded",
                                    _ => "other",
                                }
                                .into(),
                            })
                            .collect(),
                        files: download
                            .into_iter()
                            .map(|url| CatalogVersionFileDto {
                                url,
                                filename: release.file_name.clone(),
                                primary: true,
                                size: i64::try_from(release.file_length).ok(),
                            })
                            .collect(),
                    },
                }
            })
            .collect();
        Ok::<_, AddonProviderError>(ProviderCatalogVersionsDto {
            provider: "curseforge".into(),
            versions,
            offset: page.pagination.index,
            total_count: page.pagination.total_count,
        })
    })
    .await;
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        Ok(Err(error)) => provider_error(error),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "catalog_failed",
            "Catalog worker failed",
        ),
    }
}

pub(super) async fn acquire(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    body: Result<Json<msc_api::dto::CatalogAcquireRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide a provider, projectId and versionId",
            );
        }
    };
    if !matches!(body.provider.as_str(), "modrinth" | "curseforge")
        || body.project_id.is_empty()
        || body.version_id.is_empty()
        || body.project_id.len() > 128
        || body.version_id.len() > 128
    {
        return invalid_body("invalid_source", "Choose an exact supported pack release");
    }
    let permit = match state.staging.transfer_slots.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "staging_capacity",
                "Content transfers are busy",
            );
        }
    };
    let operation = match state.lifecycle.operations().begin_lifecycle(
        "catalog-acquire",
        None,
        "Downloading selected modpack",
    ) {
        Ok(operation) => operation,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let operation_id = operation.as_str().to_owned();
    let servers_root = state.lifecycle.servers_root();
    let cancel = state.lifecycle.operations().cancellation_check(&operation);
    let identity = crate::auth::StreamAuthentication {
        credential: credential.clone(),
    };
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/catalog/modpacks/acquire",
        StatusCode::ACCEPTED,
    );
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let cancelled = || {
            cancel()
                || !auth.stream_retains_permissions(&identity)
                || state.lifecycle.servers_root() != servers_root
        };
        let id = Uuid::new_v4().to_string();
        let destination = staging_root(&servers_root)
            .join("uploads")
            .join(format!("{id}.bin"));
        let result = (|| {
            let secrets =
                production_secret_store().map_err(|_| AddonProviderError::MissingApiKey)?;
            let transport = HttpTransport::new();
            if body.provider == "curseforge" {
                let project_id = body
                    .project_id
                    .parse()
                    .map_err(|_| AddonProviderError::InvalidDirectUrl)?;
                let project = cf::project(&transport, secrets.as_ref(), project_id)?;
                let classes = cf::classes(&transport, secrets.as_ref())?;
                if !classes.iter().any(|class| {
                    class.slug == "modpacks" && Some(i64::from(class.id)) == project.class_id
                }) {
                    return Err(AddonProviderError::Network(
                        "Selected project is not a modpack".into(),
                    ));
                }
            } else {
                let project = modrinth_get(
                    &format!("project/{}", modrinth_path_segment(&body.project_id)),
                    "Modrinth pack project",
                )?;
                let project: serde_json::Value = serde_json::from_str(&project).map_err(|_| {
                    AddonProviderError::Network("Malformed Modrinth pack project".into())
                })?;
                if project
                    .get("project_type")
                    .and_then(serde_json::Value::as_str)
                    != Some("modpack")
                {
                    return Err(AddonProviderError::Network(
                        "Selected project is not a modpack".into(),
                    ));
                }
            }
            let artifact = provider::artifact::resolve(
                &transport,
                secrets.as_ref(),
                &body.provider,
                &body.project_id,
                &body.version_id,
            )?;
            let name = artifact.file_name.to_ascii_lowercase();
            if !(name.ends_with(".zip") || name.ends_with(".mrpack")) {
                return Err(AddonProviderError::Network(
                    "Selected release is not a pack archive".into(),
                ));
            }
            let bytes = provider::artifact::acquire(
                &artifact,
                secrets.as_ref(),
                &destination,
                MAX_STAGED_UPLOAD_BYTES,
                &cancelled,
            )?;
            if cancelled() {
                let _ = std::fs::remove_file(&destination);
                return Err(AddonProviderError::Network(
                    "Pack acquisition cancelled".into(),
                ));
            }
            state.staging.uploads.lock().unwrap().insert(
                id.clone(),
                StagedUpload {
                    provider_owner: Some(credential.credential_id.clone()),
                    native_guard: None,
                    purpose: StagedUploadPurposeDto::ModpackArchive,
                    file_name: Some(artifact.file_name),
                    operation_id: Some(operation.as_str().to_owned()),
                    file_id: None,
                    expires_at_unix: now_unix() + STAGING_TTL_SECONDS,
                    max_bytes: MAX_STAGED_UPLOAD_BYTES,
                    expected_bytes: Some(bytes),
                    received_bytes: bytes,
                    complete: true,
                    path: destination.clone(),
                },
            );
            Ok(())
        })();
        match result {
            Ok(()) => {
                let mut result = BTreeMap::new();
                result.insert("stagedUploadId".into(), id);
                result.insert("provider".into(), body.provider);
                result.insert("projectId".into(), body.project_id);
                result.insert("versionId".into(), body.version_id);
                let _ = state.lifecycle.finish_operation_success(
                    &operation,
                    "Modpack ready for inspection",
                    result,
                );
            }
            Err(error) => {
                let _ = std::fs::remove_file(&destination);
                if cancel() {
                    let _ = state
                        .lifecycle
                        .operations()
                        .cancel(&operation, "Modpack acquisition cancelled");
                } else {
                    let _ = state.lifecycle.finish_operation_failure(
                        &operation,
                        "acquisition_failed",
                        error.to_string(),
                    );
                }
            }
        }
    });
    (
        StatusCode::ACCEPTED,
        Json(msc_api::dto::CatalogAcquireResultDto { operation_id }),
    )
        .into_response()
}
