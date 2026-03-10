use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension};

use crate::provider_settings::{
    model::{ProviderKind, ProviderProfileRecord, ProviderProfileSummary},
    ProviderSettingsError,
};

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS provider_profiles (
    id TEXT PRIMARY KEY,
    provider_kind TEXT NOT NULL,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    model TEXT NOT NULL,
    api_key_ciphertext BLOB NOT NULL,
    api_key_nonce BLOB NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_provider_profiles_single_default
ON provider_profiles(is_default)
WHERE is_default = 1;
"#;

#[derive(Debug, Clone)]
pub struct ProviderProfileStore {
    db_path: PathBuf,
}

impl ProviderProfileStore {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ProviderSettingsError> {
        let db_path = path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| ProviderSettingsError::Validation(err.to_string()))?;
        }

        let store = Self { db_path };
        store.init_schema()?;
        Ok(store)
    }

    pub(crate) fn list_profiles(
        &self,
    ) -> Result<Vec<ProviderProfileSummary>, ProviderSettingsError> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, provider_kind, name, base_url, model, is_default
             FROM provider_profiles
             ORDER BY is_default DESC, updated_at DESC, name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let provider_kind = row.get::<_, String>(1)?;
            Ok(ProviderProfileSummary {
                id: row.get(0)?,
                provider_kind: ProviderKind::parse(&provider_kind).ok_or_else(|| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::<dyn std::error::Error + Send + Sync>::from("invalid provider kind"),
                    )
                })?,
                name: row.get(2)?,
                base_url: row.get(3)?,
                model: row.get(4)?,
                is_default: row.get::<_, i64>(5)? == 1,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(ProviderSettingsError::from)
    }

    pub(crate) fn load_profile(
        &self,
        profile_id: &str,
    ) -> Result<Option<ProviderProfileRecord>, ProviderSettingsError> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, provider_kind, name, base_url, model, api_key_ciphertext, api_key_nonce,
                    is_default, created_at, updated_at
             FROM provider_profiles
             WHERE id = ?1",
        )?;

        stmt.query_row([profile_id], map_record_row)
            .optional()
            .map_err(ProviderSettingsError::from)
    }

    pub(crate) fn load_default_profile(
        &self,
    ) -> Result<Option<ProviderProfileRecord>, ProviderSettingsError> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, provider_kind, name, base_url, model, api_key_ciphertext, api_key_nonce,
                    is_default, created_at, updated_at
             FROM provider_profiles
             WHERE is_default = 1
             LIMIT 1",
        )?;

        stmt.query_row([], map_record_row)
            .optional()
            .map_err(ProviderSettingsError::from)
    }

    pub(crate) fn has_default(&self) -> Result<bool, ProviderSettingsError> {
        let conn = self.connection()?;
        let exists = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM provider_profiles WHERE is_default = 1)",
            [],
            |row| row.get::<_, i64>(0),
        )?;

        Ok(exists == 1)
    }

    pub(crate) fn save_profile(
        &self,
        record: &ProviderProfileRecord,
    ) -> Result<(), ProviderSettingsError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;

        if record.is_default {
            tx.execute(
                "UPDATE provider_profiles SET is_default = 0 WHERE is_default = 1 AND id != ?1",
                [&record.id],
            )?;
        }

        let exists = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM provider_profiles WHERE id = ?1)",
            [&record.id],
            |row| row.get::<_, i64>(0),
        )? == 1;

        if exists {
            tx.execute(
                "UPDATE provider_profiles
                 SET provider_kind = ?2,
                     name = ?3,
                     base_url = ?4,
                     model = ?5,
                     api_key_ciphertext = ?6,
                     api_key_nonce = ?7,
                     is_default = ?8,
                     updated_at = ?9
                 WHERE id = ?1",
                params![
                    record.id,
                    record.provider_kind.as_str(),
                    record.name,
                    record.base_url,
                    record.model,
                    record.api_key_ciphertext,
                    record.api_key_nonce,
                    bool_to_int(record.is_default),
                    record.updated_at,
                ],
            )?;
        } else {
            tx.execute(
                "INSERT INTO provider_profiles (
                    id, provider_kind, name, base_url, model, api_key_ciphertext,
                    api_key_nonce, is_default, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    record.id,
                    record.provider_kind.as_str(),
                    record.name,
                    record.base_url,
                    record.model,
                    record.api_key_ciphertext,
                    record.api_key_nonce,
                    bool_to_int(record.is_default),
                    record.created_at,
                    record.updated_at,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub(crate) fn set_default_profile(
        &self,
        profile_id: &str,
    ) -> Result<ProviderProfileRecord, ProviderSettingsError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let updated = tx.execute(
            "UPDATE provider_profiles SET is_default = 1 WHERE id = ?1",
            [profile_id],
        )?;

        if updated == 0 {
            return Err(ProviderSettingsError::NotFound(format!(
                "provider profile `{profile_id}` not found"
            )));
        }

        tx.execute(
            "UPDATE provider_profiles SET is_default = 0 WHERE id != ?1 AND is_default = 1",
            [profile_id],
        )?;
        tx.commit()?;

        self.load_profile(profile_id)?.ok_or_else(|| {
            ProviderSettingsError::NotFound(format!("provider profile `{profile_id}` not found"))
        })
    }

    pub(crate) fn delete_profile(&self, profile_id: &str) -> Result<(), ProviderSettingsError> {
        let conn = self.connection()?;
        let deleted = conn.execute("DELETE FROM provider_profiles WHERE id = ?1", [profile_id])?;

        if deleted == 0 {
            return Err(ProviderSettingsError::NotFound(format!(
                "provider profile `{profile_id}` not found"
            )));
        }

        Ok(())
    }

    fn init_schema(&self) -> Result<(), ProviderSettingsError> {
        let conn = self.connection()?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(())
    }

    fn connection(&self) -> Result<Connection, ProviderSettingsError> {
        Connection::open(&self.db_path).map_err(ProviderSettingsError::from)
    }
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn map_record_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderProfileRecord> {
    let provider_kind = row.get::<_, String>(1)?;
    let provider_kind = ProviderKind::parse(&provider_kind).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            Box::<dyn std::error::Error + Send + Sync>::from("invalid provider kind"),
        )
    })?;

    Ok(ProviderProfileRecord {
        id: row.get(0)?,
        provider_kind,
        name: row.get(2)?,
        base_url: row.get(3)?,
        model: row.get(4)?,
        api_key_ciphertext: row.get(5)?,
        api_key_nonce: row.get(6)?,
        is_default: row.get::<_, i64>(7)? == 1,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}
