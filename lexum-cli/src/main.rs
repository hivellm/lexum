//! Lexum CLI - Command-line interface for Lexum search engine

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use lexum_cli::{commands, repl::ReplSession};

/// Lexum CLI - Search engine command-line interface
#[derive(Parser)]
#[command(name = "lexum")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Server URL
    #[arg(short, long, default_value = "http://localhost:9200")]
    url: String,

    /// Output format: json, json-pretty, table
    #[arg(short = 'f', long, default_value = "table")]
    format: String,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive REPL session
    Repl,

    /// Index management commands
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },

    /// Document operations
    Doc {
        #[command(subcommand)]
        action: DocAction,
    },

    /// Search documents
    Search {
        /// Index name
        index: String,
        /// Query text
        query: String,
        /// Limit results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum IndexAction {
    /// Create a new index
    Create {
        /// Index name
        name: String,
        /// Schema definition file (YAML)
        #[arg(short, long)]
        schema: String,
    },
    /// List all indices
    List,
    /// Get index info
    Get {
        /// Index name
        name: String,
    },
    /// Delete an index
    Delete {
        /// Index name
        name: String,
    },
}

#[derive(Subcommand)]
enum DocAction {
    /// Add a document
    Add {
        /// Index name
        index: String,
        /// Document JSON file
        #[arg(short, long)]
        file: String,
    },
    /// Get a document
    Get {
        /// Index name
        index: String,
        /// Document ID
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Repl) => {
            println!("{}", "Lexum Interactive Shell".bright_cyan().bold());
            println!("Server: {}\n", cli.url.bright_yellow());

            let mut repl = ReplSession::new(cli.url);
            repl.run().await?;
        }
        Some(Commands::Index { action }) => match action {
            IndexAction::Create { name, schema } => {
                commands::index::create(&cli.url, &name, &schema).await?;
            }
            IndexAction::List => {
                commands::index::list(&cli.url).await?;
            }
            IndexAction::Get { name } => {
                commands::index::get(&cli.url, &name).await?;
            }
            IndexAction::Delete { name } => {
                commands::index::delete(&cli.url, &name).await?;
            }
        },
        Some(Commands::Doc { action }) => match action {
            DocAction::Add { index, file } => {
                commands::document::add(&cli.url, &index, &file).await?;
            }
            DocAction::Get { index, id } => {
                commands::document::get(&cli.url, &index, &id).await?;
            }
        },
        Some(Commands::Search {
            index,
            query,
            limit,
        }) => {
            commands::search::search(&cli.url, &index, &query, limit).await?;
        }
        None => {
            // No command provided, start REPL by default
            println!("{}", "Lexum Interactive Shell".bright_cyan().bold());
            println!("Server: {}\n", cli.url.bright_yellow());

            let mut repl = ReplSession::new(cli.url);
            repl.run().await?;
        }
    }

    Ok(())
}
