use std::path::{Component, Path};

use domain::error::CacheError;
use domain::media::{DerivedAssetDir, DerivedAssetFile};
use jiff::Timestamp;

fn io(err: impl std::fmt::Display) -> CacheError {
    CacheError::Io(err.to_string())
}

fn modified_at(meta: &std::fs::Metadata) -> Timestamp {
    meta.modified()
        .ok()
        .and_then(|at| Timestamp::try_from(at).ok())
        .unwrap_or(Timestamp::UNIX_EPOCH)
}

pub(crate) async fn list_dirs(root: &Path) -> Result<Vec<DerivedAssetDir>, CacheError> {
    let mut entries = match tokio::fs::read_dir(root).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(io(err)),
    };
    let mut dirs = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(io)? {
        let meta = entry.metadata().await.map_err(io)?;
        if !meta.is_dir() {
            continue;
        }
        let Some(owner) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        dirs.push(DerivedAssetDir {
            owner,
            modified_at: modified_at(&meta),
        });
    }
    Ok(dirs)
}

pub(crate) async fn list_files(
    root: &Path,
    owner: &str,
) -> Result<Vec<DerivedAssetFile>, CacheError> {
    if !is_plain_name(owner) {
        return Err(io(format!("refusing to list suspicious directory {owner}")));
    }
    let mut entries = match tokio::fs::read_dir(root.join(owner)).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(io(err)),
    };
    let mut files = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(io)? {
        let meta = entry.metadata().await.map_err(io)?;
        if !meta.is_file() {
            continue;
        }
        let Some(path) = entry.path().to_str().map(str::to_owned) else {
            continue;
        };
        files.push(DerivedAssetFile {
            path,
            modified_at: modified_at(&meta),
        });
    }
    Ok(files)
}

pub(crate) async fn remove_file(root: &Path, path: &str) -> Result<(), CacheError> {
    let target = Path::new(path);
    if !is_inside(root, target) {
        return Err(io(format!("refusing to remove {path} outside the store")));
    }
    match tokio::fs::remove_file(target).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(io(err)),
    }
}

fn is_inside(root: &Path, target: &Path) -> bool {
    let Ok(rest) = target.strip_prefix(root) else {
        return false;
    };
    let mut components = rest.components().peekable();
    if components.peek().is_none() {
        return false;
    }
    components.all(|component| matches!(component, Component::Normal(_)))
}

pub(crate) async fn remove_dir(root: &Path, owner: &str) -> Result<(), CacheError> {
    if !is_plain_name(owner) {
        return Err(io(format!(
            "refusing to remove suspicious directory {owner}"
        )));
    }
    match tokio::fs::remove_dir_all(root.join(owner)).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(io(err)),
    }
}

