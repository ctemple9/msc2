//! Paper launch-command construction for the Phase 4 Java lifecycle slice.
//!
//! The Java executable has already been normalized and validated before it
//! reaches this module. This code only mirrors MSC 1's Paper argv shape and
//! the "server JAR must exist in the working directory" check.

use std::fmt;
use std::path::{Path, PathBuf};

const SANDBOX_SUPPRESS_FLAGS: [&str; 4] = [
    "-Djna.nosys=true",
    "-Djna.nounpack=true",
    "-Djline.terminal=dumb",
    "-Dio.netty.noUnsafe=true",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedJavaLaunch {
    pub executable_path: PathBuf,
    pub prefix_arguments: Vec<String>,
}

impl ValidatedJavaLaunch {
    pub fn new(
        executable_path: impl Into<PathBuf>,
        prefix_arguments: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            executable_path: executable_path.into(),
            prefix_arguments: prefix_arguments.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaperLaunchRequest {
    pub java: ValidatedJavaLaunch,
    pub server_dir: PathBuf,
    pub paper_jar_path: PathBuf,
    pub min_ram_gb: f64,
    pub max_ram_gb: f64,
    pub extra_flags: String,
}

impl PaperLaunchRequest {
    pub fn new(
        java: ValidatedJavaLaunch,
        server_dir: impl Into<PathBuf>,
        paper_jar_path: impl Into<PathBuf>,
        min_ram_gb: f64,
        max_ram_gb: f64,
        extra_flags: impl Into<String>,
    ) -> Self {
        Self {
            java,
            server_dir: server_dir.into(),
            paper_jar_path: paper_jar_path.into(),
            min_ram_gb,
            max_ram_gb,
            extra_flags: extra_flags.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperLaunchCommand {
    pub executable_path: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaLaunchError {
    ServerJarNotFound { path: PathBuf },
}

impl fmt::Display for JavaLaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ServerJarNotFound { path } => {
                write!(
                    f,
                    "Server JAR not found in server folder: {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for JavaLaunchError {}

pub trait JavaLaunchFileSystem {
    fn is_file(&self, path: &Path) -> bool;
}

pub struct StdJavaLaunchFileSystem;

impl JavaLaunchFileSystem for StdJavaLaunchFileSystem {
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
}

pub fn build_paper_launch_command(
    fs: &dyn JavaLaunchFileSystem,
    request: &PaperLaunchRequest,
) -> Result<PaperLaunchCommand, JavaLaunchError> {
    let jar_name = paper_jar_name(&request.paper_jar_path);
    let jar_in_working_dir = request.server_dir.join(&jar_name);
    if !fs.is_file(&jar_in_working_dir) {
        return Err(JavaLaunchError::ServerJarNotFound {
            path: jar_in_working_dir,
        });
    }

    let mut arguments = request.java.prefix_arguments.clone();
    arguments.extend(jvm_flags(
        request.min_ram_gb,
        request.max_ram_gb,
        &request.extra_flags,
    ));
    arguments.extend(["-jar".to_string(), jar_name, "--nogui".to_string()]);

    Ok(PaperLaunchCommand {
        executable_path: request.java.executable_path.clone(),
        arguments,
        working_directory: request.server_dir.clone(),
    })
}

pub fn jvm_flags(min_ram_gb: f64, max_ram_gb: f64, extra_flags: &str) -> Vec<String> {
    let mut flags = vec![
        format!("-Xms{}M", megabytes_from_gb(min_ram_gb)),
        format!("-Xmx{}M", megabytes_from_gb(max_ram_gb)),
    ];
    flags.extend(
        SANDBOX_SUPPRESS_FLAGS
            .iter()
            .map(|flag| (*flag).to_string()),
    );
    flags.extend(extra_flags.split_whitespace().map(str::to_string));
    flags
}

pub fn megabytes_from_gb(gb: f64) -> i64 {
    (gb * 1024.0).round() as i64
}

fn paper_jar_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("paper.jar")
        .to_string()
}
