mod mock;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "agent-session-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Create,
    List,
    Run,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    Ok(())
}
