//! Help and documentation commands

use colored::Colorize;

/// Show comprehensive help for all commands
pub fn show_comprehensive_help() {
    println!(
        "{}",
        "Lexum CLI - Complete Command Reference"
            .bright_cyan()
            .bold()
    );
    println!("Version: {}\n", env!("CARGO_PKG_VERSION").bright_yellow());

    // Server Commands
    show_server_help();
    println!();

    // Index Commands
    show_index_help();
    println!();

    // Document Commands
    show_document_help();
    println!();

    // Search Commands
    show_search_help();
    println!();

    // Snapshot Commands
    show_snapshot_help();
    println!();

    // Examples
    show_examples();
}

fn show_server_help() {
    println!("{}", "SERVER MANAGEMENT:".bright_cyan().bold());
    println!(
        "  {}    Start the Lexum server",
        "server start [--config FILE] [--daemon]".bright_yellow()
    );
    println!(
        "  {}    Stop the Lexum server",
        "server stop".bright_yellow()
    );
    println!(
        "  {}    Check server status",
        "server status".bright_yellow()
    );
    println!(
        "  {}    Validate configuration file",
        "server config [FILE]".bright_yellow()
    );
    println!();
    println!("    Examples:");
    println!(
        "      {}",
        "lexum server start --config my-config.yml".bright_green()
    );
    println!("      {}", "lexum server start --daemon".bright_green());
    println!("      {}", "lexum server status".bright_green());
    println!(
        "      {}",
        "lexum server config validate config.yml".bright_green()
    );
}

fn show_index_help() {
    println!("{}", "INDEX MANAGEMENT:".bright_cyan().bold());
    println!("  {}    List all indices", "index list".bright_yellow());
    println!(
        "  {}    Create a new index",
        "index create <NAME> --schema <FILE>".bright_yellow()
    );
    println!(
        "  {}    Get index information",
        "index get <NAME>".bright_yellow()
    );
    println!(
        "  {}    Get index statistics",
        "index stats <NAME>".bright_yellow()
    );
    println!(
        "  {}    Delete an index",
        "index delete <NAME>".bright_yellow()
    );
    println!();
    println!("    Examples:");
    println!("      {}", "lexum index list".bright_green());
    println!(
        "      {}",
        "lexum index create my_index --schema schema.yml".bright_green()
    );
    println!("      {}", "lexum index get my_index".bright_green());
    println!("      {}", "lexum index stats my_index".bright_green());
    println!("      {}", "lexum index delete my_index".bright_green());
}

fn show_document_help() {
    println!("{}", "DOCUMENT OPERATIONS:".bright_cyan().bold());
    println!(
        "  {}    Add a document",
        "doc add <INDEX> --file <FILE>".bright_yellow()
    );
    println!(
        "  {}    Get a document",
        "doc get <INDEX> <ID>".bright_yellow()
    );
    println!(
        "  {}    Delete a document",
        "doc delete <INDEX> <ID>".bright_yellow()
    );
    println!(
        "  {}    Bulk index documents",
        "doc bulk <INDEX> --file <FILE>".bright_yellow()
    );
    println!();
    println!("    Examples:");
    println!(
        "      {}",
        "lexum doc add my_index --file document.json".bright_green()
    );
    println!("      {}", "lexum doc get my_index doc_123".bright_green());
    println!(
        "      {}",
        "lexum doc delete my_index doc_123".bright_green()
    );
    println!(
        "      {}",
        "lexum doc bulk my_index --file documents.json".bright_green()
    );
}

fn show_search_help() {
    println!("{}", "SEARCH:".bright_cyan().bold());
    println!(
        "  {}    Search documents",
        "search <INDEX> <QUERY> [--limit N]".bright_yellow()
    );
    println!();
    println!("    Examples:");
    println!(
        "      {}",
        "lexum search my_index \"search query\"".bright_green()
    );
    println!(
        "      {}",
        "lexum search my_index \"search query\" --limit 20".bright_green()
    );
    println!(
        "      {}",
        "lexum search my_index \"*\" --limit 100".bright_green()
    );
}

