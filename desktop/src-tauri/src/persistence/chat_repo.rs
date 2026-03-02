use std::path::PathBuf;

use agent_core::{InputPart, TranscriptItem};
use rusqlite::ToSql;
use thiserror::Error;

use super::{open_and_bootstrap, SchemaError};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMessageRange {
    Last24Hours,
    All,
}

#[derive(Debug, Clone, Copy)]
pub struct ChatMessageQuery {
    pub range: ChatMessageRange,
    pub cursor: Option<i64>,
    pub limit: usize,
}

impl Default for ChatMessageQuery {
    fn default() -> Self {
        Self {
            range: ChatMessageRange::Last24Hours,
            cursor: None,
            limit: 300,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
    pub cursor: i64,
}

#[derive(Debug, Error)]
pub enum ChatRepoError {
    #[error("schema error: {0}")]
    Schema(#[from] SchemaError),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub struct ChatRepo {
    db_path: PathBuf,
}

impl ChatRepo {
    pub fn new(db_path: PathBuf) -> Result<Self, ChatRepoError> {
        let _ = open_and_bootstrap(&db_path)?;
        Ok(Self { db_path })
    }

    fn open_connection(&self) -> Result<rusqlite::Connection, ChatRepoError> {
        open_and_bootstrap(&self.db_path).map_err(Into::into)
    }

    pub fn list_messages(
        &self,
        session_id: &str,
        query: ChatMessageQuery,
    ) -> Result<Vec<StoredChatMessage>, ChatRepoError> {
        let mut sql = String::from(
            r#"
SELECT
  ti.id,
  ti.session_id,
  ti.turn_id,
  ti.seq,
  ti.payload_json,
  COALESCE(t.ended_at_ms, t.started_at_ms, tc.started_at_ms, 0) AS turn_time
FROM transcript_items ti
LEFT JOIN turns t ON t.turn_id = ti.turn_id AND t.session_id = ti.session_id
LEFT JOIN turn_contexts tc ON tc.turn_id = ti.turn_id AND tc.session_id = ti.session_id
WHERE ti.session_id = ?
"#,
        );
        let mut binds: Vec<Box<dyn ToSql>> = vec![Box::new(session_id.to_string())];

        if matches!(query.range, ChatMessageRange::Last24Hours) {
            let since = chrono::Utc::now().timestamp_millis() - DAY_MS;
            sql.push_str(" AND COALESCE(t.ended_at_ms, t.started_at_ms, tc.started_at_ms, 0) >= ? ");
            binds.push(Box::new(since));
        }

        if let Some(cursor) = query.cursor {
            sql.push_str(" AND ti.id < ? ");
            binds.push(Box::new(cursor));
        }

        sql.push_str(" ORDER BY ti.id DESC LIMIT ? ");
        binds.push(Box::new(query.limit.max(1) as i64));

        let conn = self.open_connection()?;
        let bind_refs: Vec<&dyn ToSql> = binds.iter().map(|value| value.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(bind_refs.as_slice())?;

        let mut messages = Vec::new();
        while let Some(row) = rows.next()? {
            let cursor: i64 = row.get(0)?;
            let row_session_id: String = row.get(1)?;
            let seq: i64 = row.get(3)?;
            let payload_json: String = row.get(4)?;
            let turn_time: i64 = row.get(5)?;

            let transcript_item = serde_json::from_str::<TranscriptItem>(&payload_json)?;
            let Some((role, content)) = transcript_item_to_role_and_content(transcript_item) else {
                continue;
            };

            messages.push(StoredChatMessage {
                id: cursor.to_string(),
                session_id: row_session_id,
                role,
                content,
                created_at: turn_time.saturating_add(seq),
                cursor,
            });
        }

        messages.sort_by_key(|message| message.cursor);
        Ok(messages)
    }
}

fn transcript_item_to_role_and_content(item: TranscriptItem) -> Option<(String, String)> {
    match item {
        TranscriptItem::UserMessage { input, .. } => {
            let content = input
                .parts
                .into_iter()
                .map(|part| match part {
                    InputPart::Text { text } => text,
                    InputPart::Json { value } => value.to_string(),
                })
                .filter(|part| !part.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if content.is_empty() {
                return None;
            }
            Some(("user".to_string(), content))
        }
        TranscriptItem::AssistantMessage { text, .. } => Some(("assistant".to_string(), text)),
        TranscriptItem::SystemNote { message, .. } => Some(("system".to_string(), message)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_core::InputEnvelope;
    use rusqlite::params;
    use tempfile::tempdir;

    fn insert_session(
        conn: &rusqlite::Connection,
        session_id: &str,
        updated_at_ms: i64,
    ) -> rusqlite::Result<()> {
        conn.execute(
            r#"
INSERT INTO sessions (
  session_id, user_id, parent_id, title, status, created_at_ms, updated_at_ms, archived_at_ms
) VALUES (?1, NULL, NULL, ?2, 'idle', ?3, ?4, NULL)
"#,
            params![
                session_id,
                format!("Session {session_id}"),
                updated_at_ms - 100,
                updated_at_ms
            ],
        )?;
        Ok(())
    }

    fn insert_turn(
        conn: &rusqlite::Connection,
        session_id: &str,
        turn_id: &str,
        started_at_ms: i64,
        ended_at_ms: i64,
    ) -> rusqlite::Result<()> {
        conn.execute(
            r#"
INSERT INTO turns (
  turn_id, session_id, epoch, started_at_ms, ended_at_ms, status, final_message,
  tool_calls_count, input_tokens, output_tokens
) VALUES (?1, ?2, 0, ?3, ?4, 'done', NULL, 0, 0, 0)
"#,
            params![turn_id, session_id, started_at_ms, ended_at_ms],
        )?;
        Ok(())
    }

    fn insert_turn_context(
        conn: &rusqlite::Connection,
        session_id: &str,
        turn_id: &str,
        started_at_ms: i64,
    ) -> rusqlite::Result<()> {
        conn.execute(
            r#"
INSERT INTO turn_contexts (turn_id, session_id, epoch, started_at_ms)
VALUES (?1, ?2, 0, ?3)
"#,
            params![turn_id, session_id, started_at_ms],
        )?;
        Ok(())
    }

    fn insert_transcript_item(
        conn: &rusqlite::Connection,
        session_id: &str,
        turn_id: &str,
        seq: i64,
        item: TranscriptItem,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO transcript_items (session_id, turn_id, seq, payload_json) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, turn_id, seq, serde_json::to_string(&item).expect("serialize transcript item")],
        )?;
        Ok(())
    }

    #[test]
    fn chat_repo_default_query_loads_only_last_24h() {
        let temp = tempdir().expect("create tempdir");
        let db_path = temp.path().join("chat.db");
        let repo = ChatRepo::new(db_path.clone()).expect("create repo");
        let conn = open_and_bootstrap(&db_path).expect("open db");
        let now = chrono::Utc::now().timestamp_millis();
        let older = now - (DAY_MS * 2);
        let recent = now - (60 * 60 * 1000);

        insert_session(&conn, "s1", now).expect("insert session");
        insert_turn(&conn, "s1", "t-old", older, older + 10).expect("insert old turn");
        insert_turn(&conn, "s1", "t-new", recent, recent + 10).expect("insert recent turn");
        insert_transcript_item(
            &conn,
            "s1",
            "t-old",
            0,
            TranscriptItem::user_message(InputEnvelope::user_text("old message")),
        )
        .expect("insert old transcript");
        insert_transcript_item(
            &conn,
            "s1",
            "t-new",
            0,
            TranscriptItem::assistant_message("recent message"),
        )
        .expect("insert recent transcript");

        let messages = repo
            .list_messages("s1", ChatMessageQuery::default())
            .expect("query messages");

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "assistant");
        assert_eq!(messages[0].content, "recent message");
    }

    #[test]
    fn chat_repo_all_query_supports_cursor_pagination() {
        let temp = tempdir().expect("create tempdir");
        let db_path = temp.path().join("chat.db");
        let repo = ChatRepo::new(db_path.clone()).expect("create repo");
        let conn = open_and_bootstrap(&db_path).expect("open db");
        let now = chrono::Utc::now().timestamp_millis();

        insert_session(&conn, "s1", now).expect("insert session");
        insert_turn(&conn, "s1", "t1", now - 100, now - 10).expect("insert turn");
        insert_transcript_item(
            &conn,
            "s1",
            "t1",
            0,
            TranscriptItem::assistant_message("m1"),
        )
        .expect("insert m1");
        insert_transcript_item(
            &conn,
            "s1",
            "t1",
            1,
            TranscriptItem::assistant_message("m2"),
        )
        .expect("insert m2");
        insert_transcript_item(
            &conn,
            "s1",
            "t1",
            2,
            TranscriptItem::assistant_message("m3"),
        )
        .expect("insert m3");

        let first_page = repo
            .list_messages(
                "s1",
                ChatMessageQuery {
                    range: ChatMessageRange::All,
                    cursor: None,
                    limit: 2,
                },
            )
            .expect("query first page");

        assert_eq!(
            first_page
                .iter()
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>(),
            vec!["m2", "m3"]
        );

        let cursor = first_page.first().expect("first page not empty").cursor;
        let second_page = repo
            .list_messages(
                "s1",
                ChatMessageQuery {
                    range: ChatMessageRange::All,
                    cursor: Some(cursor),
                    limit: 2,
                },
            )
            .expect("query second page");

        assert_eq!(second_page.len(), 1);
        assert_eq!(second_page[0].content, "m1");
    }

    #[test]
    fn chat_repo_default_query_includes_running_turn_context_within_24h() {
        let temp = tempdir().expect("create tempdir");
        let db_path = temp.path().join("chat.db");
        let repo = ChatRepo::new(db_path.clone()).expect("create repo");
        let conn = open_and_bootstrap(&db_path).expect("open db");
        let now = chrono::Utc::now().timestamp_millis();
        let recent = now - (15 * 60 * 1000);

        insert_session(&conn, "s1", now).expect("insert session");
        insert_turn_context(&conn, "s1", "t-running", recent).expect("insert running turn context");
        insert_transcript_item(
            &conn,
            "s1",
            "t-running",
            0,
            TranscriptItem::user_message(InputEnvelope::user_text("running message")),
        )
        .expect("insert running transcript");

        let messages = repo
            .list_messages("s1", ChatMessageQuery::default())
            .expect("query messages");

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "running message");
    }
}
