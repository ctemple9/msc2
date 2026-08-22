//! Playit-specific process inputs.
//!
//! The secret itself never enters this type.  A platform secret bridge gives
//! `playitd` an opaque, access-restricted path; the process supervisor only
//! receives that path as its `--secret-path` argument.

use crate::process::ProcessSpawnRequest;
use std::path::PathBuf;

pub const PLAYIT_SECRET_KEY: &str = "playit.secret-key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayitLaunch {
    pub executable_path: PathBuf,
    pub working_directory: PathBuf,
    pub secret_path: PathBuf,
}

impl PlayitLaunch {
    pub fn process_request(&self) -> ProcessSpawnRequest {
        ProcessSpawnRequest::new(&self.executable_path, &self.working_directory).args([
            "--secret-path".to_string(),
            self.secret_path.to_string_lossy().into_owned(),
        ])
    }
}
