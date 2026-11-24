// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl-3.0.html

use std::collections::HashMap;
use std::env;
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use axum_client_ip::ClientIpSource;
use pbkdf2::password_hash::PasswordHasher;
use pbkdf2::password_hash::SaltString;
use pbkdf2::Pbkdf2;
use sha1::Digest as _;
use sha1::Sha1;

/// Fixed salt matching upstream Anki SimpleServer when plaintext passwords are supplied in env.
const FIXED_SALT_B64: &str = "tonuvYGpksNFQBlEmm3lxg";

#[derive(Debug, Clone)]
pub struct UserCredentials {
    pub username: String,
    pub password_hash: String,
    /// Host key used by Anki's built-in sync protocol (`/sync/*`, `/msync/*`).
    pub hkey: String,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub data_dir: PathBuf,
    pub upstream_web_dir: PathBuf,
    pub shell_dir: Option<PathBuf>,
    pub session_secret: Vec<u8>,
    pub cookie_secure: bool,
    pub max_upload_bytes: usize,
    pub ip_source: ClientIpSource,
    pub users: Vec<UserCredentials>,
    pub users_by_name: HashMap<String, usize>,
    pub users_by_hkey: HashMap<String, usize>,
    pub demo_mode: bool,
    pub demo_user: Option<String>,
    pub demo_reset_interval_secs: Option<u64>,
    pub demo_ai_enabled: bool,
    pub ai_server_key: Option<String>,
    pub ai_global_daily_limit: u32,
    pub ai_ip_daily_limit: u32,
}

/// Minimal `.env` loader that does not overwrite variables already set in the environment.
pub fn load_dotenv(path: &Path) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || env::var_os(key).is_some() {
            continue;
        }
        let mut val = val.trim().to_string();
        if (val.starts_with('"') && val.ends_with('"') && val.len() >= 2)
            || (val.starts_with('\'') && val.ends_with('\'') && val.len() >= 2)
        {
            val = val[1..val.len() - 1].to_string();
        }
        // Safe: called once at startup before multi-threaded work begins.
        unsafe {
            env::set_var(key, val);
        }
    }
}

pub fn validate_username(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 64 {
        bail!("Invalid username '{name}': must be 1..=64 characters");
    }
    if name == "." || name == ".." {
        bail!("Invalid username '{name}'");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        bail!("Invalid username '{name}': only ASCII letters, digits, '_', '-', '.' allowed");
    }
    Ok(())
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::from_b64(FIXED_SALT_B64).map_err(|e| anyhow!("invalid salt: {e}"))?;
    let hash = Pbkdf2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("hashing password: {e}"))?
        .to_string();
    Ok(hash)
}

pub fn derive_hkey(user_and_pass: &str) -> String {
    hex::encode(Sha1::digest(user_and_pass.as_bytes()))
}

impl ServerConfig {
    pub fn from_env() -> Result<Self> {
        let host: IpAddr = env::var("CARDLY_HOST")
            .or_else(|_| env::var("SYNC_HOST"))
            .unwrap_or_else(|_| "0.0.0.0".to_string())
            .parse()
            .context("parsing CARDLY_HOST")?;

        let port: u16 = env::var("CARDLY_PORT")
            .or_else(|_| env::var("SYNC_PORT"))
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .context("parsing CARDLY_PORT")?;

        let data_dir = PathBuf::from(
            env::var("CARDLY_DATA_DIR")
                .or_else(|_| env::var("SYNC_BASE"))
                .unwrap_or_else(|_| "./data".to_string()),
        );
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("creating data dir {}", data_dir.display()))?;

        let upstream_web_dir = PathBuf::from(
            env::var("CARDLY_UPSTREAM_WEB_DIR")
                .unwrap_or_else(|_| "../anki-upstream/out/qt/_aqt/data/web".to_string()),
        );

