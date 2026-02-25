# PromptLab Database Design

## Overview

PromptLab uses SQLite as its database. The schema is defined in migration files located at `prompt_lab_core/migrations/`.

## Entity Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                              PromptLab Database Schema                             │
└─────────────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────┐       ┌──────────────────────┐
│   checklist_items    │       │    check_results     │
│   (evaluation        │       │   (evaluation       │
│    prompts)          │       │    results)          │
├──────────────────────┤       ├──────────────────────┤
│ id (PK)              │◄──────│ check_item_id (FK)  │
│ name                 │       │ id (PK)             │
│ prompt               │       │ context_type         │
│ target_level         │       │ context_id           │
│ result_schema        │       │ source_type          │
│ version              │       │ operator_id         │
│ status               │       │ result              │
│ created_at           │       │ is_pass             │
│ updated_at           │       │ created_at           │
│ created_by           │       └──────────┬──────────┘
│ updated_by           │                  │
│ deleted_at           │                  │
└──────────┬───────────┘                  │
           │                              │
           │         ┌────────────────────┴──────────────┐
           │         │                                  │
           ▼         ▼                                  ▼
┌──────────────────────┐       ┌──────────────────────┐       ┌──────────────────────┐
│  golden_set_items    │       │ai_execution_logs     │       │        sops         │
│  (grouped items)     │       │   (LLM audit)       │       │  (procedures)       │
├──────────────────────┤       ├──────────────────────┤       ├──────────────────────┤
│ golden_set_id (FK)   │       │ check_result_id(FK) │       │ id (PK)              │
│ checklist_item_id(FK)│◄──────│ id (PK)             │       │ sop_id (UNIQUE)     │
│ sort_order           │       │ context_type         │       │ name                 │
│ created_at           │       │ context_id          │       │ ticket_id            │
└──────────┬───────────┘       │ check_item_id (FK) │       │ version              │
           │                   │ model_provider      │       │ detect               │
           │                   │ model_version       │       │ handle               │
           │                   │ temperature         │       │ verification         │
           │                   │ prompt_snapshot     │       │ rollback             │
           │                   │ raw_output          │       │ status               │
           │                   │ input_tokens        │       │ created_at           │
           │                   │ output_tokens       │       │ updated_at           │
           │                   │ exec_status         │       └──────────┬──────────┘
           │                   │ error_message       │                  │
           │                   │ latency_ms          │                  │
           │                   │ created_at          │                  │
           │                   └──────────┬───────────┘                  │
           │                              │                              │
           │                              │                              │
           └──────────────────────────────┼──────────────────────────────┘
                                          │
                                          ▼
                               ┌──────────────────────┐
                               │      sop_steps       │
                               │     (steps)          │
                               ├──────────────────────┤
                               │ id (PK)              │
                               │ sop_id (FK)          │
                               │ name                 │
                               │ version              │
                               │ operation            │
                               │ verification         │
                               │ impact_analysis      │
                               │ rollback             │
                               │ created_at           │
                               │ updated_at           │
                               └──────────────────────┘
