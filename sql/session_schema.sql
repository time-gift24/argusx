CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    default_model TEXT NOT NULL,
    system_prompt TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS threads (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    agent_profile_id TEXT,
    is_subagent INTEGER NOT NULL DEFAULT 0,
    title TEXT,
    lifecycle TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_turn_number INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_threads_session_id ON threads(session_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS turns (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL,
    user_input TEXT NOT NULL,
    status TEXT NOT NULL,
    finish_reason TEXT,
    transcript_json TEXT NOT NULL,
    final_output TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_turns_thread_turn_number ON turns(thread_id, turn_number);
CREATE INDEX IF NOT EXISTS idx_turns_thread_id ON turns(thread_id, turn_number ASC);
CREATE INDEX IF NOT EXISTS idx_turns_status ON turns(status);

CREATE TABLE IF NOT EXISTS thread_agent_snapshots (
    thread_id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    display_name_snapshot TEXT NOT NULL,
    system_prompt_snapshot TEXT NOT NULL,
    tool_policy_snapshot_json TEXT NOT NULL,
    model_config_snapshot_json TEXT NOT NULL,
    allow_subagent_dispatch_snapshot INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS subagent_dispatches (
    id TEXT PRIMARY KEY,
    parent_thread_id TEXT NOT NULL,
    parent_turn_id TEXT NOT NULL,
    dispatch_tool_call_id TEXT NOT NULL,
    child_thread_id TEXT NOT NULL,
    child_agent_profile_id TEXT NOT NULL,
    status TEXT NOT NULL,
    requested_at TEXT NOT NULL,
    finished_at TEXT,
    result_summary TEXT,
    FOREIGN KEY (parent_thread_id) REFERENCES threads(id) ON DELETE CASCADE,
    FOREIGN KEY (child_thread_id) REFERENCES threads(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_subagent_dispatches_parent_thread
    ON subagent_dispatches(parent_thread_id, requested_at DESC);
CREATE INDEX IF NOT EXISTS idx_subagent_dispatches_status
    ON subagent_dispatches(status);