        let shell_dir = env::var("CARDLY_SHELL_DIR")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                ["./web/dist", "../web/dist"]
                    .iter()
                    .map(PathBuf::from)
                    .find(|p| p.join("index.html").is_file())
            });

        let session_secret = load_or_create_session_secret(&data_dir)?;

        let cookie_secure = env::var("CARDLY_COOKIE_SECURE")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
            .unwrap_or(false);

        let max_upload_mb: usize = env::var("CARDLY_MAX_UPLOAD_MB")
            .unwrap_or_else(|_| "512".to_string())
            .parse()
            .context("parsing CARDLY_MAX_UPLOAD_MB")?;

        let passwords_hashed = env::var("PASSWORDS_HASHED")
            .map(|v| !v.is_empty() && v != "0" && v.to_lowercase() != "false")
            .unwrap_or(false);

        let demo_mode = env::var("CARDLY_DEMO_MODE")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
            .unwrap_or(false);

        let demo_user_raw = env::var("CARDLY_DEMO_USER")
            .unwrap_or_else(|_| "demo:demo123".to_string());

        let demo_reset_interval_secs: u64 = env::var("CARDLY_DEMO_RESET_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7200);

        let demo_ai_enabled = env::var("CARDLY_DEMO_AI_ENABLED")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
            .unwrap_or(false);

        let ai_server_key = env::var("CARDLY_AI_SERVER_KEY")
            .or_else(|_| env::var("CARDLY_GEMINI_API_KEY"))
            .or_else(|_| env::var("CARDLY_OPENAI_API_KEY"))
            .ok()
            .filter(|k| !k.trim().is_empty());

        let ai_global_daily_limit: u32 = env::var("CARDLY_AI_GLOBAL_DAILY_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);

        let ai_ip_daily_limit: u32 = env::var("CARDLY_AI_IP_DAILY_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let mut raw_users = collect_users_from_env()?;
        let mut demo_user_name = None;

        if demo_mode {
            let (name, _) = demo_user_raw
                .split_once(':')
                .unwrap_or(("demo", "demo123"));
            let name = name.trim();
            demo_user_name = Some(name.to_string());
            if !raw_users.iter().any(|u| u.starts_with(&format!("{name}:"))) {
                raw_users.push(demo_user_raw);
            }
        }

        // Audit for weak default passwords when not in demo mode
        if !demo_mode {
            const WEAK_PASSWORDS: &[&str] = &[
                "change-me-please",
                "cardly123",
                "password",
                "123456",
                "12345678",
                "admin",
                "secret",
            ];
            for raw in &raw_users {
                if let Some((uname, secret)) = raw.split_once(':') {
                    if WEAK_PASSWORDS.contains(&secret.trim()) {
                        tracing::warn!(
                            "⚠️ SECURITY WARNING: User '{}' is configured with default/weak password '{}'. Please update CARDLY_USER1 in .env with a strong secret before public deployment!",
                            uname.trim(),
                            secret.trim()
                        );
                    }
                }
            }
        }

        let users = parse_users(&raw_users, passwords_hashed)?;

        Self::build_with_all_options(
            host,
            port,
            data_dir,
            upstream_web_dir,
            shell_dir,
            session_secret,
            cookie_secure,
            max_upload_mb * 1024 * 1024,
            ClientIpSource::ConnectInfo,
            users,
            demo_mode,
            demo_user_name,
            Some(demo_reset_interval_secs),
            demo_ai_enabled,
            ai_server_key,
            ai_global_daily_limit,
            ai_ip_daily_limit,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_with_users(
        host: IpAddr,
        port: u16,
        data_dir: PathBuf,
        upstream_web_dir: PathBuf,
        shell_dir: Option<PathBuf>,
        session_secret: Vec<u8>,
        cookie_secure: bool,
        max_upload_bytes: usize,
        ip_source: ClientIpSource,
        users: Vec<UserCredentials>,
    ) -> Result<Self> {
        Self::build_with_all_options(
            host,
            port,
            data_dir,
            upstream_web_dir,
            shell_dir,
            session_secret,
            cookie_secure,
            max_upload_bytes,
            ip_source,
            users,
            false,
            None,
            None,
            false,
            None,
            50,
            10,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_with_all_options(
        host: IpAddr,
        port: u16,
        data_dir: PathBuf,
        upstream_web_dir: PathBuf,
        shell_dir: Option<PathBuf>,
        session_secret: Vec<u8>,
        cookie_secure: bool,
        max_upload_bytes: usize,
        ip_source: ClientIpSource,
        users: Vec<UserCredentials>,
        demo_mode: bool,
        demo_user: Option<String>,
        demo_reset_interval_secs: Option<u64>,
        demo_ai_enabled: bool,
        ai_server_key: Option<String>,
        ai_global_daily_limit: u32,
        ai_ip_daily_limit: u32,
    ) -> Result<Self> {
        if users.is_empty() {
            bail!("No users configured; set CARDLY_USER1=username:password (or SYNC_USER1).");
        }
        let mut users_by_name = HashMap::new();
        let mut users_by_hkey = HashMap::new();
        for (idx, u) in users.iter().enumerate() {
            validate_username(&u.username)?;
            if users_by_name.insert(u.username.clone(), idx).is_some() {
                bail!("Duplicate username configured: {}", u.username);
            }
            users_by_hkey.insert(u.hkey.clone(), idx);
        }
        Ok(Self {
            host,
            port,
            data_dir,
            upstream_web_dir,
            shell_dir,
            session_secret,
            cookie_secure,
            max_upload_bytes,
            ip_source,
            users,
            users_by_name,
            users_by_hkey,
            demo_mode,
            demo_user,
            demo_reset_interval_secs,
            demo_ai_enabled,
            ai_server_key,
            ai_global_daily_limit,
            ai_ip_daily_limit,
        })
    }
}

fn collect_users_from_env() -> Result<Vec<String>> {
    let mut entries = Vec::new();
    for prefix in ["CARDLY_USER", "SYNC_USER"] {
        let mut idx = 1usize;
        loop {
            let key = format!("{prefix}{idx}");
            match env::var(&key) {
                Ok(val) if !val.trim().is_empty() => {
                    entries.push(val);
                    idx += 1;
                }
                _ => break,
            }
        }
        if !entries.is_empty() {
            break;
        }
    }
    Ok(entries)
}

pub fn parse_users(raw_entries: &[String], passwords_hashed: bool) -> Result<Vec<UserCredentials>> {
    let mut users = Vec::new();
    for val in raw_entries {
        let (name, secret) = val
            .split_once(':')
            .ok_or_else(|| anyhow!("User entry must be in 'username:password' format"))?;
        let name = name.trim();
        validate_username(name)?;
        let hkey = derive_hkey(val);
        let password_hash = if passwords_hashed {
            secret.to_string()
        } else {
            hash_password(secret)?
        };
        users.push(UserCredentials {
            username: name.to_string(),
            password_hash,
            hkey,
        });
    }
    Ok(users)
}

fn load_or_create_session_secret(data_dir: &Path) -> Result<Vec<u8>> {
    if let Ok(secret) = env::var("CARDLY_SESSION_SECRET") {
        let trimmed = secret.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.as_bytes().to_vec());
        }
    }
    let secret_path = data_dir.join(".session_secret");
    if let Ok(existing) = fs::read_to_string(&secret_path) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.as_bytes().to_vec());
        }
    }
    let mut raw = [0u8; 32];
    rand::fill(&mut raw);
    let encoded = hex::encode(raw);
    fs::write(&secret_path, &encoded)
        .with_context(|| format!("writing session secret to {}", secret_path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&secret_path, fs::Permissions::from_mode(0o600));
    }
    Ok(encoded.into_bytes())
}
