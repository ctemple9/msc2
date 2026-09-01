//! Server file browser (S1): admin-only directory listing and previewable
//! file reads for the active server's directory. Ports MSC 1's
//! `wireSkinAndFileProviders`'s `filesProvider`/`fileReadProvider`
//! (`AppViewModel+APIWiringContent.swift:170-275`) — same previewable-
//! extension allowlist, same dirs-first/case-insensitive sort, same 512 KiB
//! read cap with a UTF-8-then-Latin-1-then-empty decode fallback, and the
//! same `safe_path`-equivalent escape guard (`resolvedServerFileURL`, line
//! 41) the oracle applies to every path this module resolves. The read side
//! can surface secrets (`rcon.password` in `server.properties`, API keys in
//! plugin YAMLs), which is why the oracle keeps both routes admin-only —
//! `crates/msc-agent/src/routes/files.rs` enforces that, this module trusts
//! its caller already did.

use std::path::Path;
use std::time::SystemTime;

use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::path_safety::safe_path;

const PREVIEWABLE_EXTENSIONS: &[&str] = &[
    "txt",
    "log",
    "yml",
    "yaml",
    "json",
    "properties",
    "sh",
    "cfg",
    "conf",
    "toml",
    "ini",
    "md",
];

/// Oracle's own cap (`let maxBytes = 512 * 1024`, line 249).
const MAX_PREVIEW_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    /// Forward-slash path relative to the server root; also serves as the
    /// entry's id, matching the oracle's `id: rel.isEmpty ? url.lastPathComponent : rel`
    /// (never empty in practice — entries are always children of a listed
    /// directory, never the root itself).
    pub path: String,
    pub is_directory: bool,
    /// `None` for directories, matching `NSURLFileSizeKey`'s own "only
    /// meaningful for regular files" behavior on the oracle side.
    pub size_bytes: Option<u64>,
    pub modified_at: Option<SystemTime>,
    pub file_extension: Option<String>,
    pub is_previewable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryListing {
    pub path: String,
    pub parent_path: Option<String>,
    pub items: Vec<FileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowseOutcome {
    Listed(DirectoryListing),
    /// The requested path escapes the server root (or the root itself is a
    /// forbidden root) — oracle's `note: "invalid_path"` (200, not an
    /// error status; matches `handleGetFiles`'s status switch, which only
    /// special-cases `no_active_server`).
    InvalidPath,
    /// The resolved path doesn't exist, or isn't a directory — oracle's
    /// `note: "directory_not_found"`.
    DirectoryNotFound,
}

/// `requested` is the raw `path` query value, `None`/empty meaning the
/// server root itself.
pub fn browse_directory(
    fs: &dyn FileSystem,
    server_dir: &Path,
    home_dir: &Path,
    requested: Option<&str>,
) -> BrowseOutcome {
    let Ok(root) = safe_path(fs, server_dir, None, home_dir) else {
        return BrowseOutcome::InvalidPath;
    };
    let Ok(dir) = safe_path(fs, server_dir, requested, home_dir) else {
        return BrowseOutcome::InvalidPath;
    };

    match fs.stat(&dir) {
        Ok(stat) if stat.is_dir => {}
        _ => return BrowseOutcome::DirectoryNotFound,
    }

    let mut items: Vec<FileEntry> = fs
        .list(&dir)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|child| file_entry(fs, &root, &child))
        .collect();
    // Dirs first, then case-insensitive name — oracle's own comparator
    // (line 207-209).
    items.sort_by(|a, b| {
        (!a.is_directory, a.name.to_lowercase()).cmp(&(!b.is_directory, b.name.to_lowercase()))
    });

    let path = relative_path(&root, &dir);
    let parent_path = if path.is_empty() {
        None
    } else {
        dir.parent().map(|parent| relative_path(&root, parent))
    };

    BrowseOutcome::Listed(DirectoryListing {
        path,
        parent_path,
        items,
    })
}

/// Returns the total size of a server directory without following symbolic
/// links. A server can contain large nested worlds and mod caches, so the
/// General tab needs one authoritative recursive measurement rather than the
/// immediate file sizes exposed by the browser listing.
pub fn directory_size(fs: &dyn FileSystem, server_dir: &Path, home_dir: &Path) -> Option<u64> {
    let root = safe_path(fs, server_dir, None, home_dir).ok()?;
    directory_size_at(fs, &root).ok()
}

fn directory_size_at(fs: &dyn FileSystem, path: &Path) -> std::io::Result<u64> {
    // Counting a link's target would both escape the configured server root
    // and risk looping through a link cycle. The link itself consumes no
    // server-directory file bytes, so it contributes zero.
    if fs.read_link(path).is_ok() {
        return Ok(0);
    }
    let metadata = fs.stat(path)?;
    if metadata.is_file {
        return Ok(metadata.size);
    }
    if !metadata.is_dir {
        return Ok(0);
    }
    fs.list(path)?.into_iter().try_fold(0u64, |total, child| {
        Ok(total.saturating_add(directory_size_at(fs, &child)?))
    })
}

