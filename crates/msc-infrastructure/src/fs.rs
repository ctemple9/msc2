//! `FileSystem`: the minimal filesystem surface later Phase 3 steps build
//! on (path safety, atomic writes, versioned config, download staging).
//! Two implementations ship here: [`StdFileSystem`], backed by `std::fs`,
//! and [`FakeFileSystem`], an in-memory stand-in for tests.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// What callers need to know about a path: file vs. directory, and
/// whether it's executable. Mirrors the shape fixtures already use
/// (`docs/msc2/fixture-format.md`'s `fsTree`), not the full breadth of
/// `std::fs::Metadata`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    pub is_file: bool,
    pub is_dir: bool,
    pub executable: bool,
}

pub trait FileSystem: Send + Sync {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()>;
    fn stat(&self, path: &Path) -> io::Result<Metadata>;
    fn list(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn remove(&self, path: &Path) -> io::Result<()>;
    /// The immediate target of `path`, if it's a symlink. Errors the same
    /// way `std::fs::read_link` does for a path that doesn't exist or
    /// isn't a symlink — P3.5's `path_safety` module treats that error as
    /// "not a symlink, leave the component as-is" rather than a real
    /// failure, since a path-safety candidate need not exist yet.
    fn read_link(&self, path: &Path) -> io::Result<PathBuf>;
}

/// The real filesystem.
pub struct StdFileSystem;

impl FileSystem for StdFileSystem {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        std::fs::write(path, contents)
    }

    fn stat(&self, path: &Path) -> io::Result<Metadata> {
        let meta = std::fs::metadata(path)?;
        Ok(Metadata {
            is_file: meta.is_file(),
            is_dir: meta.is_dir(),
            executable: is_executable(&meta),
        })
    }

    fn list(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        std::fs::read_dir(path)?
            .map(|entry| entry.map(|e| e.path()))
            .collect()
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        if std::fs::metadata(path)?.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        }
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        std::fs::read_link(path)
    }
}

#[cfg(unix)]
fn is_executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

// Windows has no POSIX executable bit; executability there is an
// extension check, not a permission bit. Not needed until a Windows
// substrate step actually asks the question (D-017).
#[cfg(not(unix))]
fn is_executable(_meta: &std::fs::Metadata) -> bool {
    false
}

/// Joins `base` and `component` with a literal `/`, never
/// `std::path::MAIN_SEPARATOR`. Every fixture path in this codebase is
/// written with forward slashes (`docs/msc2/fixture-format.md`), and
/// `FakeFileSystem` is meant to behave identically on every host OS since
/// it never touches a real filesystem — but `Path::join`/`PathBuf::push`
/// insert the *host's* separator, a backslash on Windows, when joining two
/// components that aren't already separator-terminated. `PathBuf`'s own
/// `Eq`/`Hash` are component-based and don't care, but a caller that
/// formats the result as a raw string (`to_string_lossy()` against a
/// fixture's literal expected string, as `audit_log.rs`'s test does) does
/// — found by P3.20's exit gate check, fixed by P3.20a.
/// Joins `base` and `component` with a literal `/` regardless of host OS.
/// Every fixture path in this codebase is written with forward slashes, and
/// `Path::join`/`PathBuf::push` insert `std::path::MAIN_SEPARATOR` (a
/// backslash on Windows) instead — harmless for `Path` equality/lookup
/// (Windows treats `/` and `\` as equivalent separators), but visible in
/// anything that renders the path back out as text: fixture assertions,
/// and any user-facing message built from a fixture-shaped path. `pub`
/// so other crates constructing paths from the same forward-slash
/// convention (`msc-application`'s Paper launch-command construction,
/// found needing this by a Windows CI failure) don't have to duplicate it.
pub fn join_forward_slash(base: &Path, component: &std::ffi::OsStr) -> PathBuf {
    let mut joined = base.to_string_lossy().into_owned();
    if !joined.ends_with('/') {
        joined.push('/');
    }
    joined.push_str(&component.to_string_lossy());
    PathBuf::from(joined)
}

#[derive(Debug, Clone)]
struct FakeEntry {
    contents: Vec<u8>,
    executable: bool,
}

/// An in-memory filesystem for tests. Constructible directly from the
/// `fsTree` shape fixtures already use (`{"<path>": {"type": "file",
/// "executable": true}}`, per `docs/msc2/fixture-format.md`), so
/// fixture-driven tests (P3.18's deferred java-runtime-guards cases) can
/// build one without reshaping the fixture data.
#[derive(Debug, Default)]
pub struct FakeFileSystem {
    files: Mutex<BTreeMap<PathBuf, FakeEntry>>,
    symlinks: Mutex<BTreeMap<PathBuf, PathBuf>>,
    /// Directories that exist but hold no file, tracked explicitly since
    /// `stat`'s usual "some file starts with this path" inference can't
    /// see them. P3.18's `normalization-directory-without-bin-java` fixture
    /// is the first case that needs this: a freshly created, empty JAVA_HOME
    /// candidate directory, which the fixture's `fsTree` has no way to spell
    /// (P0.1's fsTree schema only defines `"file"` and `"symlink"` entries).
    dirs: Mutex<BTreeSet<PathBuf>>,
    /// Destinations whose next `rename` call fails, simulating a process
    /// interrupted between `atomic_write`'s temp-file write and its rename
    /// step. P5.23's consumer-level atomic-write-interruption dimension
    /// needs this injected at a real `save_config`/`atomic_write` call, not
    /// just simulated by writing straight to the temp path and never
    /// calling the primitive at all (the way `fixtures/atomic-write/
    /// destination-untouched-before-rename` does at the primitive level).
    fail_rename_to: Mutex<BTreeSet<PathBuf>>,
}

