# Agent Thread Dispatch Design

## Overview

Argusx will support multiple agent capabilities with distinct system prompts and tool surfaces. A `thread` will represent one instantiated agent conversation, and subagents will run as separate child threads inside the same session.

This design keeps the existing `Session -> Thread -> Turn` boundary intact:

- `Session` remains the entry point and thread container.
- `Thread` remains the conversation aggregate and active-turn boundary.
- `Turn` remains the only single-run execution engine.

The new agent system is introduced by adding persistent agent profiles, thread-level frozen agent snapshots, and a blocking `dispatch_subagent` tool that creates and waits on child threads.

## Goals

- Support multiple agent profiles with different system prompts, tool policies, and model overrides.
- Treat each thread as an instantiated agent conversation.
- Allow the builtin main agent to plan, call `update_plan`, dispatch business subagents, and monitor them.
- Persist custom subagent profiles in SQLite.
- Freeze agent configuration per thread so history stays reproducible after profile edits.
- Keep subagent dispatch within the current `TurnDriver` contract without inventing a second turn state machine.

## Non-Goals

- Do not allow subagents to create nested subagents in v1.
- Do not add a new turn-level suspend/resume protocol for waiting on subagents.
- Do not move execution orchestration into `ThreadRuntime`.
- Do not replace the existing desktop chat compatibility API.

## Current Constraints

The current runtime already has the right top-level execution split:

- [`session/src/manager.rs`](/Users/wanyaozhong/Projects/argusx/session/src/manager.rs) owns thread orchestration and active-turn reservation.
- [`session/src/thread.rs`](/Users/wanyaozhong/Projects/argusx/session/src/thread.rs) only rebuilds replayable prior messages and tracks one active turn in memory.
- [`turn/src/driver.rs`](/Users/wanyaozhong/Projects/argusx/turn/src/driver.rs) owns the full model/tool/permission loop.
- [`desktop/src-tauri/src/chat/commands.rs`](/Users/wanyaozhong/Projects/argusx/desktop/src-tauri/src/chat/commands.rs) still accepts `targetId`, but that target does not currently affect execution capabilities.

The design must preserve these facts:

- `Thread` must not learn tool loop details.
- `Turn` must remain the only execution state machine.
- Incomplete turns still collapse to `Interrupted` on startup.

## Chosen Architecture

### Summary

Use a global SQLite-backed `AgentProfile` registry, bind each thread to one frozen agent snapshot at creation time, and let `SessionManager` resolve thread-specific execution dependencies before spawning each turn.

### Why this shape

- Agent profiles are capability configuration, not conversation history.
- Threads are execution instances and should hold frozen identity, not mutable global policy.
- Dispatching a subagent is a tool action inside a parent turn, not a new thread-managed protocol.

## Data Model

### `agent_profiles`

New global registry table for agent profiles.

Suggested fields:

- `id`
- `kind` (`builtin_main` or `custom_subagent`)
- `display_name`
- `description`
- `system_prompt`
- `tool_policy_json`
- `model_config_json`
- `allow_subagent_dispatch`
- `is_active`
- `created_at`
- `updated_at`

Builtin agents are seeded into the same table so the UI and runtime can resolve all agent targets through one registry.

### `threads`

Add instance-level agent identity fields:

- `agent_profile_id`
- `is_subagent`

This keeps the thread as an instantiated agent conversation without forcing mutable profile data directly into the thread row.

### `thread_agent_snapshots`

New one-to-one table that freezes profile data at thread creation time.

Suggested fields:

- `thread_id`
- `profile_id`
- `display_name_snapshot`
- `system_prompt_snapshot`
- `tool_policy_snapshot_json`
- `model_config_snapshot_json`
- `allow_subagent_dispatch_snapshot`
- `created_at`

This table is the runtime truth for future turns on that thread. Existing threads do not change when a profile is edited later.

### `subagent_dispatches`

New table for parent-child dispatch tracking.

Suggested fields:

- `id`
- `parent_thread_id`
- `parent_turn_id`
- `dispatch_tool_call_id`
- `child_thread_id`
- `child_agent_profile_id`
- `status` (`Running`, `Completed`, `Failed`, `Cancelled`, `Interrupted`)
- `requested_at`
- `finished_at`
- `result_summary`

This table is the stable monitoring surface for dispatch history and live subagent state.

## Runtime Boundaries

### New backend module

Introduce a dedicated backend module or crate for agent management instead of pushing agent logic into desktop-only chat files.

Responsibilities:

