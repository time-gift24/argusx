PRAGMA foreign_keys = ON;

CREATE TABLE sops (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sop_id TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  ticket_id TEXT,
  version INTEGER NOT NULL DEFAULT 1,
  detect TEXT CHECK (detect IS NULL OR json_valid(detect)),
  handle TEXT CHECK (handle IS NULL OR json_valid(handle)),
  verification TEXT CHECK (verification IS NULL OR json_valid(verification)),
  rollback TEXT CHECK (rollback IS NULL OR json_valid(rollback)),
  status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'inactive', 'draft')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX idx_sops_status ON sops(status);
CREATE INDEX idx_sops_ticket ON sops(ticket_id);

CREATE TABLE sop_steps (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sop_id TEXT NOT NULL,
  name TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  operation TEXT,
  verification TEXT,
  impact_analysis TEXT,
  rollback TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (sop_id) REFERENCES sops(sop_id)
);

CREATE INDEX idx_sop_steps_sop ON sop_steps(sop_id);

CREATE TABLE checklist_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  prompt TEXT NOT NULL,
  temperature REAL DEFAULT 0.0,
  context_type TEXT NOT NULL
    CHECK (context_type IN (
      'sop', 'sop_procedure_detect', 'sop_procedure_handle',
      'sop_procedure_verification', 'sop_procedure_rollback',
      'sop_step_operation', 'sop_step_verification',
      'sop_step_impact_analysis', 'sop_step_rollback', 'sop_step_common'
    )),
  result_schema TEXT CHECK (result_schema IS NULL OR json_valid(result_schema)),
  version INTEGER NOT NULL DEFAULT 1,
  status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'inactive', 'draft')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  deleted_at INTEGER
);

CREATE INDEX idx_checklist_status ON checklist_items(status);
CREATE INDEX idx_checklist_context_type ON checklist_items(context_type);

CREATE TABLE check_results (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  context_type TEXT NOT NULL,
  context_key TEXT NOT NULL,
  check_item_id INTEGER,
  source_type INTEGER NOT NULL DEFAULT 1 CHECK (source_type IN (1, 2)),
  operator_id TEXT,
  result TEXT CHECK (result IS NULL OR json_valid(result)),
  is_pass INTEGER NOT NULL DEFAULT 0 CHECK (is_pass IN (0, 1)),
  created_at INTEGER NOT NULL,
  FOREIGN KEY (check_item_id) REFERENCES checklist_items(id)
);

CREATE INDEX idx_check_results_item ON check_results(check_item_id);
CREATE INDEX idx_check_results_context ON check_results(context_type, context_key);

CREATE UNIQUE INDEX idx_check_results_manual_latest
ON check_results (context_type, context_key, check_item_id)
WHERE source_type = 2 AND check_item_id IS NOT NULL;

CREATE TABLE ai_execution_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  check_result_id INTEGER,
  context_type TEXT NOT NULL,
  context_key TEXT NOT NULL,
  check_item_id INTEGER NOT NULL,
  model_provider TEXT,
  model_version TEXT NOT NULL,
  temperature REAL DEFAULT 0.0,
  prompt_snapshot TEXT,
  raw_output TEXT,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  exec_status INTEGER NOT NULL DEFAULT 0 CHECK (exec_status IN (0, 1, 2, 3)),
  error_message TEXT,
  latency_ms INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  FOREIGN KEY (check_result_id) REFERENCES check_results(id),
  FOREIGN KEY (check_item_id) REFERENCES checklist_items(id)
);

CREATE INDEX idx_ai_logs_item ON ai_execution_logs(check_item_id);
CREATE INDEX idx_ai_logs_context ON ai_execution_logs(context_type, context_key);

CREATE TABLE golden_set_items (
  golden_set_id INTEGER NOT NULL,
  checklist_item_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  PRIMARY KEY (golden_set_id, checklist_item_id),
  FOREIGN KEY (golden_set_id) REFERENCES check_results(id),
  FOREIGN KEY (checklist_item_id) REFERENCES checklist_items(id)
);

CREATE INDEX idx_gsi_item ON golden_set_items(checklist_item_id);
