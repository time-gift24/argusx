#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB_PATH="${1:-${ROOT_DIR}/prompt_lab/dev.db}"

mkdir -p "$(dirname "${DB_PATH}")"

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "error: sqlite3 is not installed" >&2
  exit 1
fi

init_schema_via_cli() {
  cargo run -p prompt_lab_cli -- --db "${DB_PATH}" db status >/dev/null 2>&1
}

ensure_schema() {
  if init_schema_via_cli; then
    return
  fi

  if [[ -f "${DB_PATH}" ]]; then
    local backup_path
    backup_path="${DB_PATH}.bak.$(date +%Y%m%d%H%M%S)"
    cp "${DB_PATH}" "${backup_path}"
    rm -f "${DB_PATH}"
    echo "Detected incompatible schema; backed up old DB to: ${backup_path}" >&2
  fi

  if ! init_schema_via_cli; then
    echo "error: failed to initialize v2 schema via prompt_lab_cli" >&2
    exit 1
  fi
}

ensure_schema

NOW_MS="$(( $(date +%s) * 1000 ))"

sqlite3 "${DB_PATH}" <<SQL
PRAGMA foreign_keys = ON;
BEGIN TRANSACTION;

DELETE FROM sop_steps
WHERE sop_id IN ('SOP-MOCK-001', 'SOP-MOCK-002', 'SOP-MOCK-003');

DELETE FROM sops
WHERE sop_id IN ('SOP-MOCK-001', 'SOP-MOCK-002', 'SOP-MOCK-003');

INSERT INTO sops (
  sop_id, name, ticket_id, version, detect, handle, verification, rollback, status, created_at, updated_at
) VALUES
(
  'SOP-MOCK-001',
  'Mock: API Latency Spike',
  'INC-2401',
  1,
  json('[{"sop_step_id":9101001,"name":"Detect: p95 exceeds threshold"}]'),
  json('[{"sop_step_id":9101002,"name":"Handle: scale service replicas"}]'),
  json('[{"sop_step_id":9101003,"name":"Verify: latency recovery check"}]'),
  json('[{"sop_step_id":9101004,"name":"Rollback: revert scaling policy"}]'),
  'active',
  ${NOW_MS},
  ${NOW_MS}
),
(
  'SOP-MOCK-002',
  'Mock: Worker Queue Backlog',
  'INC-2402',
  1,
  json('[{"sop_step_id":9102001,"name":"Detect: queue depth alarm"}]'),
  json('[{"sop_step_id":9102002,"name":"Handle: add temporary workers"}]'),
  json('[{"sop_step_id":9102003,"name":"Verify: backlog drains"}]'),
  json('[{"sop_step_id":9102004,"name":"Rollback: remove temporary workers"}]'),
  'active',
  ${NOW_MS},
  ${NOW_MS}
),
(
  'SOP-MOCK-003',
  'Mock: External API Rate Limit',
  'INC-2403',
  1,
  json('[{"sop_step_id":9103001,"name":"Detect: rate-limit response spike"}]'),
  json('[{"sop_step_id":9103002,"name":"Handle: enable backoff and cache"}]'),
  json('[{"sop_step_id":9103003,"name":"Verify: success ratio recovers"}]'),
  json('[{"sop_step_id":9103004,"name":"Rollback: disable temporary cache"}]'),
  'active',
  ${NOW_MS},
  ${NOW_MS}
);

INSERT INTO sop_steps (
  id, sop_id, name, version, operation, verification, impact_analysis, rollback, created_at, updated_at
) VALUES
(9101001, 'SOP-MOCK-001', 'Detect: p95 exceeds threshold', 1, json('{"check":"latency_p95"}'), NULL, NULL, NULL, ${NOW_MS}, ${NOW_MS}),
(9101002, 'SOP-MOCK-001', 'Handle: scale service replicas', 1, json('{"action":"scale_up"}'), NULL, NULL, NULL, ${NOW_MS}, ${NOW_MS}),
(9101003, 'SOP-MOCK-001', 'Verify: latency recovery check', 1, NULL, json('{"expect":"p95<300ms"}'), NULL, NULL, ${NOW_MS}, ${NOW_MS}),
(9101004, 'SOP-MOCK-001', 'Rollback: revert scaling policy', 1, NULL, NULL, NULL, json('{"action":"scale_restore"}'), ${NOW_MS}, ${NOW_MS}),

(9102001, 'SOP-MOCK-002', 'Detect: queue depth alarm', 1, json('{"check":"queue_depth"}'), NULL, NULL, NULL, ${NOW_MS}, ${NOW_MS}),
(9102002, 'SOP-MOCK-002', 'Handle: add temporary workers', 1, json('{"action":"worker_scale_up"}'), NULL, NULL, NULL, ${NOW_MS}, ${NOW_MS}),
(9102003, 'SOP-MOCK-002', 'Verify: backlog drains', 1, NULL, json('{"expect":"queue_depth_down"}'), NULL, NULL, ${NOW_MS}, ${NOW_MS}),
(9102004, 'SOP-MOCK-002', 'Rollback: remove temporary workers', 1, NULL, NULL, NULL, json('{"action":"worker_scale_restore"}'), ${NOW_MS}, ${NOW_MS}),

(9103001, 'SOP-MOCK-003', 'Detect: rate-limit response spike', 1, json('{"check":"429_ratio"}'), NULL, NULL, NULL, ${NOW_MS}, ${NOW_MS}),
(9103002, 'SOP-MOCK-003', 'Handle: enable backoff and cache', 1, json('{"action":"enable_backoff_cache"}'), NULL, NULL, NULL, ${NOW_MS}, ${NOW_MS}),
(9103003, 'SOP-MOCK-003', 'Verify: success ratio recovers', 1, NULL, json('{"expect":"success_ratio_up"}'), NULL, NULL, ${NOW_MS}, ${NOW_MS}),
(9103004, 'SOP-MOCK-003', 'Rollback: disable temporary cache', 1, NULL, NULL, NULL, json('{"action":"disable_temp_cache"}'), ${NOW_MS}, ${NOW_MS});

COMMIT;
SQL

echo "Seeded mock SOPs into: ${DB_PATH}"
sqlite3 "${DB_PATH}" <<'SQL'
.headers on
.mode column
SELECT sop_id, name, status, version FROM sops WHERE sop_id LIKE 'SOP-MOCK-%' ORDER BY sop_id;
SELECT sop_id, COUNT(*) AS step_count FROM sop_steps WHERE sop_id LIKE 'SOP-MOCK-%' GROUP BY sop_id ORDER BY sop_id;
SQL