- `AgentProfileStore`
  SQLite CRUD, builtin seeding, validation.
- `ThreadAgentBinder`
  Creates thread rows plus frozen snapshot rows.
- `AgentExecutionResolver`
  Converts a thread snapshot into execution-time prompt, tool surface, authorizer policy, and model selection.
- `SubagentDispatcher`
  Implements the `dispatch_subagent` tool and waits on the child thread result.

### `runtime`

`runtime::build_runtime()` should assemble the agent services and inject them into the session/chat runtime. Desktop remains an adapter layer.

### `SessionManager`

`SessionManager` stays the entry point and coordinator. It gains agent-aware orchestration but does not own tool-loop logic.

New responsibilities:

- create a thread from a selected agent profile
- persist thread-level agent identity
- resolve thread snapshot execution config before spawning a turn

It should not:

- build agent prompts inline in ad hoc desktop code
- own dispatch wait loops directly
- reimplement step-by-step execution semantics

### `Thread`

`Thread` now means "conversation for one frozen agent instance".

It still only owns:

- ordered turn history
- single active-turn invariant
- replay to prior messages
- lifecycle

It does not own:

- dispatch protocols
- subagent wait state machines
- tool monitoring logic

### `Turn`

`TurnDriver` remains the only execution engine. Agent-specific behavior is injected through the seed and dependencies, not by changing the ownership boundary.

Recommended shape changes:

- extend `TurnSeed` with `system_prompt: Option<String>`
- extend `LlmStepRequest` with `system_prompt: Option<String>`
- optionally add execution metadata for agent/thread identity

The system prompt should not be persisted into transcript history as a replayed visible message.

## Agent Layers

### Builtin main agent

The builtin main agent is the default orchestrator.

Primary tools:

- `update_plan`
- `dispatch_subagent`
- `list_subagent_dispatches`
- `get_subagent_dispatch`
- optionally `read`, `glob`, `grep`

Its job is to plan, decide when to delegate, watch child execution, and synthesize final answers.

### Custom business agents

Custom SQLite-backed profiles execute domain work.

Default rules:

- no nested subagent dispatch
- tool surface comes from `tool_policy_json`
- prompt is frozen at thread creation time

## Tool Semantics

### `dispatch_subagent`

`dispatch_subagent` is a builtin orchestration tool.

Input shape:

```json
{
  "agent_profile_id": "reviewer",
  "task": "Review session manager changes for thread/turn boundary regressions",
  "context": {
    "parent_goal": "Add agent dispatch architecture safely",
    "acceptance_criteria": [
      "Find boundary regressions",
      "Call out state ownership mistakes"
    ]
  },
  "title": "Reviewer: session boundary audit"
}
```

Execution flow:

1. Validate the current parent thread is not itself a subagent.
2. Validate the current thread snapshot allows dispatch.
3. Create the child thread from the requested agent profile.
4. Freeze the child snapshot.
5. Insert `subagent_dispatches(status=Running)`.
6. Start the child turn through `SessionManager`.
7. Wait for the child turn to reach a terminal state.
8. Update `subagent_dispatches`.
9. Return structured summary JSON as the parent tool result.

Return shape:

```json
{
  "dispatch_id": "dispatch-uuid",
  "child_thread_id": "thread-uuid",
  "child_turn_id": "turn-uuid",
  "status": "completed",
  "summary": "Key findings from the child agent",
  "final_output": "Expanded child result",
  "latest_plan": null,
  "error": null
}
```

### Wait model

V1 does not introduce a turn-level suspend/resume protocol for subagent waiting.

Instead, `dispatch_subagent` blocks inside the tool implementation while the parent turn remains in a normal tool execution phase.

The wait strategy should be:

- event-driven first: subscribe to `SessionManager` events for the child turn
- store polling fallback: re-check persisted turn and dispatch status if events lag or are missed

This preserves the current `TurnDriver` contract and avoids inventing a new persistent `SuspendedWaitingSubagent` state.

### Monitoring tools

`list_subagent_dispatches`

- list dispatch records for a session or parent thread
- filter by `status`

`get_subagent_dispatch`

- return one dispatch with current child status
- expose current child turn state
- expose permission wait information if the child is blocked on approval
- expose latest child final output and latest plan snapshot if available

These tools exist for later turns and UI monitoring, not for the same blocked parent turn to poll itself.

## System Prompt Strategy

The prompt pipeline should borrow two ideas from [`.vendor/ironclaw/src/llm/reasoning.rs`](/Users/wanyaozhong/Projects/argusx/.vendor/ironclaw/src/llm/reasoning.rs):

