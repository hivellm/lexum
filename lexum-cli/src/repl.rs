//! REPL (Read-Eval-Print Loop) session

use anyhow::Result;
use colored::Colorize;
use rustyline::{DefaultEditor, error::ReadlineError};

/// REPL session
pub struct ReplSession {
    url: String,
    editor: DefaultEditor,
}

impl ReplSession {
    /// Create new REPL session
    pub fn new(url: String) -> Self {
        let editor = DefaultEditor::new().expect("Failed to create editor");

        Self { url, editor }
    }

    /// Run REPL loop
    pub async fn run(&mut self) -> Result<()> {
        println!("Type 'help' for available commands, 'exit' to quit\n");

        loop {
            let prompt = format!("{} ", "lexum>".bright_green().bold());
            let readline = self.editor.readline(&prompt);

            match readline {
                Ok(line) => {
                    let line = line.trim();

                    if !line.is_empty() {
                        let _ = self.editor.add_history_entry(line);

                        if let Err(e) = self.handle_command(line).await {
                            eprintln!("{} {}", "Error:".bright_red().bold(), e);
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                }
                Err(ReadlineError::Eof) => {
                    println!("Bye!");
                    break;
                }
                Err(err) => {
                    eprintln!("{} {:?}", "Error:".bright_red().bold(), err);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_command(&self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "help" => Self::show_help(),
            "exit" | "quit" => {
                println!("Bye!");
                std::process::exit(0);
            }
            "index" => {
                if parts.len() < 2 {
                    println!("Usage: index <list|create|delete|get|stats> [args]");
                    return Ok(());
                }

                match parts[1] {
                    "list" => crate::commands::index::list(&self.url).await?,
                    "create" => {
                        if parts.len() < 4 {
                            println!("Usage: index create <name> <schema_file>");
                            return Ok(());
                        }
                        crate::commands::index::create(&self.url, parts[2], parts[3]).await?;
                    }
                    "delete" => {
                        if parts.len() < 3 {
                            println!("Usage: index delete <name>");
                            return Ok(());
                        }
                        crate::commands::index::delete(&self.url, parts[2]).await?;
                    }
                    "get" => {
                        if parts.len() < 3 {
                            println!("Usage: index get <name>");
                            return Ok(());
                        }
                        crate::commands::index::get(&self.url, parts[2]).await?;
                    }
                    "stats" => {
                        if parts.len() < 3 {
                            println!("Usage: index stats <name>");
                            return Ok(());
                        }
                        crate::commands::index::stats(&self.url, parts[2]).await?;
                    }
                    _ => println!("Unknown index command: {}", parts[1]),
                }
            }
            "doc" => {
                if parts.len() < 2 {
                    println!("Usage: doc <add|get|delete|bulk> [args]");
                    return Ok(());
                }

                match parts[1] {
                    "add" => {
                        if parts.len() < 4 {
                            println!("Usage: doc add <index> <file>");
                            return Ok(());
                        }
                        crate::commands::document::add(&self.url, parts[2], parts[3]).await?;
                    }
                    "get" => {
                        if parts.len() < 4 {
                            println!("Usage: doc get <index> <id>");
                            return Ok(());
                        }
                        crate::commands::document::get(&self.url, parts[2], parts[3]).await?;
                    }
                    "delete" => {
                        if parts.len() < 4 {
                            println!("Usage: doc delete <index> <id>");
                            return Ok(());
                        }
                        crate::commands::document::delete(&self.url, parts[2], parts[3]).await?;
                    }
                    "bulk" => {
                        if parts.len() < 4 {
                            println!("Usage: doc bulk <index> <file>");
                            return Ok(());
                        }
                        crate::commands::document::bulk(&self.url, parts[2], parts[3]).await?;
                    }
                    _ => println!("Unknown doc command: {}", parts[1]),
                }
            }
            "search" => {
                if parts.len() < 3 {
                    println!("Usage: search <index> <query>");
                    return Ok(());
                }

                let index = parts[1];
                let query = parts[2..].join(" ");
                crate::commands::search::search(&self.url, index, &query, 10).await?;
            }
            "server" => {
                if parts.len() < 2 {
                    println!("Usage: server <start|stop|status|config> [args]");
                    return Ok(());
                }

                match parts[1] {
                    "start" => {
                        let config = if parts.len() > 2 {
                            parts[2]
                        } else {
                            "config.yml"
                        };
                        let daemon = parts.contains(&"--daemon");
                        crate::commands::server::start(config, daemon).await?;
                    }
                    "stop" => crate::commands::server::stop(&self.url).await?,
                    "status" => crate::commands::server::status(&self.url).await?,
                    "config" => {
                        let file = if parts.len() > 2 {
                            parts[2]
                        } else {
                            "config.yml"
                        };
                        crate::commands::server::validate_config(file).await?;
                    }
                    _ => println!("Unknown server command: {}", parts[1]),
                }
            }
            _ => {
                println!("Unknown command: {}", parts[0]);
                println!("Type 'help' for available commands");
            }
        }

        Ok(())
    }

    fn show_help() {
        println!("{}", "Available Commands:".bright_cyan().bold());
        println!("  {} - Show this help message", "help".bright_yellow());
        println!("  {} - Exit the REPL", "exit".bright_yellow());
        println!();

        println!("{}", "Index Management:".bright_cyan().bold());
        println!("  {} - List all indices", "index list".bright_yellow());
        println!(
            "  {} - Create index with schema",
            "index create <name> <schema_file>".bright_yellow()
        );
        println!(
            "  {} - Get index information",
            "index get <name>".bright_yellow()
        );
        println!(
            "  {} - Get index statistics",
            "index stats <name>".bright_yellow()
        );
        println!(
            "  {} - Delete an index",
            "index delete <name>".bright_yellow()
        );
        println!();

        println!("{}", "Document Operations:".bright_cyan().bold());
        println!(
            "  {} - Add document from file",
            "doc add <index> <file>".bright_yellow()
        );
        println!(
            "  {} - Get document by ID",
            "doc get <index> <id>".bright_yellow()
        );
        println!(
            "  {} - Delete document by ID",
            "doc delete <index> <id>".bright_yellow()
        );
        println!(
            "  {} - Bulk index documents",
            "doc bulk <index> <file>".bright_yellow()
        );
        println!();

        println!("{}", "Search:".bright_cyan().bold());
        println!(
            "  {} - Search documents",
            "search <index> <query>".bright_yellow()
        );
        println!();

        println!("{}", "Server Management:".bright_cyan().bold());
        println!(
            "  {} - Start server",
            "server start [config_file] [--daemon]".bright_yellow()
        );
        println!("  {} - Stop server", "server stop".bright_yellow());
        println!(
            "  {} - Check server status",
            "server status".bright_yellow()
        );
        println!(
            "  {} - Validate config file",
            "server config [file]".bright_yellow()
        );
        println!();

        println!("{}", "Tips:".bright_cyan().bold());
        println!("  • Use Tab for command completion");
        println!("  • Use ↑/↓ arrows for command history");
        println!("  • Use Ctrl+C to interrupt current operation");
        println!("  • Use Ctrl+D or 'exit' to quit");
    }
}
