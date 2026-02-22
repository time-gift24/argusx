use argusx_common::config::Settings;
use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{presets::UTF8_FULL, Cell, Table};
use prompt_lab_core::{
    AiExecutionLogFilter, AppendAiExecutionLogInput, BindGoldenSetItemInput, ChecklistFilter,
    ChecklistStatus, CreateChecklistItemInput, ExecStatus, PromptLab, SourceType, TargetLevel,
    UpdateChecklistItemInput, UpsertCheckResultInput,
};
use serde_json::Value;
use std::path::PathBuf;

const DEFAULT_DB_PATH: &str = "/Users/wanyaozhong/projects/argusx/argusx/prompt_lab/dev.db";

#[derive(Parser, Debug)]
#[command(name = "prompt-lab")]
#[command(about = "Prompt Lab management CLI", long_about = None)]
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
    GoldenSet {
        #[command(subcommand)]
        command: GoldenSetCommands,
    },
    Check {
        #[command(subcommand)]
        command: CheckCommands,
    },
    Log {
        #[command(subcommand)]
        command: LogCommands,
    },
}

#[derive(Subcommand, Debug)]
enum DbCommands {
    Init,
}

#[derive(Subcommand, Debug)]
enum ChecklistCommands {
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        prompt: String,
        #[arg(long, value_enum, default_value_t = CliTargetLevel::Step)]
        target_level: CliTargetLevel,
        #[arg(long)]
        result_schema: Option<String>,
        #[arg(long)]
        version: Option<i64>,
        #[arg(long, value_enum, default_value_t = CliChecklistStatus::Active)]
        status: CliChecklistStatus,
        #[arg(long)]
        created_by: Option<i64>,
    },
    Update {
        #[arg(long)]
        id: i64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        prompt: Option<String>,
        #[arg(long, value_enum)]
        target_level: Option<CliTargetLevel>,
        #[arg(long)]
        result_schema: Option<String>,
        #[arg(long)]
        version: Option<i64>,
        #[arg(long, value_enum)]
        status: Option<CliChecklistStatus>,
        #[arg(long)]
        updated_by: Option<i64>,
    },
    List {
        #[arg(long, value_enum)]
        status: Option<CliChecklistStatus>,
        #[arg(long, value_enum)]
        target_level: Option<CliTargetLevel>,
    },
}

