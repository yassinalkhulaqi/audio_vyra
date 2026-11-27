//! Path handling for user-facing files.
//!
//! The UI passes plain strings for preset directories, recording paths and
//! config files. Without validation a malicious or mistaken value could read
//! or overwrite files anywhere the app process can reach. Every path is
//! therefore resolved relative to an application-owned root and checked before
//! it reaches the filesystem.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    Empty,
    AbsoluteNotAllowed,
    ParentTraversal,
    InvalidComponent,
    OutsideRoot,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            PathError::Empty => "path is empty",
            PathError::AbsoluteNotAllowed => "absolute paths are not allowed",
            PathError::ParentTraversal => "parent directory traversal is not allowed",
            PathError::InvalidComponent => "path contains an invalid component",
            PathError::OutsideRoot => "path resolves outside the application directory",
        };
        f.write_str(text)
    }
}

impl std::error::Error for PathError {}

/// Reject path strings that contain traversal, absolute prefixes, or prefixes.
pub fn validate_relative(input: &str) -> Result<PathBuf, PathError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(PathError::Empty);
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(PathError::AbsoluteNotAllowed);
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Err(PathError::ParentTraversal),
            _ => return Err(PathError::InvalidComponent),
        }
    }
    Ok(path.to_path_buf())
}

/// Resolve `input` under `root`, rejecting escapes and normalising separators.
pub fn resolve_under(root: &Path, input: &str) -> Result<PathBuf, PathError> {
    let relative = validate_relative(input)?;
    let candidate = root.join(relative);
    if !candidate.starts_with(root) {
        return Err(PathError::OutsideRoot);
    }
    Ok(candidate)
}

/// Sanitise a user-supplied file name: keep alphanumerics, dash, underscore and
/// dot, collapse everything else, and never allow a leading dot.
pub fn safe_file_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    while out.starts_with('.') || out.starts_with('_') {
        out.remove(0);
    }
    if out.is_empty() {
        out.push_str("untitled");
    }
    out
}

/// Write bytes to disk atomically through a temporary file in the same
/// directory, then rename. Existing files are never partially overwritten.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut temp = path.to_path_buf();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("bin");
    temp.set_extension(format!("{ext}.tmp"));
    fs::write(&temp, bytes)?;
    match fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(&temp);
            Err(err)
        }
    }
}

/// Ensure a directory exists inside `root` and return it.
pub fn ensure_dir(root: &Path, relative: &str) -> Result<PathBuf, PathError> {
    let dir = resolve_under(root, relative)?;
    fs::create_dir_all(&dir).map_err(|_| PathError::InvalidComponent)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_relative_paths() {
        assert_eq!(validate_relative("presets").unwrap(), PathBuf::from("presets"));
        assert_eq!(validate_relative("./presets/voice.json").unwrap(), PathBuf::from("./presets/voice.json"));
        assert_eq!(validate_relative("a/b/c.wav").unwrap(), PathBuf::from("a/b/c.wav"));
    }

    #[test]
    fn rejects_traversal_and_absolute_paths() {
        assert_eq!(validate_relative("../etc/passwd"), Err(PathError::ParentTraversal));
        assert_eq!(validate_relative("a/../../b"), Err(PathError::ParentTraversal));
        assert!(validate_relative("/etc/passwd").is_err());
        assert_eq!(validate_relative(""), Err(PathError::Empty));
    }

    #[test]
    fn file_names_are_sanitised() {
        assert_eq!(safe_file_name("My Voice!.wav"), "My_Voice_.wav");
        assert_eq!(safe_file_name("...hidden"), "hidden");
        assert_eq!(safe_file_name(""), "untitled");
        assert!(!safe_file_name("..").starts_with('.'));
    }

    #[test]
    fn resolve_under_keeps_paths_inside_root() {
        let root = std::env::temp_dir().join(format!("aurivo-paths-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let resolved = resolve_under(&root, "presets/voice.json").unwrap();
        assert!(resolved.starts_with(&root));
        assert_eq!(resolve_under(&root, "../escape"), Err(PathError::ParentTraversal));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let dir = std::env::temp_dir().join(format!("aurivo-atomic-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert!(!dir.join("state.json.tmp").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ensure_dir_creates_nested_directories() {
        let root = std::env::temp_dir().join(format!("aurivo-dirs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let dir = ensure_dir(&root, "nested/deeper").unwrap();
        assert!(dir.is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn windows_drive_prefix_is_rejected() {
        #[cfg(windows)]
        {
            assert!(validate_relative("C:\\Users\\secret").is_err());
        }
    }
}
