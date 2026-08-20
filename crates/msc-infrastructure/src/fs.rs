//! `FileSystem`: the minimal filesystem surface later Phase 3 steps build
//! on (path safety, atomic writes, versioned config, download staging).
//! Two implementations ship here: [`StdFileSystem`], backed by `std::fs`,
//! and [`FakeFileSystem`], an in-memory stand-in for tests.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// What callers need to know about a path: file vs. directory, whether
/// it's executable, its size, and its last-modified time. Mirrors the
/// shape fixtures already use (`docs/msc2/fixture-format.md`'s `fsTree`),
/// not the full breadth of `std::fs::Metadata`. `size`/`modified` are a
/// P6.15 addition (flagged deviation from that step's own `Files:` list,
/// which didn't name this file) — backup listing (`backup_store::
/// list_backups`) is the first caller in this codebase that needs a
/// file's size or timestamp without reading its full contents just to
/// measure `.len()`, the trick every earlier step (`zip_size_bytes`)
/// used instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    pub is_file: bool,
    pub is_dir: bool,
    pub executable: bool,
    pub size: u64,
    pub modified: SystemTime,
}

pub trait FileSystem: Send + Sync {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()>;
    /// Writes `contents` to `path` and marks the result executable —
    /// `write` alone never touches permission bits (matching
    /// `std::fs::write`'s own behavior), which is fine for every prior
    /// caller in this crate but wrong for `java_runtime_install.rs`
    /// (P7.16), the first one that extracts a real binary (`bin/java`)
    /// out of a downloaded archive: a managed Java install whose `java`
    /// binary isn't executable is silently useless. Unix sets `0o755`;
    /// Windows has no POSIX executable bit (same rationale as this
    /// module's own [`is_executable`]) so it's a plain write there.
    fn write_executable(&self, path: &Path, contents: &[u8]) -> io::Result<()>;
    fn stat(&self, path: &Path) -> io::Result<Metadata>;
    fn list(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn remove(&self, path: &Path) -> io::Result<()>;
    /// Creates `path` and any missing intermediate directories, matching
    /// `std::fs::create_dir_all`'s "already exists" tolerance — a no-op,
    /// not an error, when `path` is already a directory. No earlier
    /// consumer of this trait needed to create a directory from scratch
    /// (every prior write landed inside an already-provisioned server
    /// directory); `world_store` (P6.10) is the first caller that does —
    /// a brand-new slot's `world_slots/{id}/` has no other reason to
    /// exist yet.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    /// Creates `path` itself, succeeding only if nothing was there
    /// already — `io::ErrorKind::AlreadyExists` otherwise. Not recursive:
    /// callers ensure `path`'s parent already exists first (typically via
    /// [`Self::create_dir_all`] on the parent), the same division of
    /// labor `std::fs::create_dir`/`create_dir_all` already draw. This is
    /// the atomic counterpart to `create_dir_all`'s deliberate
    /// already-exists tolerance: right for a directory two independent
    /// operations might both legitimately want to exist first
    /// (`world_slots/`), wrong for claiming a brand-new server's own
    /// directory, where a `stat`-then-`create_dir_all` two-step lets two
    /// concurrent creates of the same name both "win" the check before
    /// either creates anything — the P7.1-flagged, P7.33-closed race
    /// `msc-application::provisioning`'s creation functions used to have.
    fn create_dir_exclusive(&self, path: &Path) -> io::Result<()>;
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

    fn write_executable(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        std::fs::write(path, contents)?;
        set_executable(path)
    }

    fn stat(&self, path: &Path) -> io::Result<Metadata> {
        let meta = std::fs::metadata(path)?;
        Ok(Metadata {
            is_file: meta.is_file(),
            is_dir: meta.is_dir(),
            executable: is_executable(&meta),
            size: meta.len(),
            modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
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

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn create_dir_exclusive(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir(path)
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

#[cfg(unix)]
fn set_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> io::Result<()> {
    Ok(())
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
    /// Defaults to `SystemTime::UNIX_EPOCH` for every entry — no test
    /// before P6.15 needed a fake file's modified time to mean anything,
    /// so there's no seeding builder for it yet; backup-listing tests
    /// exercise mtime-dependent sort order against a real `StdFileSystem`
    /// temp directory instead, the same "archive.rs needs real files"
    /// precedent `world_slot_crud.rs` already set.
    modified: SystemTime,
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
                            modified: SystemTime::UNIX_EPOCH,
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
                modified: SystemTime::UNIX_EPOCH,
            },
        );
        self
    }

    /// Sets an already-seeded file's modification time — P7.21's
    /// `jar-summary-geyser-floodgate-pick-newest-by-modification-date`
    /// fixture is the first case that needs one fake file to read as
    /// newer than another (`msc-application/src/templates.rs`'s
    /// `jar_summary`); the field's own doc comment is now out of date,
    /// since a seeding builder exists. Panics if `path` wasn't already
    /// seeded via [`Self::with_file`]/[`Self::from_tree`] — a modified
    /// time on a file that doesn't exist is a test bug, not a case worth
    /// silently ignoring.
    pub fn with_modified(self, path: impl Into<PathBuf>, when: SystemTime) -> Self {
        let path = path.into();
        let mut files = self.files.lock().unwrap();
        let entry = files
            .get_mut(&path)
            .unwrap_or_else(|| panic!("with_modified: {path:?} was never seeded with a file"));
        entry.modified = when;
        drop(files);
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
        let modified = files
            .get(path)
            .map(|e| e.modified)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        files.insert(
            path.to_path_buf(),
            FakeEntry {
                contents: contents.to_vec(),
                executable,
                modified,
            },
        );
        Ok(())
    }

    fn write_executable(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        let mut files = self.files.lock().unwrap();
        let modified = files
            .get(path)
            .map(|e| e.modified)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        files.insert(
            path.to_path_buf(),
            FakeEntry {
                contents: contents.to_vec(),
                executable: true,
                modified,
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
                size: entry.contents.len() as u64,
                modified: entry.modified,
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
                size: 0,
                modified: SystemTime::UNIX_EPOCH,
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
        if let Some(entry) = files.remove(from) {
            files.insert(to.to_path_buf(), entry);
            return Ok(());
        }
        // Not a single stored file at `from` -- try a directory rename
        // instead: `std::fs::rename` renames a whole directory in one
        // syscall on every real filesystem (java_runtime_install's
        // atomic "extracting -> final" swap relies on exactly this), so
        // this fake needs to move every stored file nested under `from`
        // to the same relative path under `to`, not just a single key.
        let nested: Vec<PathBuf> = files
            .keys()
            .filter(|p| *p != from && p.starts_with(from))
            .cloned()
            .collect();
        if nested.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                from.display().to_string(),
            ));
        }
        for path in nested {
            let Ok(rel) = path.strip_prefix(from) else {
                continue;
            };
            let new_path = to.join(rel);
            if let Some(entry) = files.remove(&path) {
                files.insert(new_path, entry);
            }
        }
        Ok(())
    }

    /// Removes a single stored file at `path` if one exists there;
    /// otherwise removes every stored file nested under `path` as a
    /// whole subtree, matching `StdFileSystem::remove`'s real
    /// `remove_dir_all` branch — the same "not a single file, fall back
    /// to a subtree walk" shape [`Self::rename`] already uses for its
    /// own directory case. P7.20's `fleet::delete_server` (removing a
    /// whole server directory that holds a real file, e.g. `paper.jar`)
    /// is the first caller that needs this; before it, every `remove`
    /// call in this codebase targeted a single already-known file.
    fn remove(&self, path: &Path) -> io::Result<()> {
        let mut files = self.files.lock().unwrap();
        if files.remove(path).is_some() {
            self.dirs.lock().unwrap().remove(path);
            return Ok(());
        }
        let nested: Vec<PathBuf> = files
            .keys()
            .filter(|p| *p != path && p.starts_with(path))
            .cloned()
            .collect();
        if nested.is_empty() && !self.dirs.lock().unwrap().remove(path) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                path.display().to_string(),
            ));
        }
        for p in nested {
            files.remove(&p);
        }
        Ok(())
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        self.symlinks
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.display().to_string()))
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.dirs.lock().unwrap().insert(path.to_path_buf());
        Ok(())
    }

    /// Existence check and claim happen under one continuous hold of both
    /// locks (`files` first, then `dirs` — [`Self::stat`]'s own order, so
    /// this can never deadlock against it) so nothing else stored by this
    /// fake can observe a gap between the two, the in-memory equivalent
    /// of `std::fs::create_dir`'s real atomicity.
    fn create_dir_exclusive(&self, path: &Path) -> io::Result<()> {
        let files = self.files.lock().unwrap();
        let mut dirs = self.dirs.lock().unwrap();
        let exists = files.contains_key(path)
            || files.keys().any(|p| p != path && p.starts_with(path))
            || dirs.contains(path);
        if exists {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                path.display().to_string(),
            ));
        }
        dirs.insert(path.to_path_buf());
        Ok(())
    }
}
