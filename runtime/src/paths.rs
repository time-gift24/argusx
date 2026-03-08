use std::path::{Path, PathBuf};

pub fn ensure_app_home(app_home: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(app_home)?;
    Ok(())
}

pub fn resolve_path(raw: &Path, app_home: &Path) -> PathBuf {
    let raw_str = raw.to_string_lossy();
    let home = app_home.parent().unwrap_or(app_home);

    if raw_str == "~" {
        return home.to_path_buf();
    }
    if let Some(stripped) = raw_str
        .strip_prefix("~/")
        .or_else(|| raw_str.strip_prefix("~\\"))
    {
        return home.join(stripped);
    }
    if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        app_home.join(raw)
    }
}