fn file_entry(fs: &dyn FileSystem, root: &Path, child: &Path) -> Option<FileEntry> {
    let stat = fs.stat(child).ok()?;
    let name = child.file_name()?.to_string_lossy().into_owned();
    // Oracle's `.skipsHiddenFiles`.
    if name.starts_with('.') {
        return None;
    }
    let path = relative_path(root, child);
    let file_extension = (!stat.is_dir)
        .then(|| {
            child
                .extension()
                .map(|ext| ext.to_string_lossy().to_lowercase())
        })
        .flatten();
    let is_previewable = file_extension
        .as_deref()
        .is_some_and(|ext| PREVIEWABLE_EXTENSIONS.contains(&ext));
    Some(FileEntry {
        name,
        path,
        is_directory: stat.is_dir,
        size_bytes: (!stat.is_dir).then_some(stat.size),
        modified_at: Some(stat.modified),
        file_extension,
        is_previewable,
    })
}

fn relative_path(root: &Path, target: &Path) -> String {
    match target.strip_prefix(root) {
        Ok(rel) => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => String::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileReadOutcome {
    Read {
        name: String,
        size_bytes: Option<u64>,
        content: String,
        truncated: bool,
    },
    /// Escapes the server root, or the root itself is forbidden.
    InvalidPath,
    FileNotFound,
    DirectoryNotFile,
    NotPreviewable {
        name: String,
    },
    ReadFailed {
        name: String,
    },
}

/// `requested` is the raw `path` query value (required, unlike browse's).
pub fn read_previewable_file(
    fs: &dyn FileSystem,
    server_dir: &Path,
    home_dir: &Path,
    requested: &str,
) -> FileReadOutcome {
    let Ok(target) = safe_path(fs, server_dir, Some(requested), home_dir) else {
        return FileReadOutcome::InvalidPath;
    };

    let Ok(stat) = fs.stat(&target) else {
        return FileReadOutcome::FileNotFound;
    };
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if stat.is_dir {
        return FileReadOutcome::DirectoryNotFile;
    }

    let extension = target
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase());
    if !extension
        .as_deref()
        .is_some_and(|ext| PREVIEWABLE_EXTENSIONS.contains(&ext))
    {
        return FileReadOutcome::NotPreviewable { name };
    }

    let Ok(bytes) = fs.read(&target) else {
        return FileReadOutcome::ReadFailed { name };
    };
    let truncated = bytes.len() > MAX_PREVIEW_BYTES;
    let slice = if truncated {
        &bytes[..MAX_PREVIEW_BYTES]
    } else {
        &bytes[..]
    };
    // Oracle: UTF-8, else Latin-1 (which never fails to decode any byte
    // sequence), never a hard failure once the bytes are in hand.
    let content = String::from_utf8(slice.to_vec())
        .unwrap_or_else(|_| slice.iter().map(|&b| b as char).collect());

    FileReadOutcome::Read {
        name,
        size_bytes: Some(stat.size),
        content,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_infrastructure::fs::FakeFileSystem;

    fn fs_with_server(server_dir: &str) -> FakeFileSystem {
        FakeFileSystem::new()
            .with_file(
                format!("{server_dir}/server.properties"),
                b"motd=test".to_vec(),
                false,
            )
            .with_file(
                format!("{server_dir}/paper.jar"),
                b"jar-bytes".to_vec(),
                false,
            )
            .with_dir(format!("{server_dir}/plugins"))
            .with_file(
                format!("{server_dir}/plugins/Essentials.jar"),
                b"jar-bytes".to_vec(),
                false,
            )
            .with_file(format!("{server_dir}/.DS_Store"), b"junk".to_vec(), false)
    }

    #[test]
    fn browse_lists_dirs_before_files_case_insensitively() {
        let fs = fs_with_server("/srv/test");
        let outcome =
            browse_directory(&fs, Path::new("/srv/test"), Path::new("/home/nobody"), None);
        let BrowseOutcome::Listed(listing) = outcome else {
            panic!("expected Listed");
        };
        assert_eq!(listing.path, "");
        assert_eq!(listing.parent_path, None);
        let names: Vec<&str> = listing.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["plugins", "paper.jar", "server.properties"]);
        assert!(listing.items[0].is_directory);
        assert_eq!(listing.items[0].size_bytes, None);
    }

    #[test]
    fn browse_skips_hidden_entries_and_marks_previewable() {
        let fs = fs_with_server("/srv/test");
        let outcome =
            browse_directory(&fs, Path::new("/srv/test"), Path::new("/home/nobody"), None);
        let BrowseOutcome::Listed(listing) = outcome else {
            panic!("expected Listed");
        };
        assert!(listing.items.iter().all(|i| i.name != ".DS_Store"));
        let properties = listing
            .items
            .iter()
            .find(|i| i.name == "server.properties")
            .unwrap();
        assert!(properties.is_previewable);
        assert_eq!(properties.path, "server.properties");
        let jar = listing
            .items
            .iter()
            .find(|i| i.name == "paper.jar")
            .unwrap();
        assert!(!jar.is_previewable);
    }

    #[test]
    fn browse_into_subdirectory_reports_relative_paths_and_parent() {
        let fs = fs_with_server("/srv/test");
        let outcome = browse_directory(
            &fs,
            Path::new("/srv/test"),
            Path::new("/home/nobody"),
            Some("plugins"),
        );
        let BrowseOutcome::Listed(listing) = outcome else {
            panic!("expected Listed");
        };
        assert_eq!(listing.path, "plugins");
        assert_eq!(listing.parent_path.as_deref(), Some(""));
        assert_eq!(listing.items[0].path, "plugins/Essentials.jar");
    }

    #[test]
    fn browse_rejects_a_traversal_escape() {
        let fs = fs_with_server("/srv/test");
        let outcome = browse_directory(
            &fs,
            Path::new("/srv/test"),
            Path::new("/home/nobody"),
            Some("../../etc"),
        );
        assert_eq!(outcome, BrowseOutcome::InvalidPath);
    }

    #[test]
    fn browse_reports_a_missing_directory() {
        let fs = fs_with_server("/srv/test");
        let outcome = browse_directory(
            &fs,
            Path::new("/srv/test"),
            Path::new("/home/nobody"),
            Some("does-not-exist"),
        );
        assert_eq!(outcome, BrowseOutcome::DirectoryNotFound);
    }

    #[test]
    fn browse_reports_a_file_path_as_not_a_directory() {
        let fs = fs_with_server("/srv/test");
        let outcome = browse_directory(
            &fs,
            Path::new("/srv/test"),
            Path::new("/home/nobody"),
            Some("paper.jar"),
        );
        assert_eq!(outcome, BrowseOutcome::DirectoryNotFound);
    }

    #[test]
    fn directory_size_includes_nested_and_hidden_files_but_not_symlink_targets() {
        let fs = fs_with_server("/srv/test")
            .with_file("/srv/test/plugins/config.yml", b"config".to_vec(), false)
            .with_symlink("/srv/test/link-to-outside", "/elsewhere");
        assert_eq!(
            directory_size(&fs, Path::new("/srv/test"), Path::new("/home/nobody")),
            Some(37)
        );
    }

    #[test]
    fn directory_size_rejects_a_forbidden_server_root() {
        let fs = fs_with_server("/home/nobody/server");
        assert_eq!(
            directory_size(&fs, Path::new("/home/nobody"), Path::new("/home/nobody")),
            None
        );
    }

    #[test]
    fn read_returns_previewable_file_contents() {
        let fs = fs_with_server("/srv/test");
        let outcome = read_previewable_file(
            &fs,
            Path::new("/srv/test"),
            Path::new("/home/nobody"),
            "server.properties",
        );
        assert_eq!(
            outcome,
            FileReadOutcome::Read {
                name: "server.properties".to_owned(),
                size_bytes: Some(9),
                content: "motd=test".to_owned(),
                truncated: false,
            }
        );
    }

    #[test]
    fn read_rejects_a_non_previewable_extension() {
        let fs = fs_with_server("/srv/test");
        let outcome = read_previewable_file(
            &fs,
            Path::new("/srv/test"),
            Path::new("/home/nobody"),
            "paper.jar",
        );
        assert_eq!(
            outcome,
            FileReadOutcome::NotPreviewable {
                name: "paper.jar".to_owned()
            }
        );
    }

    #[test]
    fn read_rejects_a_directory() {
        let fs = fs_with_server("/srv/test");
        let outcome = read_previewable_file(
            &fs,
            Path::new("/srv/test"),
            Path::new("/home/nobody"),
            "plugins",
        );
        assert_eq!(outcome, FileReadOutcome::DirectoryNotFile);
    }

    #[test]
    fn read_reports_a_missing_file() {
        let fs = fs_with_server("/srv/test");
        let outcome = read_previewable_file(
            &fs,
            Path::new("/srv/test"),
            Path::new("/home/nobody"),
            "missing.txt",
        );
        assert_eq!(outcome, FileReadOutcome::FileNotFound);
    }

    #[test]
    fn read_rejects_a_traversal_escape() {
        let fs = fs_with_server("/srv/test");
        let outcome = read_previewable_file(
            &fs,
            Path::new("/srv/test"),
            Path::new("/home/nobody"),
            "../../etc/passwd",
        );
        assert_eq!(outcome, FileReadOutcome::InvalidPath);
    }

    #[test]
    fn read_truncates_at_the_512kib_cap() {
        let fs = FakeFileSystem::new().with_file(
            "/srv/test/big.log",
            vec![b'a'; MAX_PREVIEW_BYTES + 10],
            false,
        );
        let outcome = read_previewable_file(
            &fs,
            Path::new("/srv/test"),
            Path::new("/home/nobody"),
            "big.log",
        );
        let FileReadOutcome::Read {
            content, truncated, ..
        } = outcome
        else {
            panic!("expected Read");
        };
        assert!(truncated);
        assert_eq!(content.len(), MAX_PREVIEW_BYTES);
    }
}
