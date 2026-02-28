use argusx_common::config::Settings;
use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{presets::UTF8_FULL, Cell, Table};
use prompt_lab_core::{
    AiExecutionLog, AiExecutionLogFilter, AppendAiExecutionLogInput, CheckResult,
    CheckResultFilter, ChecklistContextType, ChecklistFilter, ChecklistItem, ChecklistStatus,
    ExecStatus, PromptLab, SopAggregate, SourceType, UpsertCheckResultInput,
};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

const DEFAULT_DB_PATH: &str = "./prompt_lab/dev.db";

#[derive(Parser, Debug)]
#[command(name = "prompt-lab")]
#[command(about = "Prompt Lab CLI (v2 prompt_lab_core)", long_about = None)]
struct Cli {
    #[arg(long, default_value = DEFAULT_DB_PATH)]
    db: PathBuf,

    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
    Checklist {
        #[command(subcommand)]
        command: ChecklistCommands,
    },
    Check {
        #[command(subcommand)]
        command: CheckCommands,
    },
    Log {
        #[command(subcommand)]
        command: LogCommands,
    },
    Sop {
        #[command(subcommand)]
        command: SopCommands,
    },
}

#[derive(Subcommand, Debug)]
enum DbCommands {
    Init,
    Status,
}

#[derive(Subcommand, Debug)]
enum ChecklistCommands {
    List {
        #[arg(long, value_enum)]
        status: Option<CliChecklistStatus>,
        #[arg(long, value_enum)]
        context_type: Option<CliChecklistContextType>,
    },
}

#[derive(Subcommand, Debug)]
enum CheckCommands {
    Run {
        #[arg(long)]
        id: Option<i64>,
        #[arg(long, default_value = "sop")]
        context_type: String,
        #[arg(long)]
        context_key: String,
        #[arg(long)]
        check_item_id: Option<i64>,
        #[arg(long, value_enum, default_value_t = CliSourceType::Ai)]
        source_type: CliSourceType,
        #[arg(long)]
        operator_id: Option<String>,
        #[arg(long)]
        result: Option<String>,
        #[arg(long)]
        is_pass: Option<bool>,
        #[arg(long, default_value_t = false)]
        append_log: bool,
        #[arg(long)]
        log_model_provider: Option<String>,
        #[arg(long)]
        log_model_version: Option<String>,
        #[arg(long)]
        log_temperature: Option<f64>,
        #[arg(long)]
        log_prompt_snapshot: Option<String>,
        #[arg(long)]
        log_raw_output: Option<String>,
        #[arg(long)]
        log_input_tokens: Option<i64>,
        #[arg(long)]
        log_output_tokens: Option<i64>,
        #[arg(long, value_enum, default_value_t = CliExecStatus::Success)]
        log_exec_status: CliExecStatus,
        #[arg(long)]
        log_error_message: Option<String>,
        #[arg(long)]
        log_latency_ms: Option<i64>,
    },
    List {
        #[arg(long)]
        context_type: Option<String>,
        #[arg(long)]
        context_key: Option<String>,
        #[arg(long)]
        check_item_id: Option<i64>,
    },
}

#[derive(Subcommand, Debug)]
enum LogCommands {
    List {
        #[arg(long)]
        context_type: Option<String>,
        #[arg(long)]
        context_key: Option<String>,
        #[arg(long)]
        check_item_id: Option<i64>,
    },
}