fn show_snapshot_help() {
    println!("{}", "SNAPSHOT MANAGEMENT:".bright_cyan().bold());
    println!(
        "  {}    List snapshot repositories",
        "snapshot list-repos".bright_yellow()
    );
    println!(
        "  {}    List snapshots in repository",
        "snapshot list <REPOSITORY>".bright_yellow()
    );
    println!(
        "  {}    Get snapshot information",
        "snapshot get <REPOSITORY> <SNAPSHOT>".bright_yellow()
    );
    println!(
        "  {}    Create a snapshot",
        "snapshot create <REPOSITORY> <SNAPSHOT> [--indices INDEX1,INDEX2] [--wait]".bright_yellow()
    );
    println!(
        "  {}    Delete a snapshot",
        "snapshot delete <REPOSITORY> <SNAPSHOT>".bright_yellow()
    );
    println!(
        "  {}    Get repository information",
        "snapshot repo <REPOSITORY>".bright_yellow()
    );
    println!();
    println!("    Examples:");
    println!(
        "      {}",
        "lexum snapshot list-repos".bright_green()
    );
    println!(
        "      {}",
        "lexum snapshot list my_repo".bright_green()
    );
    println!(
        "      {}",
        "lexum snapshot create my_repo backup_2024 --indices index1,index2 --wait".bright_green()
    );
    println!(
        "      {}",
        "lexum snapshot get my_repo backup_2024".bright_green()
    );
    println!(
        "      {}",
        "lexum snapshot delete my_repo old_backup".bright_green()
    );
}

fn show_examples() {
    println!("{}", "COMPLETE WORKFLOW EXAMPLES:".bright_cyan().bold());
    println!();

    println!("{}", "1. Start a new Lexum server:".bright_yellow().bold());
    println!("   {}", "lexum server start --daemon".bright_green());
    println!("   {}", "lexum server status".bright_green());
    println!();

    println!(
        "{}",
        "2. Create an index with schema:".bright_yellow().bold()
    );
    println!("   {}", "# Create schema.yml file first".bright_cyan());
    println!(
        "   {}",
        "lexum index create my_index --schema schema.yml".bright_green()
    );
    println!("   {}", "lexum index list".bright_green());
    println!();

    println!("{}", "3. Add documents:".bright_yellow().bold());
    println!("   {}", "# Create document.json file first".bright_cyan());
    println!(
        "   {}",
        "lexum doc add my_index --file document.json".bright_green()
    );
    println!(
        "   {}",
        "lexum doc bulk my_index --file documents.json".bright_green()
    );
    println!();

    println!("{}", "4. Search documents:".bright_yellow().bold());
    println!(
        "   {}",
        "lexum search my_index \"search query\"".bright_green()
    );
    println!(
        "   {}",
        "lexum search my_index \"*\" --limit 50".bright_green()
    );
    println!();

    println!("{}", "5. Interactive mode:".bright_yellow().bold());
    println!("   {}", "lexum repl".bright_green());
    println!(
        "   {}",
        "# Then use commands like: index list, search my_index \"query\"".bright_cyan()
    );
    println!();

    println!(
        "{}",
        "SCHEMA FILE FORMAT (schema.yml):".bright_cyan().bold()
    );
    println!("{}", "```yaml".bright_cyan());
    println!("{}", "- name: title".bright_cyan());
    println!("{}", "  type: text".bright_cyan());
    println!("{}", "  stored: true".bright_cyan());
    println!("{}", "  indexed: true".bright_cyan());
    println!("{}", "- name: content".bright_cyan());
    println!("{}", "  type: text".bright_cyan());
    println!("{}", "  stored: true".bright_cyan());
    println!("{}", "  indexed: true".bright_cyan());
    println!("{}", "- name: views".bright_cyan());
    println!("{}", "  type: i64".bright_cyan());
    println!("{}", "  stored: true".bright_cyan());
    println!("{}", "  fast: true".bright_cyan());
    println!("{}", "```".bright_cyan());
    println!();

    println!("{}", "SUPPORTED FIELD TYPES:".bright_cyan().bold());
    println!("  {} - Full-text search", "text".bright_yellow());
    println!("  {} - Exact matching", "keyword".bright_yellow());
    println!("  {} - 64-bit integer", "i64".bright_yellow());
    println!("  {} - 64-bit float", "f64".bright_yellow());
    println!("  {} - Date/timestamp", "date".bright_yellow());
    println!("  {} - True/false", "boolean".bright_yellow());
    println!();

    println!(
        "{}",
        "For more information, visit: https://github.com/your-org/lexum".bright_cyan()
    );
}
