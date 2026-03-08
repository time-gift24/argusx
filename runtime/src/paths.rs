use std::path::{Path, PathBuf};

pub fn ensure_app_home(app_home: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(app_home)?;
    Ok(())
}

pub fn resolve_path(raw: &Path, app_home: &Path) -> PathBuf {
    let raw_str = raw.to_string_lossy();
    if raw_str == "~/.argusx/sqlite.db" {
        return app_home.join("sqlite.db");
    }
    if raw_str == "~/.argusx/argusx.log" {
        return app_home.join("argusx.log");
    }
    if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        app_home.join(raw)
    }
}
