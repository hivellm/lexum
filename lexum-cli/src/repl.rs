//! REPL (Read-Eval-Print Loop) session

use anyhow::Result;
use colored::Colorize;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

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
                    println!("Usage: index <list|create|delete> [args]");
                    return Ok(());
                }

                match parts[1] {
                    "list" => crate::commands::index::list(&self.url).await?,
                    _ => println!("Unknown index command: {}", parts[1]),
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
        println!("  {} - List all indices", "index list".bright_yellow());
        println!(
            "  {} - Search documents",
            "search <index> <query>".bright_yellow()
        );
        println!("  {} - Exit the REPL", "exit".bright_yellow());
    }
}