#[derive(Subcommand, Debug)]
enum GoldenSetCommands {
    Bind {
        #[arg(long)]
        golden_set_id: i64,
        #[arg(long)]
        checklist_item_id: i64,
        #[arg(long, default_value_t = 0)]
        sort_order: i64,
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
        context_id: i64,
        #[arg(long)]
        check_item_id: i64,
        #[arg(long, value_enum, default_value_t = CliSourceType::Ai)]
        source_type: CliSourceType,
        #[arg(long)]
        operator_id: Option<String>,
        #[arg(long)]
        result: Option<String>,
        #[arg(long, default_value_t = false)]
        is_pass: bool,
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
}

#[derive(Subcommand, Debug)]
enum LogCommands {
    List {
        #[arg(long)]
        context_type: Option<String>,
        #[arg(long)]
        context_id: Option<i64>,
        #[arg(long)]
        check_item_id: Option<i64>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliTargetLevel {
    Step,
    Sop,
}

impl From<CliTargetLevel> for TargetLevel {
    fn from(value: CliTargetLevel) -> Self {
        match value {
            CliTargetLevel::Step => TargetLevel::Step,
            CliTargetLevel::Sop => TargetLevel::Sop,
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
            DbCommands::Init => {
                let status = lab.pragma_status().await?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&status)?);
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
            ChecklistCommands::Create {
                name,
                prompt,
                target_level,
                result_schema,
                version,
                status,
                created_by,
            } => {
                let item = lab
                    .checklist_service()
                    .create(CreateChecklistItemInput {
                        name,
                        prompt,
                        target_level: target_level.into(),
                        result_schema: parse_optional_json(
                            result_schema.as_deref(),
                            "result_schema",
                        )?,
                        version,
                        status: status.into(),
                        created_by,
                    })
                    .await?;
                print_checklist_items(cli.json, &[item])?;
            }
            ChecklistCommands::Update {
                id,
                name,
                prompt,
                target_level,
                result_schema,
                version,
                status,
                updated_by,
            } => {
                let item = lab
                    .checklist_service()
                    .update(UpdateChecklistItemInput {
                        id,
                        name,
                        prompt,
                        target_level: target_level.map(Into::into),
                        result_schema: parse_optional_json(
                            result_schema.as_deref(),
                            "result_schema",
                        )?,
                        version,
                        status: status.map(Into::into),
                        updated_by,
                    })
                    .await?;
                print_checklist_items(cli.json, &[item])?;
            }
            ChecklistCommands::List {
                status,
                target_level,
            } => {
                let items = lab
                    .checklist_service()
                    .list(ChecklistFilter {
                        status: status.map(Into::into),
                        target_level: target_level.map(Into::into),
                    })
                    .await?;
                print_checklist_items(cli.json, &items)?;
            }
        },
        Commands::GoldenSet { command } => match command {
            GoldenSetCommands::Bind {
                golden_set_id,
                checklist_item_id,
                sort_order,
            } => {
                let item = lab
                    .golden_set_service()
                    .bind(BindGoldenSetItemInput {
                        golden_set_id,
                        checklist_item_id,
                        sort_order,
                    })
                    .await?;

                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&item)?);
                } else {
                    let mut table = default_table();
                    table.set_header([
                        "golden_set_id",
                        "checklist_item_id",
                        "sort_order",
                        "created_at",
                    ]);
                    table.add_row([
                        Cell::new(item.golden_set_id),
                        Cell::new(item.checklist_item_id),
                        Cell::new(item.sort_order),
                        Cell::new(item.created_at),
                    ]);
                    println!("{table}");
                }
            }
        },
        Commands::Check { command } => match command {
            CheckCommands::Run {
                id,
                context_type,
                context_id,
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
                let check_result = lab
                    .check_result_service()
                    .upsert(UpsertCheckResultInput {
                        id,
                        context_type: context_type.clone(),
                        context_id,
                        check_item_id,
                        source_type: source_type.into(),
                        operator_id,
                        result: parse_optional_json(result.as_deref(), "result")?,
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
                    let model_version = log_model_version.unwrap_or_else(|| "unknown".to_string());
                    let log = lab
                        .ai_log_service()
                        .append(AppendAiExecutionLogInput {
                            check_result_id: Some(check_result.id),
                            context_type,
                            context_id,
                            check_item_id,
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
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "check_result": check_result,
                            "appended_log_id": appended_log_id,
                        }))?
                    );
                } else {
                    let mut table = default_table();
                    table.set_header([
                        "check_result_id",
                        "context_type",
                        "context_id",
                        "check_item_id",
                        "is_pass",
                        "appended_log_id",
                    ]);
                    table.add_row([
                        Cell::new(check_result.id),
                        Cell::new(check_result.context_type),
                        Cell::new(check_result.context_id),
                        Cell::new(check_result.check_item_id),
                        Cell::new(check_result.is_pass),
                        Cell::new(appended_log_id.map_or("-".to_string(), |v| v.to_string())),
                    ]);
                    println!("{table}");
                }
            }
        },
        Commands::Log { command } => match command {
            LogCommands::List {
                context_type,
                context_id,
                check_item_id,
            } => {
                let logs = lab
                    .ai_log_service()
                    .list(AiExecutionLogFilter {
                        context_type,
                        context_id,
                        check_item_id,
                    })
                    .await?;

                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&logs)?);
                } else {
                    let mut table = default_table();
                    table.set_header([
                        "id",
                        "check_result_id",
                        "context_type",
                        "context_id",
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
                            Cell::new(log.context_type),
                            Cell::new(log.context_id),
                            Cell::new(log.check_item_id),
                            Cell::new(log.model_version),
                            Cell::new(format!("{:?}", log.exec_status)),
                            Cell::new(log.created_at),
                        ]);
                    }
                    println!("{table}");
                }
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

fn print_checklist_items(
    json: bool,
    items: &[prompt_lab_core::ChecklistItem],
) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string_pretty(items)?);
        return Ok(());
    }

    let mut table = default_table();
    table.set_header([
        "id",
        "name",
        "target_level",
        "status",
        "version",
        "updated_at",
    ]);
    for item in items {
        table.add_row([
            Cell::new(item.id),
            Cell::new(&item.name),
            Cell::new(item.target_level),
            Cell::new(item.status),
            Cell::new(item.version),
            Cell::new(&item.updated_at),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn default_table() -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table
}

#[cfg(test)]
mod tests {
    use super::{ChecklistCommands, Cli, Commands};
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
}
