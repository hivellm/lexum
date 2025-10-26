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

    /// Server management commands
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },

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
        /// Query text or file path (use @file for file input)
        query: String,
        /// Limit results
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Sort by field (format: field:asc or field:desc)
        #[arg(long)]
        sort: Option<Vec<String>>,
        /// Fields to return
        #[arg(long)]
        fields: Option<Vec<String>>,
        /// Highlight search terms in results
        #[arg(long)]
        highlight: bool,
        /// Explain query execution
        #[arg(long)]
        explain: bool,
        /// Minimum score threshold
        #[arg(long)]
        min_score: Option<f32>,
    },

    /// LQL (Lexum Query Language) commands
    Lql {
        /// Index name
        index: String,
        /// LQL query string or file path (use @file for file input)
        query: String,
        /// Limit results
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Sort by field (format: field:asc or field:desc)
        #[arg(long)]
        sort: Option<Vec<String>>,
        /// Fields to return
        #[arg(long)]
        fields: Option<Vec<String>>,
    },

    /// Snapshot management commands
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },

    /// Alias management commands
    Alias {
        #[command(subcommand)]
        action: AliasAction,
    },
}

#[derive(Subcommand)]
enum ServerAction {
    /// Start the Lexum server
    Start {
        /// Configuration file path
        #[arg(short, long, default_value = "config.yml")]
        config: String,
        /// Run as daemon
        #[arg(short, long)]
        daemon: bool,
    },
    /// Stop the Lexum server
    Stop,
    /// Get server status
    Status,
    /// Validate configuration file
    Config {
        /// Configuration file path
        #[arg(short, long, default_value = "config.yml")]
        file: String,
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
    /// Get index statistics
    Stats {
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
    /// Delete a document
    Delete {
        /// Index name
        index: String,
        /// Document ID
        id: String,
    },
    /// Bulk index documents from file
    Bulk {
        /// Index name
        index: String,
        /// JSON array file
        #[arg(short, long)]
        file: String,
    },
}

#[derive(Subcommand)]
enum SnapshotAction {
    /// List all snapshot repositories
    ListRepos,
    /// List snapshots in a repository
    List {
        /// Repository name
        repository: String,
    },
    /// Get snapshot information
    Get {
        /// Repository name
        repository: String,
        /// Snapshot name
        snapshot: String,
    },
    /// Create a snapshot
    Create {
        /// Repository name
        repository: String,
        /// Snapshot name
        snapshot: String,
        /// Indices to include (comma-separated)
        #[arg(short, long)]
        indices: Option<String>,
        /// Wait for completion
        #[arg(short, long)]
        wait: bool,
    },
    /// Delete a snapshot
    Delete {
        /// Repository name
        repository: String,
        /// Snapshot name
        snapshot: String,
    },
    /// Get repository information
    Repo {
        /// Repository name
        repository: String,
    },
}

#[derive(Subcommand)]
enum AliasAction {
    /// List all aliases
    List,
    /// Get aliases for a specific index
    Get {
        /// Index name
        index: String,
    },
    /// Create an alias
    Create {
        /// Index name
        index: String,
        /// Alias name
        alias: String,
        /// Configuration file (JSON)
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Delete an alias
    Delete {
        /// Index name
        index: String,
        /// Alias name
        alias: String,
    },
    /// Perform atomic alias operations
    Atomic {
        /// Operations file (JSON)
        #[arg(short, long)]
        file: String,
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

            let repl = ReplSession::new(cli.url);
            repl.start().await?;
        }
        Some(Commands::Server { action }) => match action {
            ServerAction::Start { config, daemon } => {
                commands::server::start(&config, daemon).await?;
            }
            ServerAction::Stop => {
                commands::server::stop(&cli.url).await?;
            }
            ServerAction::Status => {
                commands::server::status(&cli.url).await?;
            }
            ServerAction::Config { file } => {
                commands::server::validate_config(&file).await?;
            }
        },
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
            IndexAction::Stats { name } => {
                commands::index::stats(&cli.url, &name).await?;
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
            DocAction::Delete { index, id } => {
                commands::document::delete(&cli.url, &index, &id).await?;
            }
            DocAction::Bulk { index, file } => {
                commands::document::bulk(&cli.url, &index, &file).await?;
            }
        },
        Some(Commands::Search {
            index,
            query,
            limit,
            offset,
            sort,
            fields,
            highlight,
            explain,
            min_score,
        }) => {
            if let Some(file_path) = query.strip_prefix('@') {
                // Query from file
                commands::search::search_from_file_advanced(
                    &cli.url, &index, file_path, limit, offset, highlight, explain, min_score,
                )
                .await?;
            } else {
                // Parse sort options
                let sort_options = sort.map(|sort_fields| {
                    sort_fields
                        .into_iter()
                        .map(|s| {
                            if s.ends_with(":asc") {
                                let field = s[..s.len() - 4].to_string();
                                (field, commands::search::SortOrder::Asc)
                            } else if s.ends_with(":desc") {
                                let field = s[..s.len() - 5].to_string();
                                (field, commands::search::SortOrder::Desc)
                            } else {
                                (s, commands::search::SortOrder::Desc) // Default to desc
                            }
                        })
                        .collect()
                });

                // Direct query with advanced options
                commands::search::search_advanced_with_options(
                    &cli.url,
                    &index,
                    &query,
                    limit,
                    offset,
                    sort_options,
                    fields,
                    highlight,
                    explain,
                    min_score,
                )
                .await?;
            }
        }
        Some(Commands::Lql {
            index,
            query,
            limit,
            sort,
            fields,
        }) => {
            if let Some(file_path) = query.strip_prefix('@') {
                // LQL query from file
                commands::lql::lql_from_file(&cli.url, &index, file_path, limit).await?;
            } else {
                // Parse sort options
                let sort_options = sort.map(|sort_fields| {
                    sort_fields
                        .into_iter()
                        .map(|s| {
                            if s.ends_with(":asc") {
                                let field = s[..s.len() - 4].to_string();
                                (field, commands::search::SortOrder::Asc)
                            } else if s.ends_with(":desc") {
                                let field = s[..s.len() - 5].to_string();
                                (field, commands::search::SortOrder::Desc)
                            } else {
                                (s, commands::search::SortOrder::Desc) // Default to desc
                            }
                        })
                        .collect()
                });

                // Direct LQL query with advanced options
                commands::lql::lql_advanced(&cli.url, &index, &query, limit, sort_options, fields)
                    .await?;
            }
        }
        Some(Commands::Snapshot { action }) => match action {
            SnapshotAction::ListRepos => {
                commands::snapshot::list_repositories(&cli.url).await?;
            }
            SnapshotAction::List { repository } => {
                commands::snapshot::list_snapshots(&cli.url, &repository).await?;
            }
            SnapshotAction::Get {
                repository,
                snapshot,
            } => {
                commands::snapshot::get_snapshot(&cli.url, &repository, &snapshot).await?;
            }
            SnapshotAction::Create {
                repository,
                snapshot,
                indices,
                wait,
            } => {
                let indices_list = indices
                    .map(|i| i.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();
                commands::snapshot::create_snapshot(
                    &cli.url,
                    &repository,
                    &snapshot,
                    indices_list,
                    wait,
                )
                .await?;
            }
            SnapshotAction::Delete {
                repository,
                snapshot,
            } => {
                commands::snapshot::delete_snapshot(&cli.url, &repository, &snapshot).await?;
            }
            SnapshotAction::Repo { repository } => {
                commands::snapshot::get_repository(&cli.url, &repository).await?;
            }
        },
        Some(Commands::Alias { action }) => match action {
            AliasAction::List => {
                commands::alias::list_aliases(&cli.url).await?;
            }
            AliasAction::Get { index } => {
                commands::alias::get_index_aliases(&cli.url, &index).await?;
            }
            AliasAction::Create {
                index,
                alias,
                config,
            } => {
                let config_value = if let Some(config_file) = config {
                    let content = std::fs::read_to_string(&config_file)?;
                    Some(serde_json::from_str(&content)?)
                } else {
                    None
                };
                commands::alias::create_alias(&cli.url, &index, &alias, config_value).await?;
            }
            AliasAction::Delete { index, alias } => {
                commands::alias::delete_alias(&cli.url, &index, &alias).await?;
            }
            AliasAction::Atomic { file } => {
                let content = std::fs::read_to_string(&file)?;
                let operations: serde_json::Value = serde_json::from_str(&content)?;
                commands::alias::atomic_operations(&cli.url, operations).await?;
            }
        },
        None => {
            // No command provided, start REPL by default
            println!("{}", "Lexum Interactive Shell".bright_cyan().bold());
            println!("Server: {}\n", cli.url.bright_yellow());

            let repl = ReplSession::new(cli.url);
            repl.start().await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing_default() {
        let cli = Cli::try_parse_from(["lexum"]).unwrap();
        assert_eq!(cli.url, "http://localhost:9200");
        assert_eq!(cli.format, "table");
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_parsing_with_url() {
        let cli = Cli::try_parse_from(["lexum", "--url", "http://example.com:8080"]).unwrap();
        assert_eq!(cli.url, "http://example.com:8080");
        assert_eq!(cli.format, "table");
    }

    #[test]
    fn test_cli_parsing_with_format() {
        let cli = Cli::try_parse_from(["lexum", "--format", "json"]).unwrap();
        assert_eq!(cli.url, "http://localhost:9200");
        assert_eq!(cli.format, "json");
    }

    #[test]
    fn test_repl_command() {
        let cli = Cli::try_parse_from(["lexum", "repl"]).unwrap();
        match cli.command {
            Some(Commands::Repl) => (),
            _ => panic!("Expected Repl command"),
        }
    }

    #[test]
    fn test_server_start_command() {
        let cli =
            Cli::try_parse_from(["lexum", "server", "start", "--config", "test.yml"]).unwrap();
        match cli.command {
            Some(Commands::Server { action }) => match action {
                ServerAction::Start { config, daemon } => {
                    assert_eq!(config, "test.yml");
                    assert!(!daemon);
                }
                _ => panic!("Expected Start action"),
            },
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_server_start_daemon() {
        let cli = Cli::try_parse_from(["lexum", "server", "start", "--daemon"]).unwrap();
        match cli.command {
            Some(Commands::Server { action }) => match action {
                ServerAction::Start { daemon, .. } => {
                    assert!(daemon);
                }
                _ => panic!("Expected Start action"),
            },
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_server_stop_command() {
        let cli = Cli::try_parse_from(["lexum", "server", "stop"]).unwrap();
        match cli.command {
            Some(Commands::Server { action }) => match action {
                ServerAction::Stop => (),
                _ => panic!("Expected Stop action"),
            },
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_server_status_command() {
        let cli = Cli::try_parse_from(["lexum", "server", "status"]).unwrap();
        match cli.command {
            Some(Commands::Server { action }) => match action {
                ServerAction::Status => (),
                _ => panic!("Expected Status action"),
            },
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_server_config_command() {
        let cli =
            Cli::try_parse_from(["lexum", "server", "config", "--file", "config.yml"]).unwrap();
        match cli.command {
            Some(Commands::Server { action }) => match action {
                ServerAction::Config { file } => {
                    assert_eq!(file, "config.yml");
                }
                _ => panic!("Expected Config action"),
            },
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_index_create_command() {
        let cli = Cli::try_parse_from([
            "lexum",
            "index",
            "create",
            "test_index",
            "--schema",
            "schema.yml",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Index { action }) => match action {
                IndexAction::Create { name, schema } => {
                    assert_eq!(name, "test_index");
                    assert_eq!(schema, "schema.yml");
                }
                _ => panic!("Expected Create action"),
            },
            _ => panic!("Expected Index command"),
        }
    }

    #[test]
    fn test_index_list_command() {
        let cli = Cli::try_parse_from(["lexum", "index", "list"]).unwrap();
        match cli.command {
            Some(Commands::Index { action }) => match action {
                IndexAction::List => (),
                _ => panic!("Expected List action"),
            },
            _ => panic!("Expected Index command"),
        }
    }

    #[test]
    fn test_index_get_command() {
        let cli = Cli::try_parse_from(["lexum", "index", "get", "test_index"]).unwrap();
        match cli.command {
            Some(Commands::Index { action }) => match action {
                IndexAction::Get { name } => {
                    assert_eq!(name, "test_index");
                }
                _ => panic!("Expected Get action"),
            },
            _ => panic!("Expected Index command"),
        }
    }

    #[test]
    fn test_index_stats_command() {
        let cli = Cli::try_parse_from(["lexum", "index", "stats", "test_index"]).unwrap();
        match cli.command {
            Some(Commands::Index { action }) => match action {
                IndexAction::Stats { name } => {
                    assert_eq!(name, "test_index");
                }
                _ => panic!("Expected Stats action"),
            },
            _ => panic!("Expected Index command"),
        }
    }

    #[test]
    fn test_index_delete_command() {
        let cli = Cli::try_parse_from(["lexum", "index", "delete", "test_index"]).unwrap();
        match cli.command {
            Some(Commands::Index { action }) => match action {
                IndexAction::Delete { name } => {
                    assert_eq!(name, "test_index");
                }
                _ => panic!("Expected Delete action"),
            },
            _ => panic!("Expected Index command"),
        }
    }

    #[test]
    fn test_doc_add_command() {
        let cli = Cli::try_parse_from(["lexum", "doc", "add", "test_index", "--file", "doc.json"])
            .unwrap();
        match cli.command {
            Some(Commands::Doc { action }) => match action {
                DocAction::Add { index, file } => {
                    assert_eq!(index, "test_index");
                    assert_eq!(file, "doc.json");
                }
                _ => panic!("Expected Add action"),
            },
            _ => panic!("Expected Doc command"),
        }
    }

    #[test]
    fn test_doc_get_command() {
        let cli = Cli::try_parse_from(["lexum", "doc", "get", "test_index", "doc123"]).unwrap();
        match cli.command {
            Some(Commands::Doc { action }) => match action {
                DocAction::Get { index, id } => {
                    assert_eq!(index, "test_index");
                    assert_eq!(id, "doc123");
                }
                _ => panic!("Expected Get action"),
            },
            _ => panic!("Expected Doc command"),
        }
    }

    #[test]
    fn test_doc_delete_command() {
        let cli = Cli::try_parse_from(["lexum", "doc", "delete", "test_index", "doc123"]).unwrap();
        match cli.command {
            Some(Commands::Doc { action }) => match action {
                DocAction::Delete { index, id } => {
                    assert_eq!(index, "test_index");
                    assert_eq!(id, "doc123");
                }
                _ => panic!("Expected Delete action"),
            },
            _ => panic!("Expected Doc command"),
        }
    }

    #[test]
    fn test_doc_bulk_command() {
        let cli =
            Cli::try_parse_from(["lexum", "doc", "bulk", "test_index", "--file", "docs.json"])
                .unwrap();
        match cli.command {
            Some(Commands::Doc { action }) => match action {
                DocAction::Bulk { index, file } => {
                    assert_eq!(index, "test_index");
                    assert_eq!(file, "docs.json");
                }
                _ => panic!("Expected Bulk action"),
            },
            _ => panic!("Expected Doc command"),
        }
    }

    #[test]
    fn test_search_command() {
        let cli = Cli::try_parse_from([
            "lexum",
            "search",
            "test_index",
            "test query",
            "--limit",
            "5",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Search {
                index,
                query,
                limit,
                sort,
                fields,
                offset: _,
                highlight: _,
                explain: _,
                min_score: _,
            }) => {
                assert_eq!(index, "test_index");
                assert_eq!(query, "test query");
                assert_eq!(limit, 5);
                assert!(sort.is_none());
                assert!(fields.is_none());
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_command_with_sort() {
        let cli = Cli::try_parse_from([
            "lexum",
            "search",
            "test_index",
            "test query",
            "--sort",
            "field1:asc",
            "--sort",
            "field2:desc",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Search { sort, .. }) => {
                let sort_fields = sort.unwrap();
                assert_eq!(sort_fields.len(), 2);
                assert_eq!(sort_fields[0], "field1:asc");
                assert_eq!(sort_fields[1], "field2:desc");
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_command_with_fields() {
        let cli = Cli::try_parse_from([
            "lexum",
            "search",
            "test_index",
            "test query",
            "--fields",
            "field1",
            "--fields",
            "field2",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Search { fields, .. }) => {
                let field_list = fields.unwrap();
                assert_eq!(field_list.len(), 2);
                assert_eq!(field_list[0], "field1");
                assert_eq!(field_list[1], "field2");
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_command_file_input() {
        let cli = Cli::try_parse_from(["lexum", "search", "test_index", "@query.json"]).unwrap();
        match cli.command {
            Some(Commands::Search { query, .. }) => {
                assert_eq!(query, "@query.json");
            }
            _ => panic!("Expected Search command"),
        }
    }
}
