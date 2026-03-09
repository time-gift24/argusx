use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub port: u16,
    pub chrome_path: Option<String>,
    pub profile_dir: Option<String>,
    pub headless: bool,
    pub is_enabled: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            port: 9222,
            chrome_path: None,
            profile_dir: None,
            headless: false,
            is_enabled: false,
        }
    }
}

pub struct BrowserConfigManager {
    conn: Mutex<Connection>,
}

impl BrowserConfigManager {
    pub fn new(db_path: PathBuf) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS browser_config (
                id INTEGER PRIMARY KEY DEFAULT 1,
                port INTEGER NOT NULL DEFAULT 9222,
                chrome_path TEXT,
                profile_dir TEXT,
                headless INTEGER NOT NULL DEFAULT 0,
                is_enabled INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Insert default config if not exists
        conn.execute(
            "INSERT OR IGNORE INTO browser_config (id, port, headless, is_enabled, updated_at)
             VALUES (1, 9222, 0, 0, strftime('%s', 'now'))",
            [],
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn get_config(&self) -> Result<BrowserConfig, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT port, chrome_path, profile_dir, headless, is_enabled
             FROM browser_config WHERE id = 1"
        )?;

        let config = stmt.query_row([], |row| {
            Ok(BrowserConfig {
                port: row.get(0)?,
                chrome_path: row.get(1)?,
                profile_dir: row.get(2)?,
                headless: row.get::<_, i32>(3)? != 0,
                is_enabled: row.get::<_, i32>(4)? != 0,
            })
        })?;

        Ok(config)
    }

    pub fn update_config(&self, config: &BrowserConfig) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE browser_config SET
             port = ?1, chrome_path = ?2, profile_dir = ?3,
             headless = ?4, is_enabled = ?5, updated_at = strftime('%s', 'now')
             WHERE id = 1",
            params![
                config.port,
                config.chrome_path,
                config.profile_dir,
                config.headless as i32,
                config.is_enabled as i32,
            ],
        )?;
        Ok(())
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE browser_config SET is_enabled = ?1, updated_at = strftime('%s', 'now') WHERE id = 1",
            params![enabled as i32],
        )?;
        Ok(())
    }
}