pub(crate) fn is_plain_name(owner: &str) -> bool {
    let mut components = Path::new(owner).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_are_accepted_and_traversal_is_not() {
        assert!(is_plain_name("abc123"));
        assert!(!is_plain_name(".."));
        assert!(!is_plain_name("a/b"));
        assert!(!is_plain_name("/etc"));
        assert!(!is_plain_name(""));
    }

    #[tokio::test]
    async fn a_missing_root_lists_nothing_and_removing_is_a_no_op() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("never-created");
        assert!(list_dirs(&missing).await.unwrap().is_empty());
        remove_dir(&missing, "anything").await.unwrap();
    }

    #[tokio::test]
    async fn lists_only_directories_and_removes_by_owner() {
        let root = tempfile::tempdir().unwrap();
        tokio::fs::create_dir(root.path().join("owner-a"))
            .await
            .unwrap();
        tokio::fs::write(root.path().join("loose.txt"), b"x")
            .await
            .unwrap();

        let dirs = list_dirs(root.path()).await.unwrap();
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].owner, "owner-a");

        remove_dir(root.path(), "owner-a").await.unwrap();
        assert!(list_dirs(root.path()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_root_that_is_a_file_is_an_error_not_an_empty_listing() {
        let file = tempfile::NamedTempFile::new().unwrap();
        assert!(list_dirs(file.path()).await.is_err());
    }

    #[tokio::test]
    async fn removing_a_dir_whose_root_is_a_file_is_an_error() {
        let file = tempfile::NamedTempFile::new().unwrap();
        assert!(remove_dir(file.path(), "owner-a").await.is_err());
    }

    #[tokio::test]
    async fn lists_only_files_inside_one_owner() {
        let root = tempfile::tempdir().unwrap();
        let owner = root.path().join("v1");
        tokio::fs::create_dir(&owner).await.unwrap();
        tokio::fs::write(owner.join("42.srt"), b"x").await.unwrap();
        tokio::fs::create_dir(owner.join("nested")).await.unwrap();
        tokio::fs::create_dir(root.path().join("v2")).await.unwrap();
        tokio::fs::write(root.path().join("v2/99.srt"), b"x")
            .await
            .unwrap();

        let files = list_files(root.path(), "v1").await.unwrap();
        assert_eq!(
            files.len(),
            1,
            "directories and other owners are not listed"
        );
        assert!(files[0].path.ends_with("42.srt"));
    }

    #[tokio::test]
    async fn listing_a_missing_owner_is_empty_and_traversal_is_refused() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            list_files(root.path(), "never-created")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(list_files(root.path(), "../etc").await.is_err());
    }

    #[tokio::test]
    async fn removes_a_file_inside_the_store_and_refuses_anything_outside_it() {
        let root = tempfile::tempdir().unwrap();
        let owner = root.path().join("v1");
        tokio::fs::create_dir(&owner).await.unwrap();
        let target = owner.join("99.srt");
        tokio::fs::write(&target, b"x").await.unwrap();
        let outside = root.path().parent().unwrap().join("outside.srt");
        tokio::fs::write(&outside, b"x").await.unwrap();

        remove_file(root.path(), target.to_str().unwrap())
            .await
            .unwrap();
        assert!(!target.exists());

        assert!(
            remove_file(root.path(), outside.to_str().unwrap())
                .await
                .is_err()
        );
        assert!(outside.exists(), "a path outside the root is never deleted");
        assert!(
            remove_file(root.path(), root.path().join("../escape").to_str().unwrap())
                .await
                .is_err(),
            "a traversal that textually starts with the root is still refused"
        );
        assert!(
            remove_file(root.path(), root.path().to_str().unwrap())
                .await
                .is_err(),
            "the root itself is not a file the sweep may remove"
        );
        tokio::fs::remove_file(&outside).await.unwrap();
    }

    #[tokio::test]
    async fn an_owner_that_is_not_a_directory_is_an_error_not_an_empty_listing() {
        let file = tempfile::NamedTempFile::new().unwrap();
        assert!(list_files(file.path(), "owner-a").await.is_err());
    }

    #[tokio::test]
    async fn removing_a_directory_through_the_file_path_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        let owner = root.path().join("v1");
        tokio::fs::create_dir(&owner).await.unwrap();
        assert!(
            remove_file(root.path(), owner.to_str().unwrap())
                .await
                .is_err()
        );
        assert!(owner.exists());
    }

    #[tokio::test]
    async fn removing_a_file_that_is_already_gone_is_a_no_op() {
        let root = tempfile::tempdir().unwrap();
        let owner = root.path().join("v1");
        tokio::fs::create_dir(&owner).await.unwrap();
        remove_file(root.path(), owner.join("missing.srt").to_str().unwrap())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn traversal_is_refused_before_touching_the_filesystem() {
        let root = tempfile::tempdir().unwrap();
        let victim = root.path().join("keep");
        tokio::fs::create_dir(&victim).await.unwrap();
        assert!(
            remove_dir(&root.path().join("sub"), "../keep")
                .await
                .is_err()
        );
        assert!(victim.exists());
    }
}
