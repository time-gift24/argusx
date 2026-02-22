mod mock;

use std::path::PathBuf;

use agent_session::SessionFilter;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "agent-session-cli")]
struct Cli {
    #[arg(long)]
    store_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Create {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        user_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    Run,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create {
            title,
            user_id,
            json,
        } => {
            let runtime = mock::build_runtime(cli.store_dir.clone());
            let session_id = runtime.create_session(user_id, title).await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({ "session_id": session_id }))?
                );
            } else {
                println!("session_id: {session_id}");
            }
        }
        Commands::List { json } => {
            let runtime = mock::build_runtime(cli.store_dir.clone());
            let sessions = runtime.list_sessions(SessionFilter::default()).await?;
            if json {
                println!("{}", serde_json::to_string(&sessions)?);
            } else {
                for session in sessions {
                    println!("{} {}", session.session_id, session.title);
                }
            }
        }
        Commands::Run => {
            anyhow::bail!("run command is not implemented yet");
        }
    }

    Ok(())
}
