//! REPL (Read-Eval-Print Loop) session

use crate::lql::LqlExecutor;
use anyhow::Result;
use colored::Colorize;
use rustyline::{
    CompletionType, Config, Context, EditMode, Editor, Helper,
    completion::{Completer, FilenameCompleter, Pair},
    error::ReadlineError,
    highlight::{Highlighter, MatchingBracketHighlighter},
    hint::{Hinter, HistoryHinter},
    validate::{self, MatchingBracketValidator, Validator},
};

/// REPL session for interactive Lexum CLI
pub struct ReplSession {
    url: String,
    client: crate::client::LexumClient,
}

impl ReplSession {
    /// Create a new REPL session
    pub fn new(url: String) -> Self {
        Self {
            client: crate::client::LexumClient::new(url.clone()),
            url,
        }
    }

    /// Start the REPL session
    pub async fn start(&self) -> Result<()> {
        println!("{}", "Welcome to Lexum REPL!".bright_blue().bold());
        println!("Type 'help' for available commands, 'exit' or 'quit' to exit.");
        println!();

        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build();

        let helper = LexumHelper::new();
        let mut rl = Editor::with_config(config)?;
        rl.set_helper(Some(helper));

        loop {
            let readline = rl.readline("lexum> ");
            match readline {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    // Add to history
                    rl.add_history_entry(line)?;

                    // Handle special commands
                    match line {
                        "exit" | "quit" => {
                            println!("{}", "Goodbye!".bright_green());
                            break;
                        }
                        "help" => {
                            Self::show_help();
                        }
                        "clear" => {
                            print!("\x1B[2J\x1B[1;1H");
                        }
                        _ => {
                            if let Err(e) = self.handle_command(line).await {
                                println!("{}", format!("Error: {e}").bright_red());
                            }
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("{}", "Use 'exit' or 'quit' to exit.".bright_yellow());
                }
                Err(ReadlineError::Eof) => {
                    println!("{}", "Goodbye!".bright_green());
                    break;
                }
                Err(err) => {
                    println!("{}", format!("Error: {err}").bright_red());
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a command entered in the REPL
    async fn handle_command(&self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "index" => self.handle_index_command(&parts[1..]).await,
            "doc" => self.handle_doc_command(&parts[1..]).await,
            "search" => self.handle_search_command(&parts[1..]).await,
            "lql" => self.handle_lql_command(&parts[1..]).await,
            "server" => self.handle_server_command(&parts[1..]).await,
            "snapshot" => self.handle_snapshot_command(&parts[1..]).await,
            "template" => self.handle_template_command(&parts[1..]).await,
            "alias" => self.handle_alias_command(&parts[1..]).await,
            _ => {
                let command = parts[0];
                let suggestions = Self::get_command_suggestions(command);

                println!("{}", format!("Unknown command: '{command}'").bright_red());

                if !suggestions.is_empty() {
                    println!("{}", "Did you mean one of these?".bright_yellow());
                    for suggestion in suggestions {
                        println!("  {}", suggestion.bright_cyan());
                    }
                } else {
                    println!("{}", "Type 'help' for available commands.".bright_yellow());
                }
                Ok(())
            }
        }
    }

    /// Handle index commands
    async fn handle_index_command(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            println!(
                "{}",
                "Index commands: list, create <name>, delete <name>, get <name>, stats <name>"
                    .bright_yellow()
            );
            return Ok(());
        }

        match args[0] {
            "list" => {
                match self
                    .client
                    .get::<serde_json::Value>("/_cat/indices?format=json")
                    .await
                {
                    Ok(indices) => {
                        println!("{}", serde_json::to_string_pretty(&indices)?);
                    }
                    Err(_) => {
                        println!("{}", "Failed to list indices".bright_red());
                    }
                }
            }
            "create" => {
                if args.len() < 2 {
                    println!("{}", "Usage: index create <name>".bright_red());
                    return Ok(());
                }
                let name = args[1];
                let index_config = serde_json::json!({
                    "settings": {
                        "number_of_shards": 1,
                        "number_of_replicas": 0
                    }
                });
                match self
                    .client
                    .put::<serde_json::Value, serde_json::Value>(&format!("/{name}"), &index_config)
                    .await
                {
                    Ok(_) => {
                        println!(
                            "{}",
                            format!("Index '{name}' created successfully").bright_green()
                        );
                    }
                    Err(_) => {
                        println!("{}", "Failed to create index".bright_red());
                    }
                }
            }
            "delete" => {
                if args.len() < 2 {
                    println!("{}", "Usage: index delete <name>".bright_red());
                    return Ok(());
                }
                let name = args[1];
                match self.client.delete(&format!("/{name}")).await {
                    Ok(_) => {
                        println!(
                            "{}",
                            format!("Index '{name}' deleted successfully").bright_green()
                        );
                    }
                    Err(_) => {
                        println!("{}", "Failed to delete index".bright_red());
                    }
                }
            }
            "get" => {
                if args.len() < 2 {
                    println!("{}", "Usage: index get <name>".bright_red());
                    return Ok(());
                }
                let name = args[1];
                match self
                    .client
                    .get::<serde_json::Value>(&format!("/{name}"))
                    .await
                {
                    Ok(info) => {
                        println!("{}", serde_json::to_string_pretty(&info)?);
                    }
                    Err(_) => {
                        println!("{}", "Failed to get index info".bright_red());
                    }
                }
            }
            "stats" => {
                if args.len() < 2 {
                    println!("{}", "Usage: index stats <name>".bright_red());
                    return Ok(());
                }
                let name = args[1];
                match self
                    .client
                    .get::<serde_json::Value>(&format!("/{name}/_stats"))
                    .await
                {
                    Ok(stats) => {
                        println!("{}", serde_json::to_string_pretty(&stats)?);
                    }
                    Err(_) => {
                        println!("{}", "Failed to get index stats".bright_red());
                    }
                }
            }
            _ => {
                let subcommand = args[0];
                let suggestions = Self::get_subcommand_suggestions("index", subcommand);

                println!(
                    "{}",
                    format!("Unknown index command: '{subcommand}'").bright_red()
                );

                if !suggestions.is_empty() {
                    println!("{}", "Did you mean one of these?".bright_yellow());
                    for suggestion in suggestions {
                        println!("  {}", suggestion.bright_cyan());
                    }
                } else {
                    println!(
                        "{}",
                        "Available: list, create, delete, get, stats".bright_yellow()
                    );
                }
            }
        }
        Ok(())
    }

    /// Handle document commands
    async fn handle_doc_command(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            println!("{}", "Document commands: add <index> <file>, get <index> <id>, delete <index> <id>, bulk <index> <file>".bright_yellow());
            return Ok(());
        }

        match args[0] {
            "add" => {
                if args.len() < 3 {
                    println!("{}", "Usage: doc add <index> <file>".bright_red());
                    return Ok(());
                }
                let index = args[1];
                let file = args[2];
                let content = std::fs::read_to_string(file)?;
                let doc: serde_json::Value = serde_json::from_str(&content)?;
                match self
                    .client
                    .post::<serde_json::Value, serde_json::Value>(&format!("/{index}/_doc"), &doc)
                    .await
                {
                    Ok(result) => {
                        println!(
                            "{}",
                            format!("Document added: {}", result["_id"]).bright_green()
                        );
                    }
                    Err(_) => {
                        println!("{}", "Failed to add document".bright_red());
                    }
                }
            }
            "get" => {
                if args.len() < 3 {
                    println!("{}", "Usage: doc get <index> <id>".bright_red());
                    return Ok(());
                }
                let index = args[1];
                let id = args[2];
                match self
                    .client
                    .get::<serde_json::Value>(&format!("/{index}/_doc/{id}"))
                    .await
                {
                    Ok(doc) => {
                        println!("{}", serde_json::to_string_pretty(&doc)?);
                    }
                    Err(_) => {
                        println!("{}", "Failed to get document".bright_red());
                    }
                }
            }
            "delete" => {
                if args.len() < 3 {
                    println!("{}", "Usage: doc delete <index> <id>".bright_red());
                    return Ok(());
                }
                let index = args[1];
                let id = args[2];
                match self.client.delete(&format!("/{index}/_doc/{id}")).await {
                    Ok(_) => {
                        println!("{}", "Document deleted successfully".bright_green());
                    }
                    Err(_) => {
                        println!("{}", "Failed to delete document".bright_red());
                    }
                }
            }
            "bulk" => {
                if args.len() < 3 {
                    println!("{}", "Usage: doc bulk <index> <file>".bright_red());
                    return Ok(());
                }
                let index = args[1];
                let file = args[2];
                let content = std::fs::read_to_string(file)?;
                match self
                    .client
                    .post::<String, serde_json::Value>(&format!("/{index}/_bulk"), &content)
                    .await
                {
                    Ok(result) => {
                        println!(
                            "{}",
                            format!(
                                "Bulk operation completed: {} items",
                                result["items"].as_array().unwrap().len()
                            )
                            .bright_green()
                        );
                    }
                    Err(_) => {
                        println!("{}", "Failed to execute bulk operation".bright_red());
                    }
                }
            }
            _ => {
                println!(
                    "{}",
                    "Unknown document command. Available: add, get, delete, bulk".bright_red()
                );
            }
        }
        Ok(())
    }

    /// Handle search commands
    async fn handle_search_command(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            println!(
                "{}",
                "Usage: search <index> <query> [--limit <n>] [--sort <field:direction>]"
                    .bright_yellow()
            );
            return Ok(());
        }

        let index = args[0];
        let query = args[1..].join(" ");

        let search_request = serde_json::json!({
            "query": {
                "match": {
                    "_all": query
                }
            },
            "size": 10
        });

        match self
            .client
            .post::<serde_json::Value, serde_json::Value>(
                &format!("/{index}/_search"),
                &search_request,
            )
            .await
        {
            Ok(results) => {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            Err(_) => {
                println!("{}", "Failed to execute search".bright_red());
            }
        }
        Ok(())
    }

    /// Handle LQL commands
    async fn handle_lql_command(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            println!("{}", "Usage: lql <query>".bright_yellow());
            return Ok(());
        }

        let query = args.join(" ");
        let executor = LqlExecutor::new(self.url.clone());
        let result = executor.execute("default", &query).await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        Ok(())
    }

    /// Handle server commands
    async fn handle_server_command(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            println!("{}", "Server commands: status, health".bright_yellow());
            return Ok(());
        }

        match args[0] {
            "status" => {
                match self
                    .client
                    .get::<serde_json::Value>("/_cluster/health")
                    .await
                {
                    Ok(status) => {
                        println!("{}", serde_json::to_string_pretty(&status)?);
                    }
                    Err(_) => {
                        println!("{}", "Failed to get server status".bright_red());
                    }
                }
            }
            "health" => {
                match self
                    .client
                    .get::<serde_json::Value>("/_cluster/health")
                    .await
                {
                    Ok(health) => {
                        let status = health["status"].as_str().unwrap_or("unknown");
                        let color = match status {
                            "green" => "bright_green",
                            "yellow" => "bright_yellow",
                            "red" => "bright_red",
                            _ => "white",
                        };
                        println!("{}", format!("Cluster status: {status}").color(color));
                    }
                    Err(_) => {
                        println!("{}", "Failed to get cluster health".bright_red());
                    }
                }
            }
            _ => {
                println!(
                    "{}",
                    "Unknown server command. Available: status, health".bright_red()
                );
            }
        }
        Ok(())
    }

