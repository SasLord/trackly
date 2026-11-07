//! Content-addressed blob store for attachments (D-18).
//!
//! Layout: `<root>/<first 2 hex>/<64 hex>` (no extension), thumbnails in
//! `<root>/thumbs/<hash>.jpg`, in-flight uploads in `<root>/.tmp`.
//! The only path input is a validated hash — user file names never reach disk.
//! All operations are synchronous; callers wrap them in `spawn_blocking`.

// Consumed by the attachment service (plans 41.3-05..08); not wired yet.
#![allow(dead_code)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

use tempfile::NamedTempFile;
use trackly_core::error::AppError;

/// Local hash gate: exactly 64 chars of `[0-9a-f]`.
fn valid_hash(h: &str) -> bool {
    h.len() == 64 && h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn bad_hash() -> AppError {
    AppError::Validation {
        field: "hash".to_string(),
        message: "Некорректный хеш вложения".to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitOutcome {
    Moved,
    AlreadyPresent,
}

#[derive(Debug, Clone)]
pub struct BlobEntry {
    pub hash: String,
    pub size: u64,
    pub mtime: Option<SystemTime>,
}

#[derive(Debug, Clone, Default)]
pub struct StoreScan {
    pub blobs: Vec<BlobEntry>,
    pub tmp_files: Vec<(PathBuf, Option<SystemTime>)>,
}

#[derive(Debug, Clone)]
pub struct AttachmentStore {
    root: PathBuf,
}

impl AttachmentStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn tmp_dir(&self) -> PathBuf {
        self.root.join(".tmp")
    }

    pub fn ensure_dirs(&self) -> Result<(), AppError> {
        for d in [self.root.clone(), self.tmp_dir(), self.root.join("thumbs")] {
            fs::create_dir_all(&d).map_err(|e| AppError::Validation {
                field: "attachments_path".to_string(),
                message: format!("Не удалось создать каталог вложений: {e}"),
            })?;
        }
        Ok(())
    }

    /// True when the root exists, is a directory and can be listed.
    pub fn root_available(&self) -> bool {
        fs::metadata(&self.root)
            .map(|m| m.is_dir())
            .unwrap_or(false)
            && fs::read_dir(&self.root).is_ok()
    }

    pub fn blob_path(&self, hash: &str) -> Result<PathBuf, AppError> {
        if !valid_hash(hash) {
            return Err(bad_hash());
        }
        Ok(self.root.join(&hash[..2]).join(hash))
    }

    pub fn thumb_path(&self, hash: &str) -> Result<PathBuf, AppError> {
        if !valid_hash(hash) {
            return Err(bad_hash());
        }
        Ok(self.root.join("thumbs").join(format!("{hash}.jpg")))
    }

    pub fn blob_exists(&self, hash: &str) -> bool {
        self.blob_path(hash).map(|p| p.is_file()).unwrap_or(false)
    }

    /// Temp file on the same filesystem as the final location.
    pub fn begin_staging(&self) -> io::Result<NamedTempFile> {
        let dir = self.tmp_dir();
        fs::create_dir_all(&dir)?;
        NamedTempFile::new_in(dir)
    }

    /// Atomically moves a staged file into its shard. If the blob already
    /// exists the temp file is dropped (same content = one file).
    pub fn commit_staged(
        &self,
        temp: NamedTempFile,
        hash: &str,
    ) -> Result<CommitOutcome, AppError> {
        let dest = self.blob_path(hash)?;
        let io_err = |e: io::Error| AppError::Validation {
            field: "attachments_path".to_string(),
            message: format!("Не удалось сохранить вложение: {e}"),
        };
        if dest.is_file() {
            drop(temp);
            return Ok(CommitOutcome::AlreadyPresent);
        }
        temp.as_file().sync_all().map_err(io_err)?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(io_err)?;
        }
        let mut temp = temp;
        let mut attempt = 0;
        loop {
            match temp.persist_noclobber(&dest) {
                Ok(_) => return Ok(CommitOutcome::Moved),
                Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => {
                    return Ok(CommitOutcome::AlreadyPresent);
                }
                Err(e) if e.error.kind() == io::ErrorKind::PermissionDenied && attempt < 3 => {
                    attempt += 1;
                    temp = e.file;
                    thread::sleep(Duration::from_millis(50 * attempt as u64));
                }
                Err(e) => return Err(io_err(e.error)),
            }
        }
    }

    /// Removes the blob and its thumbnail; Ok when already absent.
    pub fn remove_blob(&self, hash: &str) -> Result<(), AppError> {
        let blob = self.blob_path(hash)?;
        let thumb = self.thumb_path(hash)?;
        for p in [blob, thumb] {
            match fs::remove_file(&p) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(AppError::Validation {
                        field: "attachments_path".to_string(),
                        message: format!("Не удалось удалить файл вложения: {e}"),
                    })
                }
            }
        }
        Ok(())
    }

    /// Lists blobs and stale temp files. Symlinks are skipped, `thumbs/` and
    /// `.tmp/` are never treated as shards, non-hash names are ignored.
    pub fn scan(&self) -> io::Result<StoreScan> {
        let mut out = StoreScan::default();
        for shard in fs::read_dir(&self.root)? {
            let shard = shard?;
            let name = shard.file_name().to_string_lossy().into_owned();
            let md = fs::symlink_metadata(shard.path())?;
            if md.file_type().is_symlink() || !md.is_dir() {
                continue;
            }
            if name == ".tmp" {
                for f in fs::read_dir(shard.path())? {
                    let f = f?;
                    let fm = fs::symlink_metadata(f.path())?;
                    if fm.file_type().is_symlink() || !fm.is_file() {
                        continue;
                    }
                    out.tmp_files.push((f.path(), fm.modified().ok()));
                }
                continue;
            }
            if name.len() != 2 || !name.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
                continue;
            }
            for f in fs::read_dir(shard.path())? {
                let f = f?;
                let fname = f.file_name().to_string_lossy().into_owned();
                if !valid_hash(&fname) || !fname.starts_with(&name) {
                    continue;
                }
                let fm = fs::symlink_metadata(f.path())?;
                if fm.file_type().is_symlink() || !fm.is_file() {
                    continue;
                }
                out.blobs.push(BlobEntry {
                    hash: fname,
                    size: fm.len(),
                    mtime: fm.modified().ok(),
                });
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    const H: &str = "ab00000000000000000000000000000000000000000000000000000000000001";

    fn store() -> (TempDir, AttachmentStore) {
        let d = TempDir::new().unwrap();
        let s = AttachmentStore::new(d.path().join("att"));
        s.ensure_dirs().unwrap();
        (d, s)
    }

    fn staged(s: &AttachmentStore, bytes: &[u8]) -> NamedTempFile {
        let mut t = s.begin_staging().unwrap();
        t.write_all(bytes).unwrap();
        t
    }

    #[test]
    fn blob_path_layout_and_rejects_bad_hash() {
        let (_d, s) = store();
        assert_eq!(s.blob_path(H).unwrap(), s.root().join("ab").join(H));
        for bad in [
            "",
            "../x",
            "a/b",
            &H.to_uppercase(),
            &H[..63],
            &format!("{H}0"),
        ] {
            assert!(s.blob_path(bad).is_err(), "{bad}");
            assert!(s.thumb_path(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn staging_inside_tmp_and_commit() {
        let (_d, s) = store();
        let t = staged(&s, b"x");
        assert!(t.path().starts_with(s.root().join(".tmp")));
        assert_eq!(s.commit_staged(t, H).unwrap(), CommitOutcome::Moved);
        assert!(s.blob_exists(H));
        let t2 = staged(&s, b"x");
        let p2 = t2.path().to_path_buf();
        assert_eq!(
            s.commit_staged(t2, H).unwrap(),
            CommitOutcome::AlreadyPresent
        );
        assert!(!p2.exists());
    }

    #[test]
    fn remove_blob_deletes_thumb_and_is_idempotent() {
        let (_d, s) = store();
        s.commit_staged(staged(&s, b"x"), H).unwrap();
        fs::write(s.thumb_path(H).unwrap(), b"t").unwrap();
        s.remove_blob(H).unwrap();
        assert!(!s.blob_exists(H));
        assert!(!s.thumb_path(H).unwrap().exists());
        s.remove_blob(H).unwrap();
    }

    #[test]
    fn scan_lists_blobs_and_tmp_ignores_junk() {
        let (_d, s) = store();
        s.commit_staged(staged(&s, b"hello"), H).unwrap();
        fs::write(s.thumb_path(H).unwrap(), b"t").unwrap();
        let _pending = staged(&s, b"p");
        fs::write(s.root().join("ab").join("notahash"), b"j").unwrap();
        let sc = s.scan().unwrap();
        assert_eq!(sc.blobs.len(), 1);
        assert_eq!(sc.blobs[0].hash, H);
        assert_eq!(sc.blobs[0].size, 5);
        assert_eq!(sc.tmp_files.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn scan_skips_symlinks() {
        let (d, s) = store();
        s.commit_staged(staged(&s, b"hello"), H).unwrap();
        let target = d.path().join("outside");
        fs::write(&target, b"o").unwrap();
        let h2 = "ab00000000000000000000000000000000000000000000000000000000000002";
        std::os::unix::fs::symlink(&target, s.root().join("ab").join(h2)).unwrap();
        let sc = s.scan().unwrap();
        assert_eq!(sc.blobs.len(), 1);
    }

    #[test]
    fn root_available_false_when_missing() {
        let d = TempDir::new().unwrap();
        assert!(!AttachmentStore::new(d.path().join("nope")).root_available());
        let (_d2, s) = store();
        assert!(s.root_available());
    }
}
