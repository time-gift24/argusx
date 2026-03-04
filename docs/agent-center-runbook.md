# Agent-Center Operations Runbook

## Overview

Agent-Center provides multi-agent orchestration with robust lifecycle management, concurrency control, and crash recovery capabilities.

## Key Features

- **Lifecycle Management**: Thread state machine with transitions:
  - Normal: Pending → Running → Succeeded/Failed/Cancelled
  - Close: Running → Closing → Closed
  - Force Close: Running → Closed (bypasses Closing state)
  - All terminal states are idempotent
- **Concurrency Control**: RAII-based slot reservation with configurable max concurrent threads and depth limits
- **Idempotent Operations**: All operations (spawn, wait, close) are idempotent
  - Spawn with duplicate (parent, key) returns existing thread ID without consuming quota
  - Close on terminal thread returns success with actual status
- **Crash Recovery**: Reconciliation on startup to repair orphaned threads
- **Atomic Deduplication**: Race-condition-free spawn using transactions

## Tools

### spawn_agent
Spawns a child agent with idempotent deduplication.

**Parameters:**
- `parent_thread_id`: ID of parent thread (use "root" for top-level threads)
- `key`: Unique key for deduplication within parent scope
- `agent_name`: Type of agent to spawn
- `initial_input`: Initial input for the agent

**Behavior:**
- First call with (parent, key) creates new thread
- Subsequent calls with same (parent, key) return existing thread ID (idempotent)
- Idempotent retries succeed even at concurrency limit (quota not consumed for duplicates)
- Respects concurrency limits (configurable, default: 10)
- Respects depth limits (configurable, default: 3)

### wait
Waits for threads to reach terminal state.

**Parameters:**
- `thread_ids`: List of thread IDs to wait for
- `mode`: "any" (return when any thread terminal) or "all" (wait for all)
- `timeout_ms`: Timeout in milliseconds (clamped to [1000, 300000])

**Behavior:**
- Returns immediately if condition already satisfied
- Times out if condition not met within timeout
- Returns status map with current thread statuses
- Non-busy-loop polling (100ms intervals)

### close_agent
Gracefully closes a thread with idempotency.

**Parameters:**
- `thread_id`: ID of thread to close
- `force`: If true, skip Closing state and go directly to Closed (default: false)

**Behavior:**
- Normal close: Running → Closing → Closed
- Force close: Running → Closed (bypasses Closing state)
- Idempotent: Calling close on terminal thread returns success
- Releases concurrency slot on close
- Persists final state to database

## Crash Recovery

### Reconciliation Process

Run `reconcile()` on AgentCenter startup to recover from crashes:

```rust
let center = AgentCenter::builder()
    .db_path(db_path)
    .build()?;

let report = center.reconcile().await?;
println!("Repaired {} orphaned threads", report.repaired_count);
```

**What it does:**
1. Scans all threads in non-terminal states (Pending, Running, Closing)
2. Marks orphan threads as Failed (no active runtime)
3. Releases any held concurrency slots
4. Returns count of repaired threads

**Safety:**
- Idempotent: Can run multiple times safely
- No side effects on terminal threads
- ⚠️ **WARNING**: Only run during startup when no agent runtimes are active
- Running during normal operation will incorrectly mark active threads as Failed

### Common Crash Scenarios

| Scenario | State Before Crash | Reconciliation Action |
|----------|-------------------|---------------------|
| Process killed during spawn | Thread in Pending/Running | Mark as Failed, release slot |
| Process killed during close | Thread in Closing | Mark as Failed, release slot |
| Database corruption | N/A | Restore from backup, then reconcile |
| Disk full during persist | Transaction rollback | Thread not created, no repair needed |

## Monitoring

### Key Metrics

**Concurrency:**
- `agent_center_active_threads`: Current number of running threads
- `agent_center_max_concurrent_config`: Configured max concurrent limit
- `agent_center_reservation_failures_total`: Count of failed spawn attempts due to limits

**Lifecycle:**
- `agent_center_threads_created_total`: Total threads created
- `agent_center_threads_closed_total`: Total threads closed
- `agent_center_threads_by_status`: Gauge of threads by status