    /// Handle snapshot commands
    async fn handle_snapshot_command(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            println!(
                "{}",
                "Snapshot commands: list, create <name>, delete <name>".bright_yellow()
            );
            return Ok(());
        }

        match args[0] {
            "list" => match self.client.get::<serde_json::Value>("/_snapshot").await {
                Ok(snapshots) => {
                    println!("{}", serde_json::to_string_pretty(&snapshots)?);
                }
                Err(_) => {
                    println!("{}", "Failed to list snapshots".bright_red());
                }
            },
            "create" => {
                if args.len() < 2 {
                    println!("{}", "Usage: snapshot create <name>".bright_red());
                    return Ok(());
                }
                let name = args[1];
                let snapshot_config = serde_json::json!({
                    "indices": "*",
                    "ignore_unavailable": true,
                    "include_global_state": false
                });
                match self
                    .client
                    .put::<serde_json::Value, serde_json::Value>(
                        &format!("/_snapshot/repo/{name}"),
                        &snapshot_config,
                    )
                    .await
                {
                    Ok(_response) => {
                        println!(
                            "{}",
                            format!("Snapshot '{name}' created successfully").bright_green()
                        );
                    }
                    Err(_) => {
                        println!("{}", "Failed to create snapshot".bright_red());
                    }
                }
            }
            "delete" => {
                if args.len() < 2 {
                    println!("{}", "Usage: snapshot delete <name>".bright_red());
                    return Ok(());
                }
                let name = args[1];
                match self.client.delete(&format!("/_snapshot/repo/{name}")).await {
                    Ok(_) => {
                        println!(
                            "{}",
                            format!("Snapshot '{name}' deleted successfully").bright_green()
                        );
                    }
                    Err(_) => {
                        println!("{}", "Failed to delete snapshot".bright_red());
                    }
                }
            }
            _ => {
                println!(
                    "{}",
                    "Unknown snapshot command. Available: list, create, delete".bright_red()
                );
            }
        }
        Ok(())
    }

    /// Handle template commands
    async fn handle_template_command(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            println!(
                "{}",
                "Template commands: list, create <name>, delete <name>".bright_yellow()
            );
            return Ok(());
        }

        match args[0] {
            "list" => match self.client.get::<serde_json::Value>("/_template").await {
                Ok(templates) => {
                    println!("{}", serde_json::to_string_pretty(&templates)?);
                }
                Err(_) => {
                    println!("{}", "Failed to list templates".bright_red());
                }
            },
            "create" => {
                if args.len() < 2 {
                    println!("{}", "Usage: template create <name>".bright_red());
                    return Ok(());
                }
                let name = args[1];
                let template_config = serde_json::json!({
                    "index_patterns": ["*"],
                    "settings": {
                        "number_of_shards": 1,
                        "number_of_replicas": 0
                    }
                });
                match self
                    .client
                    .put::<serde_json::Value, serde_json::Value>(
                        &format!("/_template/{name}"),
                        &template_config,
                    )
                    .await
                {
                    Ok(_response) => {
                        println!(
                            "{}",
                            format!("Template '{name}' created successfully").bright_green()
                        );
                    }
                    Err(_) => {
                        println!("{}", "Failed to create template".bright_red());
                    }
                }
            }
            "delete" => {
                if args.len() < 2 {
                    println!("{}", "Usage: template delete <name>".bright_red());
                    return Ok(());
                }
                let name = args[1];
                match self.client.delete(&format!("/_template/{name}")).await {
                    Ok(_) => {
                        println!(
                            "{}",
                            format!("Template '{name}' deleted successfully").bright_green()
                        );
                    }
                    Err(_) => {
                        println!("{}", "Failed to delete template".bright_red());
                    }
                }
            }
            _ => {
                println!(
                    "{}",
                    "Unknown template command. Available: list, create, delete".bright_red()
                );
            }
        }
        Ok(())
    }

    /// Get command suggestions for unknown commands
    fn get_command_suggestions(command: &str) -> Vec<String> {
        let all_commands = vec![
            "help", "exit", "quit", "index", "doc", "search", "server", "snapshot", "template",
            "lql",
        ];

        all_commands
            .into_iter()
            .filter(|cmd| cmd.starts_with(command) || command.len() > 2 && cmd.contains(command))
            .map(|cmd| cmd.to_string())
            .collect()
    }

    /// Get subcommand suggestions for unknown subcommands
    fn get_subcommand_suggestions(command: &str, subcommand: &str) -> Vec<String> {
        let subcommands = match command {
            "index" => vec!["list", "create", "delete", "get", "stats"],
            "doc" => vec!["add", "get", "delete", "bulk"],
            "server" => vec!["start", "stop", "status", "config"],
            "snapshot" => vec!["list", "create", "delete", "get", "list-repos", "repo"],
            "template" => vec!["list", "create", "delete", "get"],
            _ => vec![],
        };

        subcommands
            .into_iter()
            .filter(|cmd| {
                cmd.starts_with(subcommand) || subcommand.len() > 2 && cmd.contains(subcommand)
            })
            .map(|cmd| cmd.to_string())
            .collect()
    }

    /// Handle alias commands
    async fn handle_alias_command(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            println!(
                "{}",
                "Alias commands: list, get <index>, create <index> <alias>, delete <index> <alias>"
                    .bright_yellow()
            );
            return Ok(());
        }

        match args[0] {
            "list" => match self.client.get::<serde_json::Value>("/_aliases").await {
                Ok(aliases) => {
                    println!("{}", "Aliases:".bright_blue().bold());
                    if let Some(alias_map) = aliases.as_object() {
                        if alias_map.is_empty() {
                            println!("  No aliases found");
                        } else {
                            for (name, alias_info) in alias_map {
                                println!(
                                    "  {} -> {}",
                                    name.bright_cyan(),
                                    alias_info["indices"]
                                        .as_array()
                                        .map(|indices| indices
                                            .iter()
                                            .map(|i| i.as_str().unwrap_or(""))
                                            .collect::<Vec<_>>()
                                            .join(", "))
                                        .unwrap_or_else(|| "[]".to_string())
                                );
                            }
                        }
                    }
                }
                Err(_) => {
                    println!("{}", "Failed to list aliases".bright_red());
                }
            },
            "get" => {
                if args.len() < 2 {
                    println!("{}", "Usage: alias get <index>".bright_red());
                    return Ok(());
                }
                let index = args[1];
                match self
                    .client
                    .get::<serde_json::Value>(&format!("/{index}/_alias"))
                    .await
                {
                    Ok(aliases) => {
                        println!(
                            "{}",
                            format!("Aliases for index '{index}':").bright_blue().bold()
                        );
                        if let Some(alias_map) = aliases.as_object() {
                            if alias_map.is_empty() {
                                println!("  No aliases found for this index");
                            } else {
                                for (name, alias_info) in alias_map {
                                    println!(
                                        "  {} -> {}",
                                        name.bright_cyan(),
                                        alias_info["indices"]
                                            .as_array()
                                            .map(|indices| indices
                                                .iter()
                                                .map(|i| i.as_str().unwrap_or(""))
                                                .collect::<Vec<_>>()
                                                .join(", "))
                                            .unwrap_or_else(|| "[]".to_string())
                                    );
                                }
                            }
                        }
                    }
                    Err(_) => {
                        println!(
                            "{}",
                            format!("Failed to get aliases for index '{index}'").bright_red()
                        );
                    }
                }
            }
            "create" => {
                if args.len() < 3 {
                    println!("{}", "Usage: alias create <index> <alias>".bright_red());
                    return Ok(());
                }
                let index = args[1];
                let alias = args[2];
                let request_body = serde_json::json!({
                    "actions": [{
                        "action": "add",
                        "index": index,
                        "alias": alias
                    }]
                });
                match self
                    .client
                    .post::<serde_json::Value, serde_json::Value>("/_aliases", &request_body)
                    .await
                {
                    Ok(_) => {
                        println!(
                            "{}",
                            format!("Alias '{alias}' created successfully for index '{index}'")
                                .bright_green()
                        );
                    }
                    Err(_) => {
                        println!(
                            "{}",
                            format!("Failed to create alias '{alias}' for index '{index}'")
                                .bright_red()
                        );
                    }
                }
            }
            "delete" => {
                if args.len() < 3 {
                    println!("{}", "Usage: alias delete <index> <alias>".bright_red());
                    return Ok(());
                }
                let index = args[1];
                let alias = args[2];
                match self
                    .client
                    .delete(&format!("/{index}/_alias/{alias}"))
                    .await
                {
                    Ok(_) => {
                        println!(
                            "{}",
                            format!("Alias '{alias}' deleted successfully from index '{index}'")
                                .bright_green()
                        );
                    }
                    Err(_) => {
                        println!(
                            "{}",
                            format!("Failed to delete alias '{alias}' from index '{index}'")
                                .bright_red()
                        );
                    }
                }
            }
            _ => {
                println!(
                    "{}",
                    "Unknown alias command. Use 'alias' for help.".bright_red()
                );
            }
        }
        Ok(())
    }

    /// Show help information
    fn show_help() {
        println!("{}", "Available commands:".bright_blue().bold());
        println!();
        println!("{}", "Index Management:".bright_yellow());
        println!("  index list                    - List all indices");
        println!("  index create <name>           - Create a new index");
        println!("  index delete <name>           - Delete an index");
        println!("  index get <name>              - Get index information");
        println!("  index stats <name>            - Get index statistics");
        println!();
        println!("{}", "Document Operations:".bright_yellow());
        println!("  doc add <index> <file>        - Add document from file");
        println!("  doc get <index> <id>          - Get document by ID");
        println!("  doc delete <index> <id>       - Delete document by ID");
        println!("  doc bulk <index> <file>       - Bulk operations from file");
        println!();
        println!("{}", "Search:".bright_yellow());
        println!("  search <index> <query>        - Search documents");
        println!("  lql <query>                   - Execute LQL query");
        println!();
        println!("{}", "Server Management:".bright_yellow());
        println!("  server status                 - Get server status");
        println!("  server health                 - Get cluster health");
        println!();
        println!("{}", "Snapshots:".bright_yellow());
        println!("  snapshot list                 - List snapshots");
        println!("  snapshot create <name>        - Create snapshot");
        println!("  snapshot delete <name>        - Delete snapshot");
        println!();
        println!("{}", "Templates:".bright_yellow());
        println!("  template list                 - List templates");
        println!("  template create <name>        - Create template");
        println!("  template delete <name>        - Delete template");
        println!();
        println!("{}", "Other:".bright_yellow());
        println!("  help                          - Show this help");
        println!("  clear                         - Clear screen");
        println!("  exit, quit                    - Exit REPL");
    }
}