impl FakeFileSystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// `tree` is the fixture-format `fsTree` object. Every file entry
    /// seen in the fixture corpus so far is `"type": "file"`; P3.5 adds
    /// `"type": "symlink"` with a `"target"` string (its fixtures are the
    /// first to need one). Any other type is treated as a fixture bug and
    /// panics rather than being silently ignored.
    pub fn from_tree(tree: &serde_json::Value) -> Self {
        let object = tree.as_object().expect("fsTree must be a JSON object");
        let mut files = BTreeMap::new();
        let mut symlinks = BTreeMap::new();
        for (path, meta) in object {
            let entry_type = meta
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| panic!("fsTree entry {path:?} missing 'type'"));
            match entry_type {
                "file" => {
                    let executable = meta
                        .get("executable")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    files.insert(
                        PathBuf::from(path),
                        FakeEntry {
                            contents: Vec::new(),
                            executable,
                        },
                    );
                }
                "symlink" => {
                    let target = meta
                        .get("target")
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| panic!("fsTree symlink {path:?} missing 'target'"));
                    symlinks.insert(PathBuf::from(path), PathBuf::from(target));
                }
                other => panic!("fsTree entry {path:?} has unsupported type {other:?}"),
            }
        }
        Self {
            files: Mutex::new(files),
            symlinks: Mutex::new(symlinks),
            dirs: Mutex::new(BTreeSet::new()),
            fail_rename_to: Mutex::new(BTreeSet::new()),
        }
    }

    /// Seed a single file, for tests that don't start from a fixture's
    /// `fsTree`.
    pub fn with_file(
        self,
        path: impl Into<PathBuf>,
        contents: impl Into<Vec<u8>>,
        executable: bool,
    ) -> Self {
        self.files.lock().unwrap().insert(
            path.into(),
            FakeEntry {
                contents: contents.into(),
                executable,
            },
        );
        self
    }

    /// Seed a single symlink, for tests that don't start from a fixture's
    /// `fsTree`.
    pub fn with_symlink(self, path: impl Into<PathBuf>, target: impl Into<PathBuf>) -> Self {
        self.symlinks
            .lock()
            .unwrap()
            .insert(path.into(), target.into());
        self
    }

    /// Seed a directory that exists but holds no file — see the `dirs`
    /// field's own doc comment for why this can't be expressed through
    /// `from_tree`'s fixture-shaped input alone.
    pub fn with_dir(self, path: impl Into<PathBuf>) -> Self {
        self.dirs.lock().unwrap().insert(path.into());
        self
    }

    /// The next `rename(_, to)` call targeting `to` fails, leaving both the
    /// temp source and the destination exactly as `rename` found them. Lets
    /// a test drive a real `save_config`/`atomic_write` call through an
    /// interruption between the temp-file write and the rename, rather than
    /// only being able to simulate that state by writing to the temp path
    /// directly and never invoking the primitive.
    pub fn with_failing_rename(self, to: impl Into<PathBuf>) -> Self {
        self.fail_rename_to.lock().unwrap().insert(to.into());
        self
    }
}

impl FileSystem for FakeFileSystem {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .map(|entry| entry.contents.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.display().to_string()))
    }

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        let mut files = self.files.lock().unwrap();
        let executable = files.get(path).map(|e| e.executable).unwrap_or(false);
        files.insert(
            path.to_path_buf(),
            FakeEntry {
                contents: contents.to_vec(),
                executable,
            },
        );
        Ok(())
    }

    fn stat(&self, path: &Path) -> io::Result<Metadata> {
        let files = self.files.lock().unwrap();
        if let Some(entry) = files.get(path) {
            return Ok(Metadata {
                is_file: true,
                is_dir: false,
                executable: entry.executable,
            });
        }
        // A path counts as a directory if some stored file lives underneath
        // it, or if it was seeded explicitly via `with_dir` (for a
        // directory that exists but is empty).
        if files.keys().any(|p| p != path && p.starts_with(path))
            || self.dirs.lock().unwrap().contains(path)
        {
            return Ok(Metadata {
                is_file: false,
                is_dir: true,
                executable: false,
            });
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            path.display().to_string(),
        ))
    }

    /// Immediate children of `path`: any stored file directly inside it,
    /// plus one path segment for each stored file nested further below —
    /// the same "return subdirectories too, not just direct files" behavior
    /// `std::fs::read_dir` gives `StdFileSystem::list`. The original
    /// exact-parent-match version only ever saw flat, single-level
    /// directories (audit logs, operation journals); P3.18's
    /// `detect_installed_java_runtimes` is the first caller that needs to
    /// walk a real tree (e.g. discovering `temurin-21.jdk` as a child of a
    /// search root from a file two levels further down), so this
    /// generalizes to match.
    fn list(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        let mut children = BTreeSet::new();
        for file_path in files.keys() {
            if let Ok(rel) = file_path.strip_prefix(path)
                && let Some(first) = rel.components().next()
            {
                children.insert(join_forward_slash(path, first.as_os_str()));
            }
        }
        Ok(children.into_iter().collect())
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        if self.fail_rename_to.lock().unwrap().remove(to) {
            return Err(io::Error::other("simulated rename failure"));
        }
        let mut files = self.files.lock().unwrap();
        let entry = files
            .remove(from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, from.display().to_string()))?;
        files.insert(to.to_path_buf(), entry);
        Ok(())
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        let mut files = self.files.lock().unwrap();
        if files.remove(path).is_some() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                path.display().to_string(),
            ))
        }
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        self.symlinks
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.display().to_string()))
    }
}