- build one merged system prompt instead of scattering multiple system messages throughout the message list
- keep stable tool-usage and response-format instructions in a reusable platform layer

Recommended prompt composition order:

1. platform-level execution and tool usage rules
2. session default prompt
3. builtin role block for the selected agent kind
4. frozen thread agent prompt snapshot
5. current tool surface and policy block

### Builtin main agent prompt intent

The builtin main agent prompt should explicitly say:

- you are the orchestrator, not the default long-form executor of all business work
- create or update a plan before large delegated work
- delegate work that has clear boundaries and independent acceptance criteria
- provide explicit task, goal, and success criteria when dispatching a subagent
- summarize and judge child results rather than forwarding them blindly
- do not attempt nested subagent dispatch from a subagent context
- converge near the step limit

## Failure, Permission, and Recovery Semantics

### Failure layering

Separate:

- orchestration failure
  child thread creation failed, child turn start failed, waiter broke, store state corrupted
- child task failure
  child turn completed with `Failed`, `Cancelled`, or `Interrupted`

If orchestration itself fails, `dispatch_subagent` returns a tool failure.

If orchestration succeeds but the child task fails, `dispatch_subagent` still returns success as a structured JSON payload with failed child status so the main agent can decide what to do next.

### Permission behavior

Child permission flow stays local to the child turn.

The parent turn only sees `dispatch_subagent` still running.

`get_subagent_dispatch` must expose:

- `child_turn_status`
- `waiting_permission_request_id`
- `waiting_tool_call_id`

This allows later UI or later turns to resolve the child permission through the existing permission API.

### Startup recovery

On app restart:

- running parent turns waiting inside `dispatch_subagent` become `Interrupted`
- running or waiting child turns become `Interrupted`
- running dispatch records become `Interrupted`

The system restores history only. It does not recreate waiters or resume active parent-child execution.

### Cancellation propagation

Cancellation is best-effort:

- cancel parent turn
- if parent is blocked in `dispatch_subagent`, attempt to cancel child active turn
- persist final dispatch status even if child cancellation does not complete cleanly

## Desktop and Compatibility Implications

Existing chat commands remain:

- `start_turn`
- `cancel_turn`
- `resolve_turn_permission`
- `turn-event`

The desktop layer should continue to route these through `SessionManager`, but now it must resolve the requested target into an agent-backed thread instead of treating `targetId` as presentation-only metadata.

Users may manually start a thread with a business agent profile, not only via subagent dispatch.

## Invariants

New invariants added by this design:

1. every thread binds exactly one agent profile and one frozen snapshot
2. `is_subagent=true` threads must have exactly one parent dispatch record
3. builtin main agent may dispatch; subagents may not in v1
4. thread execution always reads frozen snapshot config, never mutable latest profile state
5. `dispatch_subagent` blocks inside the tool layer, not in a new turn persistence protocol

Existing invariants that must remain:

1. one active turn per thread
2. active-turn reservation happens before any await
3. turn insert plus `last_turn_number` advance stays atomic
4. each turn persists only its own transcript increment

## Testing Matrix

### Storage and migration

- seed builtin main agent profile
- create, read, update, deactivate custom agent profile
- create thread with agent binding and snapshot
- verify profile edits do not affect existing thread snapshots

### Session and dispatch orchestration

- create root thread from builtin main agent
- create user-started thread from custom business agent
- dispatch child thread and wait to successful completion
- dispatch child thread and observe child failure payload
- reject nested dispatch from a subagent thread

### Permission and monitoring

- child enters `WaitingPermission`
- `get_subagent_dispatch` exposes permission metadata
- permission resolution resumes child and completes parent tool call

### Recovery and cancellation

- parent waiting in dispatch becomes `Interrupted` after startup recovery
- child running turn becomes `Interrupted` after startup recovery
- running dispatch record becomes `Interrupted`
- cancelling parent attempts best-effort child cancellation

### Transcript integrity

- parent turn stores only parent incremental transcript including the final `dispatch_subagent` tool result
- child thread stores its own incremental transcript only
- prior message replay remains ordered and deduplicated

## Implementation Notes

The safest first slice is:

1. add agent profile and snapshot persistence
2. route thread creation through agent binding
3. inject frozen system prompt and tool policy into turn startup
4. add `dispatch_subagent` happy path
5. add monitoring tools
6. add interruption and cancellation hardening

This keeps the first milestone aligned with the existing session/thread/turn model and avoids prematurely building general-purpose workflow orchestration.
