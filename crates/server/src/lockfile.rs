use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

const LOCK_FILE: &str = ".lock";

pub struct ServerLock {
    file: File,
}

impl ServerLock {
    pub fn try_acquire(db_root: &Path) -> io::Result<Option<Self>> {
        let file = open_lock(db_root)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Self { file })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(error)) => Err(error),
        }
    }
}

impl Drop for ServerLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn open_lock(db_root: &Path) -> io::Result<File> {
    std::fs::create_dir_all(db_root).map_err(|err| naming(&err, db_root, "create"))?;
    let path = db_root.join(LOCK_FILE);
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|err| naming(&err, &path, "write"))
}

fn naming(err: &io::Error, path: &Path, action: &str) -> io::Error {
    io::Error::new(err.kind(), format!("cannot {action} [{}]: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquires_on_free_root() {
        let dir = tempfile::tempdir().unwrap();
        let held = ServerLock::try_acquire(dir.path()).unwrap();
        assert!(held.is_some());
        assert!(dir.path().join(LOCK_FILE).exists());
    }

    #[test]
    fn contended_acquire_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let held = ServerLock::try_acquire(dir.path()).unwrap();
        assert!(held.is_some());
        assert!(ServerLock::try_acquire(dir.path()).unwrap().is_none());
    }

    #[test]
    fn releases_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let held = ServerLock::try_acquire(dir.path()).unwrap();
        assert!(held.is_some());
        drop(held);
        assert!(ServerLock::try_acquire(dir.path()).unwrap().is_some());
    }

    #[test]
    fn creates_missing_db_root() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        let _held = ServerLock::try_acquire(&nested).unwrap().unwrap();
        assert!(nested.join(LOCK_FILE).exists());
    }

    #[test]
    fn errors_when_db_root_is_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-a-dir");
        std::fs::write(&file, b"x").unwrap();
        let err = ServerLock::try_acquire(&file).map(|_| ()).expect_err("not a data directory");
        assert!(err.to_string().contains(&file.display().to_string()), "{err}");
        assert!(err.to_string().starts_with("cannot create ["), "{err}");
    }

    #[test]
    fn a_data_directory_it_cannot_write_names_itself() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("read-only");
        std::fs::create_dir(&root).unwrap();
        let mut permissions = std::fs::metadata(&root).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&root, permissions).unwrap();

        let err =
            ServerLock::try_acquire(&root).map(|_| ()).expect_err("the lock cannot be written");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert!(err.to_string().contains(&root.join(LOCK_FILE).display().to_string()), "{err}");
        assert!(err.to_string().starts_with("cannot write ["), "{err}");
    }
}