#[derive(Subcommand, Debug)]
enum SopCommands {
    Get {
        #[arg(long)]
        sop_id: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliChecklistContextType {
    Sop,
    SopProcedureDetect,
    SopProcedureHandle,
    SopProcedureVerification,
    SopProcedureRollback,
    SopStepOperation,
    SopStepVerification,
    SopStepImpactAnalysis,
    SopStepRollback,
    SopStepCommon,
}

impl From<CliChecklistContextType> for ChecklistContextType {
    fn from(value: CliChecklistContextType) -> Self {
        match value {
            CliChecklistContextType::Sop => ChecklistContextType::Sop,
            CliChecklistContextType::SopProcedureDetect => ChecklistContextType::SopProcedureDetect,
            CliChecklistContextType::SopProcedureHandle => ChecklistContextType::SopProcedureHandle,
            CliChecklistContextType::SopProcedureVerification => {
                ChecklistContextType::SopProcedureVerification
            }
            CliChecklistContextType::SopProcedureRollback => {
                ChecklistContextType::SopProcedureRollback
            }
            CliChecklistContextType::SopStepOperation => ChecklistContextType::SopStepOperation,
            CliChecklistContextType::SopStepVerification => {
                ChecklistContextType::SopStepVerification
            }
            CliChecklistContextType::SopStepImpactAnalysis => {
                ChecklistContextType::SopStepImpactAnalysis
            }
            CliChecklistContextType::SopStepRollback => ChecklistContextType::SopStepRollback,
            CliChecklistContextType::SopStepCommon => ChecklistContextType::SopStepCommon,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliChecklistStatus {
    Active,
    Inactive,
    Draft,
}

impl From<CliChecklistStatus> for ChecklistStatus {
    fn from(value: CliChecklistStatus) -> Self {
        match value {
            CliChecklistStatus::Active => ChecklistStatus::Active,
            CliChecklistStatus::Inactive => ChecklistStatus::Inactive,
            CliChecklistStatus::Draft => ChecklistStatus::Draft,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliSourceType {
    Ai,
    Manual,
}

impl From<CliSourceType> for SourceType {
    fn from(value: CliSourceType) -> Self {
        match value {
            CliSourceType::Ai => SourceType::Ai,
            CliSourceType::Manual => SourceType::Manual,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliExecStatus {
    Pending,
    Success,
    ApiError,
    ParseFailed,
}

impl From<CliExecStatus> for ExecStatus {
    fn from(value: CliExecStatus) -> Self {
        match value {
            CliExecStatus::Pending => ExecStatus::Pending,
            CliExecStatus::Success => ExecStatus::Success,
            CliExecStatus::ApiError => ExecStatus::ApiError,
            CliExecStatus::ParseFailed => ExecStatus::ParseFailed,
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let settings = Settings {
        database: argusx_common::config::DatabaseConfig {
            path: cli.db.to_string_lossy().to_string(),
            busy_timeout_ms: 5_000,
            max_connections: 5,
        },
        logging: argusx_common::config::LoggingConfig::default(),
    };
    let lab = PromptLab::new(settings).await?;

    match cli.command {
        Commands::Db { command } => match command {
            DbCommands::Init | DbCommands::Status => {
                let status = lab.pragma_status().await?;
                if cli.json {
                    print_json(&status)?;
                } else {
                    let mut table = default_table();
                    table.set_header(["foreign_keys", "journal_mode", "busy_timeout"]);
                    table.add_row([
                        Cell::new(status.foreign_keys),
                        Cell::new(status.journal_mode),
                        Cell::new(status.busy_timeout),
                    ]);
                    println!("{table}");
                }
            }
        },
        Commands::Checklist { command } => match command {
            ChecklistCommands::List {
                status,
                context_type,
            } => {
                let items = lab
                    .checklist_service()
                    .list(ChecklistFilter {
                        status: status.map(Into::into),
                        context_type: context_type.map(Into::into),
                        sop_step_id: None,
                    })
                    .await?;
                print_checklist_items(cli.json, &items)?;
            }
        },
        Commands::Check { command } => match command {
            CheckCommands::Run {
                id,
                context_type,
                context_key,
                check_item_id,
                source_type,
                operator_id,
                result,
                is_pass,
                append_log,
                log_model_provider,
                log_model_version,
                log_temperature,
                log_prompt_snapshot,
                log_raw_output,
                log_input_tokens,
                log_output_tokens,
                log_exec_status,
                log_error_message,
                log_latency_ms,
            } => {
                let result_json = parse_optional_json(result.as_deref(), "result")?;
                let context_type_for_log = context_type.clone();
                let context_key_for_log = context_key.clone();

                let check_result = lab
                    .check_result_service()
                    .upsert_or_append(UpsertCheckResultInput {
                        id,
                        context_type,
                        context_key,
                        check_item_id,
                        source_type: source_type.into(),
                        operator_id,
                        result: result_json,
                        is_pass,
                    })
                    .await?;

                let should_append_log = append_log
                    || log_model_provider.is_some()
                    || log_model_version.is_some()
                    || log_temperature.is_some()
                    || log_prompt_snapshot.is_some()
                    || log_raw_output.is_some()
                    || log_input_tokens.is_some()
                    || log_output_tokens.is_some()
                    || log_error_message.is_some()
                    || log_latency_ms.is_some();

                let mut appended_log_id: Option<i64> = None;

                if should_append_log {
                    let item_id = check_result.check_item_id.ok_or_else(|| {
                        "check_item_id is required when appending ai log".to_string()
                    })?;
                    let model_version = log_model_version.unwrap_or_else(|| "unknown".to_string());
                    let log = lab
                        .ai_log_service()
                        .append(AppendAiExecutionLogInput {
                            check_result_id: Some(check_result.id),
                            context_type: context_type_for_log,
                            context_key: context_key_for_log,
                            check_item_id: item_id,
                            model_provider: log_model_provider,
                            model_version,
                            temperature: log_temperature,
                            prompt_snapshot: log_prompt_snapshot,
                            raw_output: log_raw_output,
                            input_tokens: log_input_tokens,
                            output_tokens: log_output_tokens,
                            exec_status: log_exec_status.into(),
                            error_message: log_error_message,
                            latency_ms: log_latency_ms,
                        })
                        .await?;
                    appended_log_id = Some(log.id);
                }

                if cli.json {
                    print_json(&serde_json::json!({
                        "check_result": check_result,
                        "appended_log_id": appended_log_id,
                    }))?;
                } else {
                    let mut table = default_table();
                    table.set_header([
                        "check_result_id",
                        "context_type",
                        "context_key",
                        "check_item_id",
                        "source_type",
                        "is_pass",
                        "appended_log_id",
                    ]);
                    table.add_row([
                        Cell::new(check_result.id),
                        Cell::new(check_result.context_type),
                        Cell::new(check_result.context_key),
                        Cell::new(
                            check_result
                                .check_item_id
                                .map_or("-".to_string(), |v| v.to_string()),
                        ),
                        Cell::new(format!("{:?}", check_result.source_type)),
                        Cell::new(check_result.is_pass),
                        Cell::new(appended_log_id.map_or("-".to_string(), |v| v.to_string())),
                    ]);
                    println!("{table}");
                }
            }
            CheckCommands::List {
                context_type,
                context_key,
                check_item_id,
            } => {
                let rows = lab
                    .check_result_service()
                    .list(CheckResultFilter {
                        context_type,
                        context_key,
                        check_item_id,
                    })
                    .await?;
                print_check_results(cli.json, &rows)?;
            }
        },
        Commands::Log { command } => match command {
            LogCommands::List {
                context_type,
                context_key,
                check_item_id,
            } => {
                let logs = lab
                    .ai_log_service()
                    .list(AiExecutionLogFilter {
                        context_type,
                        context_key,
                        check_item_id,
                    })
                    .await?;
                print_ai_logs(cli.json, &logs)?;
            }
        },
        Commands::Sop { command } => match command {
            SopCommands::Get { sop_id } => {
                let agg = lab
                    .sop_service()
                    .get_sop_aggregate_by_sop_id(&sop_id)
                    .await?;
                print_sop_aggregate(cli.json, &agg)?;
            }
        },
    }

    Ok(())
}

fn parse_optional_json(
    input: Option<&str>,
    field: &str,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    match input {
        Some(raw) => {
            let value = serde_json::from_str::<Value>(raw)
                .map_err(|err| format!("failed to parse {field} as JSON: {err}"))?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_checklist_items(
    json: bool,
    items: &[ChecklistItem],
) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        return print_json(&items);
    }

    let mut table = default_table();
    table.set_header([
        "id",
        "name",
        "context_type",
        "status",
        "version",
        "updated_at",
    ]);
    for item in items {
        table.add_row([
            Cell::new(item.id),
            Cell::new(&item.name),
            Cell::new(item.context_type),
            Cell::new(item.status),
            Cell::new(item.version),
            Cell::new(&item.updated_at),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn print_check_results(json: bool, rows: &[CheckResult]) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        return print_json(&rows);
    }

    let mut table = default_table();
    table.set_header([
        "id",
        "context_type",
        "context_key",
        "check_item_id",
        "source_type",
        "is_pass",
        "created_at",
    ]);
    for row in rows {
        table.add_row([
            Cell::new(row.id),
            Cell::new(&row.context_type),
            Cell::new(&row.context_key),
            Cell::new(row.check_item_id.map_or("-".to_string(), |v| v.to_string())),
            Cell::new(format!("{:?}", row.source_type)),
            Cell::new(row.is_pass),
            Cell::new(row.created_at),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn print_ai_logs(json: bool, logs: &[AiExecutionLog]) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        return print_json(&logs);
    }

    let mut table = default_table();
    table.set_header([
        "id",
        "check_result_id",
        "context_type",
        "context_key",
        "check_item_id",
        "model_version",
        "exec_status",
        "created_at",
    ]);
    for log in logs {
        table.add_row([
            Cell::new(log.id),
            Cell::new(
                log.check_result_id
                    .map_or("-".to_string(), |v| v.to_string()),
            ),
            Cell::new(&log.context_type),
            Cell::new(&log.context_key),
            Cell::new(log.check_item_id),
            Cell::new(&log.model_version),
            Cell::new(format!("{:?}", log.exec_status)),
            Cell::new(log.created_at),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn print_sop_aggregate(json: bool, agg: &SopAggregate) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        return print_json(agg);
    }

    println!("sop_id: {}", agg.sop.sop_id);
    println!("name: {}", agg.sop.name);
    println!("status: {:?}", agg.sop.status);
    println!("detect_steps: {}", agg.detect_steps.len());
    println!("handle_steps: {}", agg.handle_steps.len());
    println!("verification_steps: {}", agg.verification_steps.len());
    println!("rollback_steps: {}", agg.rollback_steps.len());
    Ok(())
}

fn default_table() -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table
}

#[cfg(test)]
mod tests {
    use super::{CheckCommands, ChecklistCommands, Cli, Commands};
    use clap::Parser;

    #[test]
    fn parse_checklist_list_command() {
        let cli = Cli::try_parse_from(["prompt-lab", "checklist", "list"]).expect("parse");
        match cli.command {
            Commands::Checklist {
                command: ChecklistCommands::List { .. },
            } => {}
            _ => panic!("unexpected command"),
        }
    }

    #[test]
    fn parse_check_run_with_context_key() {
        let cli = Cli::try_parse_from([
            "prompt-lab",
            "check",
            "run",
            "--context-key",
            "sop:SOP-1",
            "--check-item-id",
            "7",
        ])
        .expect("parse");

        match cli.command {
            Commands::Check {
                command:
                    CheckCommands::Run {
                        context_key,
                        check_item_id,
                        ..
                    },
            } => {
                assert_eq!(context_key, "sop:SOP-1");
                assert_eq!(check_item_id, Some(7));
            }
            _ => panic!("unexpected command"),
        }
    }
}