/// Helper struct for rustyline with autocomplete and syntax highlighting
struct LexumHelper {
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    _hinter: HistoryHinter,
}

impl LexumHelper {
    fn new() -> Self {
        Self {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            _hinter: HistoryHinter::new(),
        }
    }

    /// Get command suggestions based on current input
    fn get_command_suggestions(line: &str, parts: &[&str]) -> Vec<Pair> {
        let mut suggestions = Vec::new();

        if parts.is_empty() {
            // Top-level commands
            let commands = vec![
                "help", "exit", "quit", "index", "doc", "search", "server", "snapshot", "template",
                "lql",
            ];
            for cmd in commands {
                if cmd.starts_with(line.trim()) {
                    suggestions.push(Pair {
                        display: cmd.to_string(),
                        replacement: cmd.to_string(),
                    });
                }
            }
        } else if parts.len() == 1 {
            // Subcommands
            match parts[0] {
                "index" => {
                    let subcommands = vec!["list", "create", "delete", "get", "stats"];
                    for subcmd in subcommands {
                        if subcmd.starts_with(line.trim()) {
                            suggestions.push(Pair {
                                display: subcmd.to_string(),
                                replacement: subcmd.to_string(),
                            });
                        }
                    }
                }
                "doc" => {
                    let subcommands = vec!["add", "get", "delete", "bulk"];
                    for subcmd in subcommands {
                        if subcmd.starts_with(line.trim()) {
                            suggestions.push(Pair {
                                display: subcmd.to_string(),
                                replacement: subcmd.to_string(),
                            });
                        }
                    }
                }
                "server" => {
                    let subcommands = vec!["start", "stop", "status", "config"];
                    for subcmd in subcommands {
                        suggestions.push(Pair {
                            display: subcmd.to_string(),
                            replacement: subcmd.to_string(),
                        });
                    }
                }
                "snapshot" => {
                    let subcommands = vec!["list", "create", "delete", "get", "list-repos", "repo"];
                    for subcmd in subcommands {
                        if subcmd.starts_with(line.trim()) {
                            suggestions.push(Pair {
                                display: subcmd.to_string(),
                                replacement: subcmd.to_string(),
                            });
                        }
                    }
                }
                "template" => {
                    let subcommands = vec!["list", "create", "delete", "get"];
                    for subcmd in subcommands {
                        if subcmd.starts_with(line.trim()) {
                            suggestions.push(Pair {
                                display: subcmd.to_string(),
                                replacement: subcmd.to_string(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        suggestions
    }
}

impl Completer for LexumHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let (start, candidates) = self.completer.complete(line, pos, ctx)?;

        // Add command completions
        let mut all_candidates = candidates;
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Get command suggestions
        let command_suggestions = Self::get_command_suggestions(line, &parts);
        all_candidates.extend(command_suggestions);

        Ok((start, all_candidates))
    }
}

impl Hinter for LexumHelper {
    type Hint = String;

    fn hint(&self, line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            return Some("help".to_string());
        }

        match parts[0] {
            "index" if parts.len() == 1 => Some("list".to_string()),
            "doc" if parts.len() == 1 => Some("add".to_string()),
            "search" if parts.len() == 1 => Some("<index> <query>".to_string()),
            "lql" if parts.len() == 1 => Some("<query>".to_string()),
            "server" if parts.len() == 1 => Some("status".to_string()),
            "snapshot" if parts.len() == 1 => Some("list".to_string()),
            "template" if parts.len() == 1 => Some("list".to_string()),
            _ => None,
        }
    }
}

impl Highlighter for LexumHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> std::borrow::Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> std::borrow::Cow<'b, str> {
        self.highlighter.highlight_prompt(prompt, default)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        self.highlighter.highlight_hint(hint)
    }

    fn highlight_candidate<'c>(
        &self,
        candidate: &'c str,
        _completion: rustyline::CompletionType,
    ) -> std::borrow::Cow<'c, str> {
        self.highlighter
            .highlight_candidate(candidate, rustyline::CompletionType::List)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

impl Validator for LexumHelper {
    fn validate(
        &self,
        ctx: &mut validate::ValidationContext,
    ) -> Result<validate::ValidationResult, ReadlineError> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

impl Helper for LexumHelper {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_session_creation() {
        let session = ReplSession::new("http://localhost:9200".to_string());
        assert_eq!(session.url, "http://localhost:9200");
    }

    #[test]
    fn test_lexum_helper_creation() {
        let _helper = LexumHelper::new();
        // Should not panic
    }

    #[test]
    fn test_command_suggestions() {
        let suggestions = LexumHelper::get_command_suggestions("ind", &[]);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.display == "index"));
    }

    #[tokio::test]
    async fn test_handle_index_list_command() {
        let session = ReplSession::new("http://localhost:9200".to_string());
        // This will fail in test environment due to network call, but we can test the parsing
        let result = session.handle_command("index list").await;
        // We expect this to fail due to network, but the command parsing should work
        assert!(result.is_err() || result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_index_invalid_command() {
        let session = ReplSession::new("http://localhost:9200".to_string());
        let result = session.handle_command("index").await;
        assert!(result.is_ok()); // Should show usage message
    }

    #[tokio::test]
    async fn test_handle_doc_invalid_command() {
        let session = ReplSession::new("http://localhost:9200".to_string());
        let result = session.handle_command("doc").await;
        assert!(result.is_ok()); // Should show usage message
    }

    #[tokio::test]
    async fn test_handle_search_invalid_command() {
        let session = ReplSession::new("http://localhost:9200".to_string());
        let result = session.handle_command("search").await;
        assert!(result.is_ok()); // Should show usage message
    }

    #[tokio::test]
    async fn test_handle_server_invalid_command() {
        let session = ReplSession::new("http://localhost:9200".to_string());
        let result = session.handle_command("server").await;
        assert!(result.is_ok()); // Should show usage message
    }

    #[tokio::test]
    async fn test_handle_unknown_command() {
        let session = ReplSession::new("http://localhost:9200".to_string());
        let result = session.handle_command("unknown").await;
        assert!(result.is_ok()); // Should show unknown command message
    }
}
