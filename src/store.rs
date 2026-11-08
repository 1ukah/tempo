use std::env::var_os;
use std::fs;
use std::io;
use std::path::PathBuf;

const TOKEN_EXT: &str = "token";
const STATUS_EXT: &str = "status";

pub fn dir() -> PathBuf {
    if let Some(custom) = var_os("TEMPO_HOME").filter(|v| !v.is_empty()) {
        return custom.into();
    }

    let base: PathBuf = if cfg!(windows) {
        var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| ".".into())
    } else if cfg!(target_os = "macos") {
        home().join("Library/Application Support")
    } else {
        var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"))
    };

    base.join("tempo")
}

fn home() -> PathBuf {
    var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| ".".into())
}

pub fn guard_data_path() -> PathBuf {
    dir().join("guard.json")
}

fn token_path(user: &str) -> PathBuf {
    account_path(user, TOKEN_EXT)
}

fn account_path(user: &str, ext: &str) -> PathBuf {
    let safe: String = user
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();

    dir().join(format!("{safe}.{ext}"))
}

pub fn load_token(user: &str) -> Option<String> {
    fs::read_to_string(token_path(user))
        .ok()
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
}

pub fn save_token(user: &str, token: &str) -> io::Result<()> {
    fs::create_dir_all(dir())?;

    let path = token_path(user);
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, token)?;
    restrict(&tmp);
    fs::rename(&tmp, &path)?;
    restrict(&guard_data_path());

    Ok(())
}

pub fn load_status(user: &str) -> Option<String> {
    fs::read_to_string(account_path(user, STATUS_EXT))
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

pub fn save_status(user: &str, status: &str) -> io::Result<()> {
    fs::create_dir_all(dir())?;
    fs::write(account_path(user, STATUS_EXT), status)
}

pub fn delete_token(user: &str) -> bool {
    fs::remove_file(token_path(user)).is_ok()
}

pub fn saved_users() -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir()) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == TOKEN_EXT))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect()
}

fn restrict(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }

    #[cfg(not(unix))]
    let _ = path;
}
