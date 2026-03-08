use serde::Serialize;
use serde_json::Value;
use turn::ToolOutcome;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPlanSnapshot {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub tasks: Vec<DesktopPlanTask>,
    pub is_streaming: bool,
    pub source_call_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DesktopPlanTask {
    pub id: String,
    pub title: String,
    pub status: DesktopPlanTaskStatus,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DesktopPlanTaskStatus {
    Pending,
    InProgress,
    Completed,
}

pub fn snapshot_from_tool_outcome(
    call_id: &str,
    outcome: &ToolOutcome,
) -> Option<DesktopPlanSnapshot> {
    match outcome {
        ToolOutcome::Success(output) => snapshot_from_output(call_id, output),
        ToolOutcome::Failed { .. }
        | ToolOutcome::TimedOut
        | ToolOutcome::Denied
        | ToolOutcome::Cancelled => None,
    }
}

pub fn snapshot_from_output(call_id: &str, output: &Value) -> Option<DesktopPlanSnapshot> {
    let plan = output.get("plan")?.as_object()?;
    let title = plan.get("title")?.as_str()?.trim();
    if title.is_empty() {
        return None;
    }

    let description = plan
        .get("description")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let tasks = plan
        .get("tasks")?
        .as_array()?
        .iter()
        .map(parse_task)
        .collect::<Option<Vec<_>>>()?;

    let is_streaming = plan
        .get("is_streaming")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            tasks
                .iter()
                .any(|task| task.status != DesktopPlanTaskStatus::Completed)
        });

    Some(DesktopPlanSnapshot {
        title: title.to_string(),
        description,
        tasks,
        is_streaming,
        source_call_id: call_id.to_string(),
    })
}

fn parse_task(value: &Value) -> Option<DesktopPlanTask> {
    let task = value.as_object()?;
    let id = task.get("id")?.as_str()?.trim();
    let title = task.get("title")?.as_str()?.trim();
    let status = parse_status(task.get("status")?.as_str()?.trim())?;

    if id.is_empty() || title.is_empty() {
        return None;
    }

    Some(DesktopPlanTask {
        id: id.to_string(),
        title: title.to_string(),
        status,
    })
}

fn parse_status(raw: &str) -> Option<DesktopPlanTaskStatus> {
    match raw {
        "pending" => Some(DesktopPlanTaskStatus::Pending),
        "in_progress" => Some(DesktopPlanTaskStatus::InProgress),
        "completed" => Some(DesktopPlanTaskStatus::Completed),
        _ => None,
    }
}
