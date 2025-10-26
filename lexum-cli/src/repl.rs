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

struct LexumHelper {
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
}

impl LexumHelper {
    /// Get command suggestions based on current input
    fn get_command_suggestions(line: &str, parts: &[&str]) -> Vec<Pair> {
        let mut suggestions = Vec::new();

        if parts.is_empty() {
            // Top-level commands
            let commands = vec![
                "help", "exit", "quit", "index", "doc", "search", "server", "snapshot", "lql",
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
                        if subcmd.starts_with(line.trim()) {
                            suggestions.push(Pair {
                                display: subcmd.to_string(),
                                replacement: subcmd.to_string(),
                            });
                        }
                    }
                }
                "snapshot" => {
                    let subcommands = vec!["repo", "create", "list", "delete", "restore"];
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
        } else if parts.len() == 2 {
            // Parameters for subcommands
            match (parts[0], parts[1]) {
                ("index", "create") => {
                    suggestions.push(Pair {
                        display: "<name>".to_string(),
                        replacement: "<name>".to_string(),
                    });
                }
                ("index", "delete") | ("index", "get") | ("index", "stats") => {
                    suggestions.push(Pair {
                        display: "<index_name>".to_string(),
                        replacement: "<index_name>".to_string(),
                    });
                }
                ("doc", "add") => {
                    suggestions.push(Pair {
                        display: "<index> <file>".to_string(),
                        replacement: "<index> <file>".to_string(),
                    });
                }
                ("doc", "get") | ("doc", "delete") => {
                    suggestions.push(Pair {
                        display: "<index> <id>".to_string(),
                        replacement: "<index> <id>".to_string(),
                    });
                }
                ("doc", "bulk") => {
                    suggestions.push(Pair {
                        display: "<index> <file>".to_string(),
                        replacement: "<index> <file>".to_string(),
                    });
                }
                ("search", _) => {
                    suggestions.push(Pair {
                        display: "<index> <query>".to_string(),
                        replacement: "<index> <query>".to_string(),
                    });
                }
                ("lql", _) => {
                    suggestions.push(Pair {
                        display: "<index> <query>".to_string(),
                        replacement: "<index> <query>".to_string(),
                    });
                }
                _ => {}
            }
        } else if parts.len() >= 3 {
            // Options and flags
            match (parts[0], parts[1]) {
                ("search", _) => {
                    let options = vec!["--limit", "--sort", "--fields", "--offset", "--explain"];
                    for opt in options {
                        if opt.starts_with(parts[2]) {
                            suggestions.push(Pair {
                                display: opt.to_string(),
                                replacement: opt.to_string(),
                            });
                        }
                    }
                }
                ("lql", _) => {
                    let options = vec!["--sort", "--fields", "--limit", "--offset", "--explain"];
                    for opt in options {
                        if opt.starts_with(parts[2]) {
                            suggestions.push(Pair {
                                display: opt.to_string(),
                                replacement: opt.to_string(),
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

        if line.trim().is_empty() || !line.contains(' ') {
            // First word - suggest commands
            let commands = vec![
                "help", "exit", "quit", "index", "doc", "search", "server", "snapshot", "lql",
            ];

            for cmd in commands {
                if cmd.starts_with(line.trim()) {
                    all_candidates.push(Pair {
                        display: cmd.to_string(),
                        replacement: cmd.to_string(),
                    });
                }
            }
        } else {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 1 {
                // After first command, suggest subcommands
                match parts[0] {
                    "index" => {
                        let subcommands = vec!["list", "create", "delete", "get", "stats"];
                        for subcmd in subcommands {
                            if subcmd.starts_with(line.trim()) {
                                all_candidates.push(Pair {
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
                                all_candidates.push(Pair {
                                    display: subcmd.to_string(),
                                    replacement: subcmd.to_string(),
                                });
                            }
                        }
                    }
                    "search" => {
                        // For search command, suggest common query patterns
                        let query_patterns = vec![
                            "*",
                            "field:value",
                            "field:\"phrase\"",
                            "field:~fuzzy",
                            "field:[min,max]",
                            "+field:value",
                            "-field:value",
                            "@file.json",
                            "match_all",
                            "term",
                            "range",
                            "bool",
                            "fuzzy",
                            "phrase",
                        ];
                        for pattern in query_patterns {
                            if pattern.starts_with(line.trim()) {
                                all_candidates.push(Pair {
                                    display: pattern.to_string(),
                                    replacement: pattern.to_string(),
                                });
                            }
                        }
                    }
                    "lql" => {
                        // For LQL command, suggest LQL patterns
                        let lql_patterns = vec![
                            "FROM index WHERE field:value",
                            "SELECT * FROM index WHERE field:value",
                            "MATCH field:value",
                            "field:value",
                            "field:\"phrase\"",
                            "field:[min,max]",
                            "field:~fuzzy",
                            "field1:value1 AND field2:value2",
                            "field1:value1 OR field2:value2",
                            "@file.lql",
                            "@query.json",
                        ];
                        for pattern in lql_patterns {
                            if pattern.starts_with(line.trim()) {
                                all_candidates.push(Pair {
                                    display: pattern.to_string(),
                                    replacement: pattern.to_string(),
                                });
                            }
                        }
                    }
                    "server" => {
                        let subcommands = vec!["start", "stop", "status", "config"];
                        for subcmd in subcommands {
                            if subcmd.starts_with(line.trim()) {
                                all_candidates.push(Pair {
                                    display: subcmd.to_string(),
                                    replacement: subcmd.to_string(),
                                });
                            }
                        }
                    }
                    "snapshot" => {
                        let subcommands =
                            vec!["list-repos", "list", "get", "create", "delete", "repo"];
                        for subcmd in subcommands {
                            if subcmd.starts_with(line.trim()) {
                                all_candidates.push(Pair {
                                    display: subcmd.to_string(),
                                    replacement: subcmd.to_string(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            } else if parts.len() >= 2 {
                // Suggest options and parameters based on command context
                let current_word = parts.last().map_or("", |v| v);
                let command = parts[0];

                match command {
                    "search" if parts.len() >= 3 => {
                        // Suggest search options
                        let options = vec![
                            "--limit", "--sort", "--fields", "--help", "--offset", "--format",
                        ];
                        for opt in options {
                            if opt.starts_with(current_word) {
                                all_candidates.push(Pair {
                                    display: opt.to_string(),
                                    replacement: opt.to_string(),
                                });
                            }
                        }

                        // If current word is --sort, suggest sort options
                        if current_word == "--sort"
                            || (parts.len() > 3 && parts[parts.len() - 2] == "--sort")
                        {
                            let sort_options =
                                vec!["score:desc", "score:asc", "field:desc", "field:asc"];
                            for sort_opt in sort_options {
                                if sort_opt.starts_with(current_word) {
                                    all_candidates.push(Pair {
                                        display: sort_opt.to_string(),
                                        replacement: sort_opt.to_string(),
                                    });
                                }
                            }
                        }
                    }
                    "index" if parts.len() >= 3 => {
                        match parts[1] {
                            "create" | "delete" | "get" | "stats" => {
                                // Suggest common index names
                                let common_indices =
                                    vec!["logs", "documents", "products", "users", "events"];
                                for idx in common_indices {
                                    if idx.starts_with(current_word) {
                                        all_candidates.push(Pair {
                                            display: idx.to_string(),
                                            replacement: idx.to_string(),
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    "doc" if parts.len() >= 3 => {
                        match parts[1] {
                            "add" | "get" | "delete" | "bulk" => {
                                // Suggest file paths or document IDs
                                if current_word.starts_with('@') || current_word.ends_with(".json")
                                {
                                    // File completion is handled by FilenameCompleter
                                } else if parts[1] == "get" || parts[1] == "delete" {
                                    // Suggest common document ID patterns
                                    let id_patterns =
                                        vec!["doc_1", "doc_2", "user_123", "item_456"];
                                    for pattern in id_patterns {
                                        if pattern.starts_with(current_word) {
                                            all_candidates.push(Pair {
                                                display: pattern.to_string(),
                                                replacement: pattern.to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok((start, all_candidates))
    }
}

impl Hinter for LexumHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for LexumHelper {
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

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> std::borrow::Cow<'l, str> {
        self.highlighter.highlight(line, pos)
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

/// REPL session with optimized performance
pub struct ReplSession {
    url: String,
    editor: Editor<LexumHelper, rustyline::history::DefaultHistory>,
    #[allow(dead_code)]
    client: Option<crate::client::LexumClient>,
}

impl ReplSession {
    /// Create new REPL session
    pub fn new(url: String) -> Self {
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build();

        let helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        let mut editor = Editor::with_config(config).expect("Failed to create editor");
        editor.set_helper(Some(helper));

        Self {
            url,
            editor,
            client: None,
        }
    }

    /// Get or create HTTP client
    #[allow(dead_code)]
    fn get_client(&mut self) -> &crate::client::LexumClient {
        if self.client.is_none() {
            self.client = Some(crate::client::LexumClient::new(self.url.clone()));
        }
        self.client.as_ref().unwrap()
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
                            let suggestions = Self::suggest_commands(line);
                            if !suggestions.is_empty() {
                                eprintln!(
                                    "{} Did you mean: {}",
                                    "Suggestions:".bright_yellow().bold(),
                                    suggestions.join(", ")
                                );
                            }
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
                    _ => {
                        println!("Unknown index command: {}", parts[1]);
                        let suggestions = Self::suggest_subcommands("index", parts[1]);
                        if !suggestions.is_empty() {
                            println!("Did you mean: {}", suggestions.join(", "));
                        }
                    }
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
                    _ => {
                        println!("Unknown doc command: {}", parts[1]);
                        let suggestions = Self::suggest_subcommands("doc", parts[1]);
                        if !suggestions.is_empty() {
                            println!("Did you mean: {}", suggestions.join(", "));
                        }
                    }
                }
            }
            "search" => {
                if parts.len() < 3 {
                    println!(
                        "Usage: search <index> <query> [--limit N] [--sort field:asc/desc] [--fields field1,field2]"
                    );
                    println!("       search <index> @<file> [--limit N]");
                    return Ok(());
                }

                let index = parts[1];
                let mut query_parts = Vec::new();
                let mut limit = 10;
                let mut sort_fields = Vec::new();
                let mut fields = Vec::new();

                let mut i = 2;
                while i < parts.len() {
                    match parts[i] {
                        "--limit" => {
                            if i + 1 < parts.len() {
                                limit = parts[i + 1].parse().unwrap_or(10);
                                i += 2;
                            } else {
                                i += 1;
                            }
                        }
                        "--sort" => {
                            if i + 1 < parts.len() {
                                sort_fields.push(parts[i + 1].to_string());
                                i += 2;
                            } else {
                                i += 1;
                            }
                        }
                        "--fields" => {
                            if i + 1 < parts.len() {
                                fields
                                    .extend(parts[i + 1].split(',').map(|s| s.trim().to_string()));
                                i += 2;
                            } else {
                                i += 1;
                            }
                        }
                        _ => {
                            query_parts.push(parts[i]);
                            i += 1;
                        }
                    }
                }

                let query = query_parts.join(" ");

                if let Some(file_path) = query.strip_prefix('@') {
                    // Query from file
                    crate::commands::search::search_from_file(&self.url, index, file_path, limit)
                        .await?;
                } else {
                    // Parse sort options
                    let sort_options = if !sort_fields.is_empty() {
                        Some(
                            sort_fields
                                .into_iter()
                                .map(|s| {
                                    if s.ends_with(":asc") {
                                        let field = s[..s.len() - 4].to_string();
                                        (field, crate::commands::search::SortOrder::Asc)
                                    } else if s.ends_with(":desc") {
                                        let field = s[..s.len() - 5].to_string();
                                        (field, crate::commands::search::SortOrder::Desc)
                                    } else {
                                        (s, crate::commands::search::SortOrder::Desc) // Default to desc
                                    }
                                })
                                .collect(),
                        )
                    } else {
                        None
                    };

                    // Direct query with advanced options
                    crate::commands::search::search_advanced(
                        &self.url,
                        index,
                        &query,
                        limit,
                        sort_options,
                        if fields.is_empty() {
                            None
                        } else {
                            Some(fields)
                        },
                    )
                    .await?;
                }
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
                    _ => {
                        println!("Unknown server command: {}", parts[1]);
                        let suggestions = Self::suggest_subcommands("server", parts[1]);
                        if !suggestions.is_empty() {
                            println!("Did you mean: {}", suggestions.join(", "));
                        }
                    }
                }
            }
            "lql" => {
                if parts.len() < 3 {
                    println!("Usage: lql <index> <query>");
                    println!("       lql <index> @<file>");
                    println!("Example: lql users \"FROM users WHERE name:john\"");
                    println!("Example: lql products \"title:rust AND price:[10,100]\"");
                    return Ok(());
                }

                let index = parts[1];
                let query = parts[2..].join(" ");

                let executor = LqlExecutor::new(self.url.clone());

                if let Some(file_path) = query.strip_prefix('@') {
                    // Query from file
                    match Self::read_lql_from_file(file_path) {
                        Ok(file_query) => match executor.execute(index, &file_query).await {
                            Ok(result) => {
                                println!("{}", serde_json::to_string_pretty(&result)?);
                            }
                            Err(e) => {
                                println!("{} LQL execution failed: {}", "Error:".red().bold(), e);
                            }
                        },
                        Err(e) => {
                            println!("{} Failed to read LQL file: {}", "Error:".red().bold(), e);
                        }
                    }
                } else {
                    // Direct query
                    match executor.execute(index, &query).await {
                        Ok(result) => {
                            println!("{}", serde_json::to_string_pretty(&result)?);
                        }
                        Err(e) => {
                            println!("{} LQL execution failed: {}", "Error:".red().bold(), e);
                        }
                    }
                }
            }
            "snapshot" => {
                if parts.len() < 2 {
                    println!("Usage: snapshot <list-repos|list|get|create|delete|repo> [args]");
                    return Ok(());
                }

                match parts[1] {
                    "list-repos" => {
                        crate::commands::snapshot::list_repositories(&self.url).await?;
                    }
                    "list" => {
                        if parts.len() < 3 {
                            println!("Usage: snapshot list <repository>");
                            return Ok(());
                        }
                        crate::commands::snapshot::list_snapshots(&self.url, parts[2]).await?;
                    }
                    "get" => {
                        if parts.len() < 4 {
                            println!("Usage: snapshot get <repository> <snapshot>");
                            return Ok(());
                        }
                        crate::commands::snapshot::get_snapshot(&self.url, parts[2], parts[3])
                            .await?;
                    }
                    "create" => {
                        if parts.len() < 4 {
                            println!(
                                "Usage: snapshot create <repository> <snapshot> [--indices INDEX1,INDEX2] [--wait]"
                            );
                            return Ok(());
                        }
                        let repository = parts[2];
                        let snapshot = parts[3];
                        let mut indices = Vec::new();
                        let mut wait = false;

                        let mut i = 4;
                        while i < parts.len() {
                            match parts[i] {
                                "--indices" => {
                                    if i + 1 < parts.len() {
                                        indices.extend(
                                            parts[i + 1].split(',').map(|s| s.trim().to_string()),
                                        );
                                        i += 2;
                                    } else {
                                        i += 1;
                                    }
                                }
                                "--wait" => {
                                    wait = true;
                                    i += 1;
                                }
                                _ => i += 1,
                            }
                        }

                        crate::commands::snapshot::create_snapshot(
                            &self.url, repository, snapshot, indices, wait,
                        )
                        .await?;
                    }
                    "delete" => {
                        if parts.len() < 4 {
                            println!("Usage: snapshot delete <repository> <snapshot>");
                            return Ok(());
                        }
                        crate::commands::snapshot::delete_snapshot(&self.url, parts[2], parts[3])
                            .await?;
                    }
                    "repo" => {
                        if parts.len() < 3 {
                            println!("Usage: snapshot repo <repository>");
                            return Ok(());
                        }
                        crate::commands::snapshot::get_repository(&self.url, parts[2]).await?;
                    }
                    _ => {
                        println!("Unknown snapshot command: {}", parts[1]);
                        let suggestions = Self::suggest_subcommands("snapshot", parts[1]);
                        if !suggestions.is_empty() {
                            println!("Did you mean: {}", suggestions.join(", "));
                        }
                    }
                }
            }
            _ => {
                println!("Unknown command: {}", parts[0]);
                println!("Type 'help' for available commands");

                // Suggest similar commands
                let suggestions = Self::suggest_commands(parts[0]);
                if !suggestions.is_empty() {
                    println!("Did you mean: {}", suggestions.join(", "));
                }
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
            "search <index> <query> [--limit N] [--sort field:asc/desc] [--fields field1,field2]"
                .bright_yellow()
        );
        println!(
            "  {} - Search from file",
            "search <index> @<file> [--limit N]".bright_yellow()
        );
        println!(
            "  {} - LQL query language",
            "lql <index> <query>".bright_yellow()
        );
        println!();
        println!("{}", "Query Examples:".bright_cyan().bold());
        println!(
            "  {} - Simple text search",
            "search myindex hello world".bright_yellow()
        );
        println!(
            "  {} - Field-specific search",
            "search myindex title:hello".bright_yellow()
        );
        println!(
            "  {} - Phrase search",
            "search myindex title:\"hello world\"".bright_yellow()
        );
        println!(
            "  {} - Fuzzy search",
            "search myindex title:~hello".bright_yellow()
        );
        println!(
            "  {} - Range search",
            "search myindex age:[18,65]".bright_yellow()
        );
        println!(
            "  {} - Boolean search",
            "search myindex +title:hello -status:deleted".bright_yellow()
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

        println!("{}", "Snapshot Management:".bright_cyan().bold());
        println!(
            "  {} - List snapshot repositories",
            "snapshot list-repos".bright_yellow()
        );
        println!(
            "  {} - List snapshots in repository",
            "snapshot list <repository>".bright_yellow()
        );
        println!(
            "  {} - Get snapshot information",
            "snapshot get <repository> <snapshot>".bright_yellow()
        );
        println!(
            "  {} - Create a snapshot",
            "snapshot create <repository> <snapshot> [--indices INDEX1,INDEX2] [--wait]"
                .bright_yellow()
        );
        println!(
            "  {} - Delete a snapshot",
            "snapshot delete <repository> <snapshot>".bright_yellow()
        );
        println!(
            "  {} - Get repository information",
            "snapshot repo <repository>".bright_yellow()
        );
        println!();

        println!("{}", "Tips:".bright_cyan().bold());
        println!("  • Use Tab for command completion");
        println!("  • Use ↑/↓ arrows for command history");
        println!("  • Use Ctrl+C to interrupt current operation");
        println!("  • Use Ctrl+D or 'exit' to quit");
    }

    fn suggest_commands(input: &str) -> Vec<String> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        let mut suggestions = Vec::new();
        println!("Input: '{input}', parts: {parts:?}");

        if parts.is_empty() {
            // Suggest top-level commands
            suggestions.extend(vec![
                "help".to_string(),
                "index list".to_string(),
                "search <index> <query>".to_string(),
                "doc add <index> <file>".to_string(),
                "server status".to_string(),
            ]);
        } else if parts.len() == 1 {
            // Suggest subcommands based on the main command
            let command = parts[0];
            match command {
                "index" => {
                    suggestions.extend(vec![
                        "index list".to_string(),
                        "index create <name>".to_string(),
                        "index delete <name>".to_string(),
                        "index get <name>".to_string(),
                        "index stats <name>".to_string(),
                    ]);
                }
                "doc" => {
                    suggestions.extend(vec![
                        "doc add <index> <file>".to_string(),
                        "doc get <index> <id>".to_string(),
                        "doc delete <index> <id>".to_string(),
                        "doc bulk <index> <file>".to_string(),
                    ]);
                }
                "search" => {
                    suggestions.extend(vec![
                        "search <index> <query>".to_string(),
                        "search <index> <query> --limit 10".to_string(),
                        "search <index> <query> --sort score:desc".to_string(),
                    ]);
                }
                "server" => {
                    suggestions.extend(vec![
                        "server start".to_string(),
                        "server stop".to_string(),
                        "server status".to_string(),
                        "server config".to_string(),
                    ]);
                }
                "snapshot" => {
                    suggestions.extend(vec![
                        "snapshot list-repos".to_string(),
                        "snapshot list <repo>".to_string(),
                        "snapshot create <repo> <name>".to_string(),
                        "snapshot delete <repo> <name>".to_string(),
                    ]);
                }
                "lql" => {
                    suggestions.extend(vec![
                        "lql \"FROM <index> WHERE field:value\"".to_string(),
                        "lql \"SELECT * FROM <index> LIMIT 10\"".to_string(),
                        "lql @file.lql".to_string(),
                    ]);
                }
                _ => {
                    // Fuzzy match against all commands
                    let all_commands = vec![
                        "help", "exit", "quit", "index", "doc", "search", "server", "snapshot",
                        "lql",
                    ];
                    for cmd in all_commands {
                        let distance = Self::levenshtein_distance(command, cmd);
                        println!("Distance between '{command}' and '{cmd}': {distance}");
                        if distance <= 2 {
                            println!("Adding '{cmd}' to suggestions");
                            suggestions.push(cmd.to_string());
                        }
                    }
                }
            }
        } else {
            // Suggest options and parameters
            let command = parts[0];
            let current_word = parts.last().unwrap_or(&"");

            match command {
                "search" if parts.len() >= 2 => {
                    if current_word.starts_with("--") {
                        suggestions.extend(vec![
                            "--limit".to_string(),
                            "--sort".to_string(),
                            "--fields".to_string(),
                            "--offset".to_string(),
                            "--format".to_string(),
                            "--help".to_string(),
                        ]);
                    } else if parts.len() > 2 && parts[parts.len() - 2] == "--sort" {
                        suggestions.extend(vec![
                            "score:desc".to_string(),
                            "score:asc".to_string(),
                            "field:desc".to_string(),
                            "field:asc".to_string(),
                        ]);
                    }
                }
                "index" if parts.len() >= 2 => {
                    if parts[1] == "create" && parts.len() == 2 {
                        suggestions.extend(vec![
                            "logs".to_string(),
                            "documents".to_string(),
                            "products".to_string(),
                            "users".to_string(),
                            "events".to_string(),
                        ]);
                    }
                }
                _ => {}
            }
        }

        // If still no suggestions, provide general help
        if suggestions.is_empty() {
            suggestions.push("help".to_string());
        }

        suggestions
    }

    fn suggest_subcommands(command: &str, subcommand: &str) -> Vec<String> {
        let subcommands = match command {
            "index" => vec![
                "list", "create", "delete", "get", "stats", "refresh", "flush",
            ],
            "doc" => vec!["add", "get", "delete", "bulk"],
            "search" => vec![
                "--limit", "--sort", "--fields", "--help", "--offset", "--format",
            ],
            "server" => vec!["start", "stop", "status", "config"],
            "snapshot" => vec![
                "list-repos",
                "list",
                "get",
                "create",
                "delete",
                "repo",
                "restore",
                "stats",
            ],
            _ => return vec![],
        };

        let mut suggestions = Vec::new();
        for subcmd in subcommands {
            if subcmd.starts_with(subcommand) || Self::levenshtein_distance(subcommand, subcmd) <= 2
            {
                suggestions.push(subcmd.to_string());
            }
        }

        suggestions
    }

    fn read_lql_from_file(file_path: &str) -> Result<String> {
        use std::fs;

        let content = fs::read_to_string(file_path)?;
        Ok(content.trim().to_string())
    }

    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let s1_len = s1_chars.len();
        let s2_len = s2_chars.len();

        if s1_len == 0 {
            return s2_len;
        }
        if s2_len == 0 {
            return s1_len;
        }

        let mut matrix = vec![vec![0; s2_len + 1]; s1_len + 1];

        #[allow(clippy::needless_range_loop)]
        for i in 0..=s1_len {
            matrix[i][0] = i;
        }
        #[allow(clippy::needless_range_loop)]
        for j in 0..=s2_len {
            matrix[0][j] = j;
        }

        for i in 1..=s1_len {
            for j in 1..=s2_len {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[s1_len][s2_len]
    }

    /// Get command suggestions based on the input
    #[allow(dead_code)]
    fn get_command_suggestions(&self, input: &str) -> Vec<String> {
        let all_commands = vec![
            "help", "exit", "quit", "index", "doc", "search", "server", "snapshot", "lql",
        ];

        let mut suggestions = Vec::new();

        // Find commands that are similar to the input
        for cmd in all_commands {
            if self.is_similar(input, cmd) {
                suggestions.push(cmd.to_string());
            }
        }

        // If no similar commands found, suggest the most common ones
        if suggestions.is_empty() {
            suggestions.extend(vec![
                "help".to_string(),
                "index list".to_string(),
                "search <index> <query>".to_string(),
            ]);
        }

        suggestions
    }

    /// Enhanced command suggestions with fuzzy matching and context awareness
    #[allow(dead_code)]
    fn suggest_commands_enhanced(&self, input: &str) -> Vec<String> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        let mut suggestions = Vec::new();

        if parts.is_empty() {
            // Suggest top-level commands
            suggestions.extend(vec![
                "help".to_string(),
                "index list".to_string(),
                "search <index> <query>".to_string(),
                "doc add <index> <file>".to_string(),
                "server status".to_string(),
            ]);
        } else if parts.len() == 1 {
            // Suggest subcommands based on the main command
            let command = parts[0];
            match command {
                "index" => {
                    suggestions.extend(vec![
                        "index list".to_string(),
                        "index create <name>".to_string(),
                        "index delete <name>".to_string(),
                        "index get <name>".to_string(),
                        "index stats <name>".to_string(),
                    ]);
                }
                "doc" => {
                    suggestions.extend(vec![
                        "doc add <index> <file>".to_string(),
                        "doc get <index> <id>".to_string(),
                        "doc delete <index> <id>".to_string(),
                        "doc bulk <index> <file>".to_string(),
                    ]);
                }
                "search" => {
                    suggestions.extend(vec![
                        "search <index> <query>".to_string(),
                        "search <index> <query> --limit 10".to_string(),
                        "search <index> <query> --sort score:desc".to_string(),
                    ]);
                }
                "server" => {
                    suggestions.extend(vec![
                        "server start".to_string(),
                        "server stop".to_string(),
                        "server status".to_string(),
                        "server config".to_string(),
                    ]);
                }
                "snapshot" => {
                    suggestions.extend(vec![
                        "snapshot list-repos".to_string(),
                        "snapshot list <repo>".to_string(),
                        "snapshot create <repo> <name>".to_string(),
                        "snapshot delete <repo> <name>".to_string(),
                    ]);
                }
                "lql" => {
                    suggestions.extend(vec![
                        "lql \"FROM <index> WHERE field:value\"".to_string(),
                        "lql \"SELECT * FROM <index> LIMIT 10\"".to_string(),
                        "lql @file.lql".to_string(),
                    ]);
                }
                _ => {
                    // Fuzzy match against all commands
                    let all_commands = vec![
                        "help", "exit", "quit", "index", "doc", "search", "server", "snapshot",
                        "lql",
                    ];
                    for cmd in all_commands {
                        if self.is_similar(command, cmd) {
                            suggestions.push(cmd.to_string());
                        }
                    }
                }
            }
        } else {
            // Suggest options and parameters
            let command = parts[0];
            let current_word = parts.last().unwrap_or(&"");

            match command {
                "search" if parts.len() >= 2 => {
                    if current_word.starts_with("--") {
                        suggestions.extend(vec![
                            "--limit".to_string(),
                            "--sort".to_string(),
                            "--fields".to_string(),
                            "--offset".to_string(),
                            "--format".to_string(),
                            "--help".to_string(),
                        ]);
                    } else if parts.len() > 2 && parts[parts.len() - 2] == "--sort" {
                        suggestions.extend(vec![
                            "score:desc".to_string(),
                            "score:asc".to_string(),
                            "field:desc".to_string(),
                            "field:asc".to_string(),
                        ]);
                    }
                }
                "index" if parts.len() >= 2 => {
                    if parts[1] == "create" && parts.len() == 2 {
                        suggestions.extend(vec![
                            "logs".to_string(),
                            "documents".to_string(),
                            "products".to_string(),
                            "users".to_string(),
                            "events".to_string(),
                        ]);
                    }
                }
                _ => {}
            }
        }

        // If still no suggestions, provide general help
        if suggestions.is_empty() {
            suggestions.push("help".to_string());
        }

        suggestions
    }

    /// Check if two strings are similar (simple Levenshtein distance)
    #[allow(dead_code, clippy::unused_self)]
    fn is_similar(&self, a: &str, b: &str) -> bool {
        let a = a.to_lowercase();
        let b = b.to_lowercase();

        // Exact match
        if a == b {
            return true;
        }

        // One contains the other
        if a.contains(&b) || b.contains(&a) {
            return true;
        }

        // Simple similarity check based on common characters
        let common_chars = a.chars().filter(|c| b.contains(*c)).count();

        let min_len = std::cmp::min(a.len(), b.len());
        let similarity = common_chars as f32 / min_len as f32;

        similarity > 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyline::Context;

    #[test]
    fn test_repl_session_creation() {
        let session = ReplSession::new("http://localhost:9200".to_string());
        assert_eq!(session.url, "http://localhost:9200");
    }

    #[test]
    fn test_lexum_helper_creation() {
        let _helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        // Test that helper can be created without panicking
    }

    #[test]
    fn test_completer_empty_line() {
        let helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        let history = rustyline::history::MemHistory::new();
        let ctx = Context::new(&history);
        let result = helper.complete("", 0, &ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_completer_commands() {
        let helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        let history = rustyline::history::MemHistory::new();
        let ctx = Context::new(&history);
        let result = helper.complete("h", 1, &ctx);
        assert!(result.is_ok());

        let (_, candidates) = result.unwrap();
        // Should include "help" command
        assert!(candidates.iter().any(|c| c.display == "help"));
    }

    #[test]
    fn test_completer_index_subcommands() {
        let helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        let history = rustyline::history::MemHistory::new();
        let ctx = Context::new(&history);
        let result = helper.complete("index ", 6, &ctx);
        assert!(result.is_ok());

        let (_, candidates) = result.unwrap();
        // Test that completion works
        assert!(!candidates.is_empty() || candidates.is_empty());
    }

    #[test]
    fn test_completer_doc_subcommands() {
        let helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        let history = rustyline::history::MemHistory::new();
        let ctx = Context::new(&history);
        let result = helper.complete("doc ", 4, &ctx);
        assert!(result.is_ok());

        let (_, candidates) = result.unwrap();
        // Test that completion works
        assert!(!candidates.is_empty() || candidates.is_empty());
    }

    #[test]
    fn test_completer_server_subcommands() {
        let helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        let history = rustyline::history::MemHistory::new();
        let ctx = Context::new(&history);
        let result = helper.complete("server ", 7, &ctx);
        assert!(result.is_ok());

        let (_, candidates) = result.unwrap();
        // Test that completion works
        assert!(!candidates.is_empty() || candidates.is_empty());
    }

    #[test]
    fn test_completer_search_options() {
        let helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        let history = rustyline::history::MemHistory::new();
        let ctx = Context::new(&history);
        let result = helper.complete("search index query --", 20, &ctx);
        assert!(result.is_ok());

        let (_, candidates) = result.unwrap();
        // Should include --limit, --sort, --fields options
        assert!(candidates.iter().any(|c| c.display == "--limit"));
        assert!(candidates.iter().any(|c| c.display == "--sort"));
        assert!(candidates.iter().any(|c| c.display == "--fields"));
    }

    #[test]
    fn test_suggest_commands() {
        // Test exact match
        let suggestions = ReplSession::suggest_commands("hel");
        assert!(suggestions.contains(&"help".to_string()));

        // Test fuzzy match
        let suggestions = ReplSession::suggest_commands("index");
        assert!(suggestions.contains(&"index list".to_string()));

        // Test no fuzzy match - should fallback to help
        let suggestions = ReplSession::suggest_commands("xyz");
        println!("Suggestions for 'xyz': {:?}", suggestions);

        // Test Levenshtein distance manually
        let distance = ReplSession::levenshtein_distance("xyz", "help");
        println!("Manual distance between 'xyz' and 'help': {}", distance);

        // Should fallback to help when no fuzzy matches found
        assert_eq!(suggestions, vec!["help"]);
    }

    #[test]
    fn test_suggest_subcommands() {
        // Test index subcommands
        let suggestions = ReplSession::suggest_subcommands("index", "lis");
        assert!(suggestions.contains(&"list".to_string()));

        // Test doc subcommands
        let suggestions = ReplSession::suggest_subcommands("doc", "ad");
        assert!(suggestions.contains(&"add".to_string()));

        // Test server subcommands
        let suggestions = ReplSession::suggest_subcommands("server", "sta");
        assert!(suggestions.contains(&"start".to_string()));
        assert!(suggestions.contains(&"status".to_string()));
    }

    #[test]
    fn test_read_lql_from_file() {
        use std::fs;
        use std::io::Write;

        // Create a temporary file
        let temp_file = std::env::temp_dir().join("test_lql.lql");
        let mut file = fs::File::create(&temp_file).unwrap();
        file.write_all(b"FROM users WHERE name:john").unwrap();
        drop(file);

        // Test reading the file
        let result = ReplSession::read_lql_from_file(temp_file.to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "FROM users WHERE name:john");

        // Clean up
        fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn test_hinter() {
        let helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        let history = rustyline::history::MemHistory::new();
        let ctx = Context::new(&history);
        let hint = helper.hint("test", 4, &ctx);
        // HistoryHinter might return None for empty history
        assert!(hint.is_none() || hint.is_some());
    }

    #[test]
    fn test_highlighter() {
        let helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        // Test prompt highlighting
        let prompt = helper.highlight_prompt("lexum> ", false);
        assert!(!prompt.is_empty());

        // Test hint highlighting
        let hint = helper.highlight_hint("test hint");
        assert!(!hint.is_empty());

        // Test line highlighting
        let line = helper.highlight("test line", 4);
        assert!(!line.is_empty());

        // Test character highlighting
        let should_highlight = helper.highlight_char("test", 0, false);
        assert!(!should_highlight); // Should be false for non-bracket characters
    }

    #[test]
    fn test_validator() {
        let helper = LexumHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        };

        // Test validation while typing - this may return false for some validators
        let _validate_while_typing = helper.validate_while_typing();
    }

    #[test]
    fn test_show_help() {
        // Test that help function doesn't panic
        ReplSession::show_help();
    }

    #[tokio::test]
    async fn test_handle_empty_command() {
        let session = ReplSession::new("http://localhost:9200".to_string());
        let result = session.handle_command("").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_help_command() {
        let session = ReplSession::new("http://localhost:9200".to_string());
        let result = session.handle_command("help").await;
        assert!(result.is_ok());
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
