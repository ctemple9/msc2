//! `FileSystem`: the minimal filesystem surface later Phase 3 steps build
//! on (path safety, atomic writes, versioned config, download staging).
//! Two implementations ship here: [`StdFileSystem`], backed by `std::fs`,
//! and [`FakeFileSystem`], an in-memory stand-in for tests.

use std::collections::BTreeMap;
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

pub trait FileSystem {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()>;
    fn stat(&self, path: &Path) -> io::Result<Metadata>;
    fn list(&self, path: &Path) -> io::Result<Vec<PathBuf>>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn remove(&self, path: &Path) -> io::Result<()>;
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
}

impl FakeFileSystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// `tree` is the fixture-format `fsTree` object. Every entry seen in
    /// the fixture corpus so far is `"type": "file"` — no directory
    /// entries exist yet, so any other type is treated as a fixture bug
    /// and panics rather than being silently ignored.
    pub fn from_tree(tree: &serde_json::Value) -> Self {
        let object = tree.as_object().expect("fsTree must be a JSON object");
        let mut files = BTreeMap::new();
        for (path, meta) in object {
            let entry_type = meta
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| panic!("fsTree entry {path:?} missing 'type'"));
            assert_eq!(
                entry_type, "file",
                "fsTree entry {path:?} has unsupported type {entry_type:?}"
            );
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
        Self {
            files: Mutex::new(files),
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
        // No directory entries are stored explicitly — a path counts as a
        // directory here if some stored file lives underneath it.
        if files.keys().any(|p| p != path && p.starts_with(path)) {
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

    fn list(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        Ok(files
            .keys()
            .filter(|p| p.parent() == Some(path))
            .cloned()
            .collect())
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
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
}
