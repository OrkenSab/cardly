// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use sha2::Digest;
use sha2::Sha256;
use tracing::debug;

pub const CAS_DIR_NAME: &str = "_media_cas";

pub fn get_cas_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(CAS_DIR_NAME)
}

pub fn ensure_cas_dir(data_dir: &Path) -> io::Result<PathBuf> {
    let cas_dir = get_cas_dir(data_dir);
    fs::create_dir_all(&cas_dir)?;
    Ok(cas_dir)
}

/// Computes hex SHA-256 hash of bytes.
pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Stores media content in content-addressable storage (`/data/_media_cas/<sha256>`)
/// and creates a zero-copy filesystem hardlink at `dest_path`.
/// If hardlink creation fails (e.g. across mount boundaries), falls back to direct file write.
pub fn store_media_deduplicated(
    data_dir: &Path,
    dest_path: &Path,
    data: &[u8],
) -> io::Result<()> {
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let cas_dir = ensure_cas_dir(data_dir)?;
    let hash = hash_bytes(data);
    let cas_file = cas_dir.join(&hash);

    // Atomically write CAS master blob if not already stored
    if !cas_file.is_file() {
        let tmp_file = cas_dir.join(format!(".tmp_{}_{}", hash, std::process::id()));
        fs::write(&tmp_file, data)?;
        if let Err(e) = fs::rename(&tmp_file, &cas_file) {
            // If another thread already wrote it in parallel, ignore rename error and remove tmp
            let _ = fs::remove_file(&tmp_file);
            if !cas_file.is_file() {
                return Err(e);
            }
        }
    }

    // Remove existing target before linking
    if dest_path.exists() {
        let _ = fs::remove_file(dest_path);
    }

    // Try hardlink first
    match fs::hard_link(&cas_file, dest_path) {
        Ok(_) => {
            debug!(
                hash = %hash,
                target = %dest_path.display(),
                "Hardlinked media to CAS blob"
            );
            Ok(())
        }
        Err(err) => {
            // Fallback for cross-device or non-supporting filesystems
            debug!(
                error = %err,
                target = %dest_path.display(),
                "Hardlink failed, falling back to direct write"
            );
            fs::write(dest_path, data)
        }
    }
}

/// Deduplicates an existing file in-place by moving content to CAS and hardlinking.
pub fn deduplicate_file_in_place(data_dir: &Path, file_path: &Path) -> io::Result<bool> {
    if !file_path.is_file() {
        return Ok(false);
    }
    let data = fs::read(file_path)?;
    let cas_dir = ensure_cas_dir(data_dir)?;
    let hash = hash_bytes(&data);
    let cas_file = cas_dir.join(&hash);

    // Check if already pointing to the CAS file on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(m1), Ok(m2)) = (file_path.metadata(), cas_file.metadata()) {
            if m1.dev() == m2.dev() && m1.ino() == m2.ino() {
                return Ok(false); // Already hardlinked
            }
        }
    }

    if !cas_file.is_file() {
        let tmp_file = cas_dir.join(format!(".tmp_{}_{}", hash, std::process::id()));
        fs::write(&tmp_file, &data)?;
        let _ = fs::rename(&tmp_file, &cas_file);
    }

    let parent = file_path.parent().unwrap_or_else(|| Path::new("."));
    let mut rand_bytes = [0u8; 8];
    rand::fill(&mut rand_bytes);
    let tmp_dest = parent.join(format!(
        ".tmp_cas_{}_{}",
        hex::encode(rand_bytes),
        std::process::id()
    ));
    if fs::hard_link(&cas_file, &tmp_dest).is_ok() {
        if fs::rename(&tmp_dest, file_path).is_ok() {
            return Ok(true);
        }
        let _ = fs::remove_file(&tmp_dest);
    }
    Ok(false)
}

/// Recursively or flatly deduplicates all media files inside `media_dir`.
pub fn deduplicate_media_in_dir(data_dir: &Path, media_dir: &Path) -> io::Result<usize> {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(media_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(true) = deduplicate_file_in_place(data_dir, &path) {
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

/// Prunes master CAS blobs that have no user file hardlinks pointing to them.
/// On Unix, an unreferenced blob has `nlink() <= 1` (the CAS directory entry itself is the 1 link).
pub fn prune_orphaned_cas_blobs(data_dir: &Path) -> io::Result<usize> {
    let cas_dir = get_cas_dir(data_dir);
    if !cas_dir.is_dir() {
        return Ok(0);
    }
    let mut pruned = 0;
    if let Ok(entries) = fs::read_dir(&cas_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if let Ok(meta) = path.metadata() {
                    if meta.nlink() <= 1 {
                        if fs::remove_file(&path).is_ok() {
                            pruned += 1;
                        }
                    }
                }
            }
        }
    }
    Ok(pruned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cas_deduplication() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path();
        let u1_dir = data_dir.join("users/u1/media");
        let u2_dir = data_dir.join("users/u2/media");

        let content = b"Sample image data repeated across multiple users";
        let f1 = u1_dir.join("image.png");
        let f2 = u2_dir.join("another_name.png");

        store_media_deduplicated(data_dir, &f1, content).unwrap();
        store_media_deduplicated(data_dir, &f2, content).unwrap();

        assert!(f1.is_file());
        assert!(f2.is_file());
        assert_eq!(fs::read(&f1).unwrap(), content);
        assert_eq!(fs::read(&f2).unwrap(), content);

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let m1 = f1.metadata().unwrap();
            let m2 = f2.metadata().unwrap();
            assert_eq!(m1.ino(), m2.ino(), "Inodes must match for deduplicated files");
            assert_eq!(m1.nlink(), 3); // CAS blob + f1 + f2 = 3 links
        }

        // Test orphaned CAS pruning
        fs::remove_file(&f1).unwrap();
        fs::remove_file(&f2).unwrap();
        // Now master CAS blob is the only remaining link (nlink == 1)
        let pruned = prune_orphaned_cas_blobs(data_dir).unwrap();
        #[cfg(unix)]
        assert_eq!(pruned, 1, "Orphaned CAS blob must be pruned");
    }

    #[test]
    fn test_deduplicate_in_place_same_stem_collision() {
        let dir = tempdir().unwrap();
        let data_dir = dir.path();
        let media_dir = data_dir.join("users/u1/media");
        fs::create_dir_all(&media_dir).unwrap();

        let f1 = media_dir.join("sound.mp3");
        let f2 = media_dir.join("sound.png");
        fs::write(&f1, b"identical data").unwrap();
        fs::write(&f2, b"identical data").unwrap();

        assert!(deduplicate_file_in_place(data_dir, &f1).unwrap());
        assert!(deduplicate_file_in_place(data_dir, &f2).unwrap());

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let m1 = f1.metadata().unwrap();
            let m2 = f2.metadata().unwrap();
            assert_eq!(m1.ino(), m2.ino());
        }
    }
}