**Recovery:**
- `agent_center_reconcile_runs_total`: Times reconcile() called
- `agent_center_reconcile_repaired_total`: Threads repaired by reconciliation

### Health Checks

```rust
// Basic health: Can create AgentCenter
let center = AgentCenter::builder().build()?;

// Deep health: Reconcile returns successfully
let report = center.reconcile().await?;
assert!(report.repaired_count >= 0);
```

## Troubleshooting

### Spawn Failing with "Max Concurrent Exceeded"

**Symptoms:**
- spawn_agent returns error: "Maximum concurrent agents exceeded"

**Diagnosis:**
1. Check active thread count: `SELECT COUNT(*) FROM threads WHERE status IN ('Pending', 'Running', 'Closing')`
2. Compare against config: `max_concurrent` (default: 10)

**Solutions:**
- Close idle threads: `close_agent(thread_id)`
- Increase limit: `AgentCenter::builder().max_concurrent(20).build()`
- Run reconcile to clean orphaned threads

### Spawn Failing with "Max Depth Exceeded"

**Symptoms:**
- spawn_agent returns error: "Maximum depth exceeded"

**Diagnosis:**
1. Check parent depth in database
2. Compare against config: `max_depth` (default: 3)

**Solutions:**
- Restructure agent hierarchy to reduce nesting
- Increase limit: `AgentCenter::builder().max_depth(5).build()`

### Wait Timing Out

**Symptoms:**
- wait() returns with `timed_out: true`

**Diagnosis:**
1. Check thread status: `SELECT status FROM threads WHERE id = ?`
2. Verify agent is making progress (not stuck)

**Solutions:**
- Increase timeout: `timeout_ms: 60000` (60 seconds)
- Close stuck thread: `close_agent(thread_id, force=true)`
- Check agent implementation for bugs

### Database Locked Errors

**Symptoms:**
- Error: "database is locked"

**Diagnosis:**
1. Check for long-running transactions
2. Verify only one AgentCenter instance per database file

**Solutions:**
- Use separate database files per AgentCenter instance
- Ensure transactions are short-lived
- Consider using WAL mode: `PRAGMA journal_mode=WAL`

## Configuration

### Builder Options

```rust
let center = AgentCenter::builder()
    .max_concurrent(20)           // Max simultaneous threads
    .max_depth(5)                 // Max nesting depth
    .db_path(PathBuf::from("path/to/agent-center.db"))
    .build()?;
```

### Default Values

| Option | Default | Valid Range |
|--------|---------|-------------|
| `max_concurrent` | 10 | 1 - 10,000 |
| `max_depth` | 3 | 1 - 1,000 |
| `timeout_ms` (wait) | Clamped | [1,000, 300,000] |

## Database Schema

### threads table
```sql
CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    parent_thread_id TEXT,
    status TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    depth INTEGER NOT NULL DEFAULT 0,
    initial_input TEXT
);
```

**Columns:**
- `id`: Unique thread identifier
- `parent_thread_id`: Parent thread ID (NULL for root threads)
- `status`: Thread lifecycle status (Pending/Running/Succeeded/Failed/Cancelled/Closing/Closed)
- `agent_name`: Type of agent
- `created_at`: ISO 8601 timestamp
- `depth`: Nesting depth (0 for root threads)
- `initial_input`: Initial input provided at spawn time

### spawn_dedup table
```sql
CREATE TABLE spawn_dedup (
    parent_thread_id TEXT NOT NULL,
    key TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    PRIMARY KEY (parent_thread_id, key)
);
```

**Columns:**
- `parent_thread_id`: Parent thread ID
- `key`: Deduplication key within parent scope
- `thread_id`: The spawned thread ID

## Performance Considerations

- **Polling Interval**: wait() uses 100ms polling - adjust if latency critical
- **Database Connections**: One connection per AgentCenter, thread-safe via Mutex
- **Transaction Scope**: claim_spawn uses minimal transaction scope for concurrency
- **Memory**: SpawnReservation objects are small (~32 bytes), safe for thousands

## Security Considerations

- **Input Validation**: Agent names and thread IDs are validated
- **SQL Injection**: All queries use parameterized statements
- **Resource Limits**: Configurable limits prevent resource exhaustion
- **State Machine**: Enforced transitions prevent invalid state combinations
