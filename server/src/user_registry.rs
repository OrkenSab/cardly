// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use pbkdf2::password_hash::PasswordHash;
use pbkdf2::password_hash::PasswordVerifier;
use pbkdf2::Pbkdf2;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;
use tracing::warn;

use crate::config::derive_hkey;
use crate::config::hash_password;
use crate::config::validate_username;
use crate::config::UserCredentials;

const USERS_FILE_NAME: &str = "users.json";
const RESERVED_NAMES: &[&str] = &[
    "demo",
    "admin",
    "administrator",
    "system",
    "root",
    "guest",
    "cardly",
    "api",
    "null",
    "undefined",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUserRecord {
    pub username: String,
    pub password_hash: String,
    pub hkey: String,
    pub created_at: u64,
    pub is_guest: bool,
}

#[derive(Default)]
struct RegistryInner {
    users_by_name: HashMap<String, UserCredentials>,
    users_by_hkey: HashMap<String, UserCredentials>,
    stored_records: HashMap<String, StoredUserRecord>,
}

pub struct UserRegistry {
    storage_path: PathBuf,
    inner: RwLock<RegistryInner>,
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl UserRegistry {
    pub fn new(data_dir: &Path, env_users: Vec<UserCredentials>) -> Result<Self> {
        let storage_path = data_dir.join(USERS_FILE_NAME);
        let mut inner = RegistryInner::default();

        // 1. Seed static users from env config (e.g. CARDLY_USER1, demo user)
        for u in env_users {
            inner.users_by_hkey.insert(u.hkey.clone(), u.clone());
            inner.users_by_name.insert(u.username.clone(), u);
        }

        // 2. Load stored dynamic accounts from disk
        if storage_path.is_file() {
            match fs::read(&storage_path) {
                Ok(bytes) => match serde_json::from_slice::<Vec<StoredUserRecord>>(&bytes) {
                    Ok(records) => {
                        info!(
                            count = records.len(),
                            path = %storage_path.display(),
                            "Loaded registered user accounts from persistent storage"
                        );
                        for rec in records {
                            let creds = UserCredentials {
                                username: rec.username.clone(),
                                password_hash: rec.password_hash.clone(),
                                hkey: rec.hkey.clone(),
                            };
                            inner.users_by_hkey.insert(creds.hkey.clone(), creds.clone());
                            inner.users_by_name.insert(creds.username.clone(), creds);
                            inner.stored_records.insert(rec.username.clone(), rec);
                        }
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            path = %storage_path.display(),
                            "Failed to parse users storage file; skipping existing dynamic users"
                        );
                    }
                },
                Err(e) => {
                    warn!(
                        error = %e,
                        path = %storage_path.display(),
                        "Failed to read users storage file"
                    );
                }
            }
        }

        Ok(Self {
            storage_path,
            inner: RwLock::new(inner),
        })
    }

    fn save_to_disk_locked(storage_path: &Path, inner: &RegistryInner) -> Result<()> {
        let list: Vec<StoredUserRecord> = inner.stored_records.values().cloned().collect();
        let json_bytes = serde_json::to_vec_pretty(&list)
            .context("serializing user registry records to JSON")?;
        let tmp_path = storage_path.with_extension(format!("tmp.{}", std::process::id()));
        fs::write(&tmp_path, json_bytes)
            .with_context(|| format!("writing tmp file {}", tmp_path.display()))?;
        fs::rename(&tmp_path, storage_path)
            .with_context(|| format!("renaming {} to {}", tmp_path.display(), storage_path.display()))?;
        Ok(())
    }

    pub fn register(&self, username: &str, password: &str) -> Result<UserCredentials> {
        let clean_user = username.trim();
        validate_username(clean_user)?;

        if clean_user.len() < 3 {
            bail!("Username must be at least 3 characters");
        }
        if password.len() < 6 {
            bail!("Password must be at least 6 characters");
        }

        let lower = clean_user.to_lowercase();
        if RESERVED_NAMES.contains(&lower.as_str()) || lower.starts_with("guest_") {
            bail!("Username '{clean_user}' is reserved. Please choose a different username.");
        }

        let mut guard = self.inner.write().map_err(|_| anyhow!("lock poisoned"))?;

        // Case-insensitive uniqueness check
        for existing in guard.users_by_name.keys() {
            if existing.to_lowercase() == lower {
                bail!("Username '{clean_user}' is already taken");
            }
        }

        let password_hash = hash_password(password)?;
        let hkey = derive_hkey(&format!("{clean_user}:{password}"));

        let creds = UserCredentials {
            username: clean_user.to_string(),
            password_hash: password_hash.clone(),
            hkey: hkey.clone(),
        };

        let record = StoredUserRecord {
            username: clean_user.to_string(),
            password_hash,
            hkey: hkey.clone(),
            created_at: now_unix_secs(),
            is_guest: false,
        };

        guard.users_by_name.insert(clean_user.to_string(), creds.clone());
        guard.users_by_hkey.insert(hkey, creds.clone());
        guard.stored_records.insert(clean_user.to_string(), record);

        Self::save_to_disk_locked(&self.storage_path, &guard)?;
        info!(user = clean_user, "New user registered successfully");
        Ok(creds)
    }

    pub fn register_ephemeral_guest(&self) -> Result<UserCredentials> {
        let mut rand_bytes = [0u8; 4];
        rand::fill(&mut rand_bytes);
        let guest_user = format!("guest_{}", hex::encode(rand_bytes));

        let mut pass_bytes = [0u8; 16];
        rand::fill(&mut pass_bytes);
        let guest_pass = hex::encode(pass_bytes);

        let password_hash = hash_password(&guest_pass)?;
        let hkey = derive_hkey(&format!("{guest_user}:{guest_pass}"));

        let creds = UserCredentials {
            username: guest_user.clone(),
            password_hash: password_hash.clone(),
            hkey: hkey.clone(),
        };

        let record = StoredUserRecord {
            username: guest_user.clone(),
            password_hash,
            hkey: hkey.clone(),
            created_at: now_unix_secs(),
            is_guest: true,
        };

        let mut guard = self.inner.write().map_err(|_| anyhow!("lock poisoned"))?;
        guard.users_by_name.insert(guest_user.clone(), creds.clone());
        guard.users_by_hkey.insert(hkey, creds.clone());
        guard.stored_records.insert(guest_user.clone(), record);

        let _ = Self::save_to_disk_locked(&self.storage_path, &guard);
        info!(user = %guest_user, "Created ephemeral guest account");
        Ok(creds)
    }

    pub fn verify_credentials(&self, username: &str, password: &str) -> Option<UserCredentials> {
        let guard = self.inner.read().ok()?;
        let user = guard.users_by_name.get(username).cloned();

        match user {
            Some(u) => {
                let parsed = PasswordHash::new(&u.password_hash).ok()?;
                if Pbkdf2.verify_password(password.as_bytes(), &parsed).is_ok() {
                    Some(u)
                } else {
                    None
                }
            }
            None => {
                // Thwart timing analysis by running dummy verification against first user
                if let Some(first) = guard.users_by_name.values().next() {
                    if let Ok(parsed) = PasswordHash::new(&first.password_hash) {
                        let _ = Pbkdf2.verify_password(password.as_bytes(), &parsed);
                    }
                }
                None
            }
        }
    }

    pub fn has_user(&self, username: &str) -> bool {
        self.inner
            .read()
            .map(|g| g.users_by_name.contains_key(username))
            .unwrap_or(false)
    }

    pub fn get_by_name(&self, username: &str) -> Option<UserCredentials> {
        self.inner
            .read()
            .ok()?
            .users_by_name
            .get(username)
            .cloned()
    }

    pub fn get_by_hkey(&self, hkey: &str) -> Option<UserCredentials> {
        self.inner
            .read()
            .ok()?
            .users_by_hkey
            .get(hkey)
            .cloned()
    }

    pub fn is_guest(&self, username: &str) -> bool {
        if username.starts_with("guest_") {
            return true;
        }
        self.inner
            .read()
            .map(|g| {
                g.stored_records
                    .get(username)
                    .map(|r| r.is_guest)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    pub fn list_all(&self) -> Vec<UserCredentials> {
        self.inner
            .read()
            .map(|g| g.users_by_name.values().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_user_registry_registration_and_persistence() {
        let dir = tempdir().unwrap();
        let static_user = UserCredentials {
            username: "admin_env".to_string(),
            password_hash: hash_password("admin_pass").unwrap(),
            hkey: derive_hkey("admin_env:admin_pass"),
        };

        let reg = UserRegistry::new(dir.path(), vec![static_user.clone()]).unwrap();
        assert!(reg.has_user("admin_env"));
        assert!(reg.verify_credentials("admin_env", "admin_pass").is_some());
        assert!(reg.verify_credentials("admin_env", "wrong").is_none());

        // Register new user
        let bob = reg.register("bob_builder", "secret123").unwrap();
        assert_eq!(bob.username, "bob_builder");
        assert!(reg.has_user("bob_builder"));
        assert!(reg.verify_credentials("bob_builder", "secret123").is_some());

        // Duplicate rejection
        assert!(reg.register("bob_builder", "another_pass").is_err());
        assert!(reg.register("BOB_BUILDER", "another_pass").is_err());

        // Reserved name rejection
        assert!(reg.register("demo", "password123").is_err());
        assert!(reg.register("admin", "password123").is_err());

        // Reload from disk in a fresh instance
        let reg2 = UserRegistry::new(dir.path(), vec![static_user]).unwrap();
        assert!(reg2.has_user("bob_builder"));
        assert!(reg2.verify_credentials("bob_builder", "secret123").is_some());
    }

    #[test]
    fn test_ephemeral_guest_creation() {
        let dir = tempdir().unwrap();
        let reg = UserRegistry::new(dir.path(), vec![]).unwrap();
        let guest = reg.register_ephemeral_guest().unwrap();
        assert!(guest.username.starts_with("guest_"));
        assert!(reg.has_user(&guest.username));
        assert!(reg.is_guest(&guest.username));
    }
}