```

---

## Tables

### 1. checklist_items

Evaluation prompts/checks used to validate SOPs or individual steps.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | INTEGER | PK, AUTOINCREMENT | Primary key |
| `name` | TEXT | NOT NULL | Display name |
| `prompt` | TEXT | NOT NULL | Evaluation prompt (LLM prompt) |
| `target_level` | TEXT | NOT NULL, CHECK | `step` or `sop` - validation scope |
| `result_schema` | TEXT | CHECK (JSON) | JSON schema for expected result |
| `version` | INTEGER | NOT NULL, DEFAULT 1 | Optimistic locking |
| `status` | TEXT | NOT NULL, CHECK | `active`, `inactive`, `draft` |
| `created_at` | TEXT | NOT NULL | ISO8601 timestamp |
| `updated_at` | TEXT | NOT NULL | ISO8601 timestamp |
| `created_by` | INTEGER | NULLABLE | User ID |
| `updated_by` | INTEGER | NULLABLE | User ID |
| `deleted_at` | TEXT | NULLABLE | Soft delete timestamp |

**Indexes:**
- `idx_checklist_status` ON `(status)`
- `idx_checklist_level` ON `(target_level)`

**Triggers:**
- `trg_checklist_items_updated_at` - Auto-update `updated_at` on change

---

### 2. check_results

Evaluation results when prompts are executed against SOPs or steps.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | INTEGER | PK, AUTOINCREMENT | Primary key |
| `context_type` | TEXT | NOT NULL | `sop` or `step` |
| `context_id` | INTEGER | NOT NULL | ID of evaluated SOP/step |
| `check_item_id` | INTEGER | NOT NULL, FK → `checklist_items.id` | Reference to checklist item |
| `source_type` | INTEGER | NOT NULL, CHECK | `1`=AI, `2`=Manual |
| `operator_id` | TEXT | NULLABLE | Operator identifier |
| `result` | TEXT | CHECK (JSON) | JSON evaluation result |
| `is_pass` | INTEGER | NOT NULL, CHECK | `0`=failed, `1`=passed |
| `created_at` | TEXT | NOT NULL | ISO8601 timestamp |

**Indexes:**
- `idx_context_ref` ON `(context_type, context_id)`
- `idx_rule_history` ON `(check_item_id)`

**Note:** This table also serves as the "golden set" container - `golden_set_items.golden_set_id` references `check_results.id`.

---

### 3. golden_set_items

Groups checklist items into "golden sets" for batch evaluation.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `golden_set_id` | INTEGER | PK (composite), FK → `check_results.id` | Acts as set ID |
| `checklist_item_id` | INTEGER | PK (composite), FK → `checklist_items.id` | Item in the set |
| `sort_order` | INTEGER | NOT NULL, DEFAULT 0 | Display order |
| `created_at` | TEXT | NOT NULL | ISO8601 timestamp |

**Indexes:**
- `idx_gsi_item` ON `(checklist_item_id)`

**Relationships:**
- `golden_set_id` → `check_results.id` (the golden set container)
- `checklist_item_id` → `checklist_items.id` (the prompt item)

---

### 4. ai_execution_logs

Complete audit trail of LLM executions.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | INTEGER | PK, AUTOINCREMENT | Primary key |
| `check_result_id` | INTEGER | FK → `check_results.id` | Optional link to result |
| `context_type` | TEXT | NOT NULL | `sop` or `step` |
| `context_id` | INTEGER | NOT NULL | Evaluated entity ID |
| `check_item_id` | INTEGER | NOT NULL, FK → `checklist_items.id` | Checklist item used |
| `model_provider` | TEXT | NULLABLE | LLM provider (e.g., `openai`, `anthropic`) |
| `model_version` | TEXT | NOT NULL | Model name/version |
| `temperature` | REAL | DEFAULT 0.0 | LLM temperature setting |
| `prompt_snapshot` | TEXT | NULLABLE | Exact prompt sent to LLM |
| `raw_output` | TEXT | NULLABLE | Raw response from LLM |
| `input_tokens` | INTEGER | NOT NULL, DEFAULT 0 | Token count (input) |
| `output_tokens` | INTEGER | NOT NULL, DEFAULT 0 | Token count (output) |
| `exec_status` | INTEGER | NOT NULL, CHECK | `0`=pending, `1`=success, `2`=api_error, `3`=parse_failed |
| `error_message` | TEXT | NULLABLE | Error details if failed |
| `latency_ms` | INTEGER | NOT NULL, DEFAULT 0 | Execution time (ms) |
| `created_at` | TEXT | NOT NULL | ISO8601 timestamp |

**Indexes:**
- `idx_context_log` ON `(context_type, context_id)`
- `idx_rule_analysis` ON `(check_item_id, created_at)`

---

### 5. sops

Standard Operating Procedures.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | INTEGER | PK, AUTOINCREMENT | Primary key |
| `sop_id` | TEXT | NOT NULL, UNIQUE | Unique identifier (e.g., `SOP-001`) |
| `name` | TEXT | NOT NULL | Display name |
| `ticket_id` | TEXT | NULLABLE | Optional ticket reference |
| `version` | INTEGER | NOT NULL, DEFAULT 1 | Optimistic locking |
| `detect` | TEXT | NULLABLE | JSON - conditions that trigger this SOP |
| `handle` | TEXT | NULLABLE | JSON - how to handle |
| `verification` | TEXT | NULLABLE | JSON - how to verify success |
| `rollback` | TEXT | NULLABLE | JSON - rollback procedures |
| `status` | TEXT | NOT NULL, CHECK | `active`, `inactive`, `draft` |
| `created_at` | TEXT | NOT NULL | ISO8601 timestamp |
| `updated_at` | TEXT | NOT NULL | ISO8601 timestamp |

**Indexes:**
- `idx_sops_status` ON `(status)`
- `idx_sops_ticket` ON `(ticket_id)`

---

### 6. sop_steps

Individual steps within an SOP.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | INTEGER | PK, AUTOINCREMENT | Primary key |
| `sop_id` | TEXT | NOT NULL, FK → `sops.sop_id` | Parent SOP |
| `name` | TEXT | NOT NULL | Step name |
| `version` | INTEGER | NOT NULL, DEFAULT 1 | Optimistic locking |
| `operation` | TEXT | NULLABLE | JSON - what action to perform |
| `verification` | TEXT | NULLABLE | JSON - how to verify |
| `impact_analysis` | TEXT | NULLABLE | JSON - risk assessment |
| `rollback` | TEXT | NULLABLE | JSON - undo procedure |
| `created_at` | TEXT | NOT NULL | ISO8601 timestamp |
| `updated_at` | TEXT | NOT NULL | ISO8601 timestamp |

**Indexes:**
- `idx_sop_steps_sop` ON `(sop_id)`

---

## Data Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Create SOPs    │────►│  Add Steps      │────►│  Run Evaluation │
│  & Checklist   │     │  to SOPs        │     │  (check_results)│
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                          │
                                                          ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Group into    │◄────│  View Results   │◄────│  Log LLM Calls  │
│  Golden Sets   │     │  & Audit        │     │  (ai_execution) │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

---

## API to Database Mapping

| API Endpoint | Database Table | Operations |
|--------------|----------------|------------|
| `GET/POST /checklist-items` | `checklist_items` | SELECT, INSERT |
| `PATCH/DELETE /checklist-items/{id}` | `checklist_items` | UPDATE (soft delete) |
| `GET/POST /golden-sets/items` | `golden_set_items` | SELECT, INSERT |
| `DELETE /golden-sets/items` | `golden_set_items` | DELETE |
| `GET/POST /check-results` | `check_results` | SELECT, INSERT, UPDATE |
| `GET/POST /ai-logs` | `ai_execution_logs` | SELECT, INSERT |
| `GET/POST /sops` | `sops` | SELECT, INSERT |
| `PATCH/DELETE /sops/{sop_id}` | `sops` | UPDATE, DELETE |
| `GET/POST /sops/{sop_id}/steps` | `sop_steps` | SELECT, INSERT |
| `PATCH/DELETE /sops/{sop_id}/steps/{step_id}` | `sop_steps` | UPDATE, DELETE |
