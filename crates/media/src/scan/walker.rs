use std::path::Path;

use domain::error::WalkError;
use domain::library::{SourceWalker, WalkedEntry};
use walkdir::WalkDir;

#[derive(Debug, Default, Clone, Copy)]
pub struct WalkdirSourceWalker;

impl SourceWalker for WalkdirSourceWalker {
    async fn walk(&self, root: &str) -> Result<Vec<WalkedEntry>, WalkError> {
        let root = root.to_owned();
        tokio::task::spawn_blocking(move || walk_blocking(&root))
            .await
            .map_err(|err| WalkError::Unreadable(err.to_string()))?
    }
}

fn walk_blocking(root: &str) -> Result<Vec<WalkedEntry>, WalkError> {
    let path = Path::new(root);
    if !path.exists() {
        return Err(WalkError::RootNotFound(root.to_owned()));
    }
    let mut entries = Vec::new();
    for result in WalkDir::new(path).follow_links(false) {
        let entry = match result {
            Ok(entry) => entry,
            Err(err) if err.depth() == 0 => return Err(WalkError::Unreadable(root.to_owned())),
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(path) = entry.path().to_str() else {
            continue;
        };
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        entries.push(WalkedEntry { path: path.to_owned(), size_bytes: metadata.len() });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn walks_files_recursively_skipping_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("movie.mkv"), b"abc").unwrap();
        fs::write(root.join("notes.txt"), b"hello").unwrap();
        fs::create_dir(root.join("season")).unwrap();
        fs::write(root.join("season").join("ep.mp4"), b"xyzw").unwrap();

        let entries = WalkdirSourceWalker.walk(root.to_str().unwrap()).await.unwrap();

        let mut paths: Vec<String> = entries.iter().map(|e| e.path.clone()).collect();
        paths.sort();
        assert_eq!(paths.len(), 3);
        let movie = entries.iter().find(|e| e.path.ends_with("movie.mkv")).unwrap();
        assert_eq!(movie.size_bytes, 3);
        assert!(paths.iter().any(|p| p.ends_with("ep.mp4")));
    }

    #[tokio::test]
    async fn missing_root_is_not_found() {
        let err = WalkdirSourceWalker.walk("/no/such/shadowmask/root").await.unwrap_err();
        assert!(matches!(err, WalkError::RootNotFound(_)));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn does_not_descend_directory_symlinks() {
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("hidden.mkv"), b"secret").unwrap();
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("real.mkv"), b"ok").unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();

        let entries = WalkdirSourceWalker.walk(dir.path().to_str().unwrap()).await.unwrap();

        assert!(entries.iter().any(|e| e.path.ends_with("real.mkv")));
        assert!(!entries.iter().any(|e| e.path.ends_with("hidden.mkv")));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unreadable_root_is_unreadable_error() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o000)).unwrap();
        let result = WalkdirSourceWalker.walk(root.to_str().unwrap()).await;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(result, Err(WalkError::Unreadable(_))));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unreadable_subdir_is_skipped() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("ok.mkv"), b"data").unwrap();
        let locked = root.join("locked");
        fs::create_dir(&locked).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let entries = WalkdirSourceWalker.walk(root.to_str().unwrap()).await.unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(entries.iter().any(|e| e.path.ends_with("ok.mkv")));
    }
}
