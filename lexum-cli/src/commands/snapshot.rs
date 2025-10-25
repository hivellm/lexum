//! Snapshot management commands

use anyhow::Result;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};
// Note: RepositoryName and SnapshotName are used in type definitions but not directly in this module
use serde::{Deserialize, Serialize};

/// Snapshot information response
#[derive(Debug, Deserialize, Serialize)]
pub struct SnapshotInfo {
    pub name: String,
    pub repository: String,
    pub state: String,
    pub indices: Vec<String>,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration_in_millis: Option<u64>,
    pub failures: u32,
    pub shards: ShardInfo,
    pub metadata: SnapshotMetadata,
}

/// Shard information
#[derive(Debug, Deserialize, Serialize)]
pub struct ShardInfo {
    pub total: u32,
    pub successful: u32,
    pub failed: u32,
}

/// Snapshot metadata
#[derive(Debug, Deserialize, Serialize)]
pub struct SnapshotMetadata {
    pub user_metadata: std::collections::HashMap<String, String>,
    pub version: String,
    pub creation_time: String,
}

/// Snapshot list response
#[derive(Debug, Deserialize, Serialize)]
pub struct SnapshotListResponse {
    pub snapshots: Vec<SnapshotInfo>,
}

/// Repository information response
#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoryInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub repository_type: String,
    pub settings: std::collections::HashMap<String, String>,
    pub snapshot_count: u32,
    pub total_size: u64,
}

/// List all snapshot repositories
pub async fn list_repositories(url: &str) -> Result<()> {
    let client = crate::client::LexumClient::new(url.to_string());
    let response: Vec<RepositoryInfo> = client.get("/_snapshot").await?;

    if response.is_empty() {
        println!("{}", "No snapshot repositories found".bright_yellow());
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Name", "Type", "Snapshots", "Total Size"]);

    for repo in response {
        let size_str = format_size(repo.total_size);
        table.add_row(vec![
            repo.name,
            repo.repository_type,
            repo.snapshot_count.to_string(),
            size_str,
        ]);
    }

    println!("{}", "Snapshot Repositories:".bright_cyan().bold());
    println!("{table}");

    Ok(())
}

/// List snapshots in a repository
pub async fn list_snapshots(url: &str, repository: &str) -> Result<()> {
    let client = crate::client::LexumClient::new(url.to_string());
    let response: SnapshotListResponse = client
        .get(&format!("/_snapshot/{}/_all", repository))
        .await?;

    if response.snapshots.is_empty() {
        println!(
            "{} No snapshots found in repository '{}'",
            "ℹ".bright_blue(),
            repository.bright_cyan()
        );
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        "Name",
        "State",
        "Indices",
        "Start Time",
        "Duration",
        "Failures",
        "Shards",
    ]);

    for snapshot in response.snapshots {
        let indices_str = if snapshot.indices.is_empty() {
            "none".to_string()
        } else {
            snapshot.indices.join(", ")
        };

        let duration_str = snapshot
            .duration_in_millis
            .map(|d| format!("{}ms", d))
            .unwrap_or_else(|| "N/A".to_string());

        let shards_str = format!(
            "{}/{}",
            snapshot.shards.successful,
            snapshot.shards.total
        );

        let state_color = match snapshot.state.as_str() {
            "Success" => snapshot.state.bright_green(),
            "Failed" => snapshot.state.bright_red(),
            "InProgress" => snapshot.state.bright_yellow(),
            "Partial" => snapshot.state.bright_magenta(),
            _ => snapshot.state.normal(),
        };

        table.add_row(vec![
            snapshot.name,
            state_color.to_string(),
            indices_str,
            format_time(&snapshot.start_time),
            duration_str,
            snapshot.failures.to_string(),
            shards_str,
        ]);
    }

    println!(
        "{} Snapshots in repository '{}':",
        "📸".bright_cyan(),
        repository.bright_cyan().bold()
    );
    println!("{table}");

    Ok(())
}

/// Get snapshot information
pub async fn get_snapshot(url: &str, repository: &str, snapshot: &str) -> Result<()> {
    let client = crate::client::LexumClient::new(url.to_string());
    let response: SnapshotInfo = client
        .get(&format!("/_snapshot/{}/{}", repository, snapshot))
        .await?;

    println!("{}", "Snapshot Information:".bright_cyan().bold());
    println!();

    println!("{}: {}", "Name".bright_cyan(), response.name);
    println!("{}: {}", "Repository".bright_cyan(), response.repository);
    println!(
        "{}: {}",
        "State".bright_cyan(),
        match response.state.as_str() {
            "Success" => response.state.bright_green(),
            "Failed" => response.state.bright_red(),
            "InProgress" => response.state.bright_yellow(),
            "Partial" => response.state.bright_magenta(),
            _ => response.state.normal(),
        }
    );

    println!(
        "{}: {}",
        "Indices".bright_cyan(),
        if response.indices.is_empty() {
            "none".to_string()
        } else {
            response.indices.join(", ")
        }
    );

    println!("{}: {}", "Start Time".bright_cyan(), format_time(&response.start_time));

    if let Some(end_time) = response.end_time {
        println!("{}: {}", "End Time".bright_cyan(), format_time(&end_time));
    }

    if let Some(duration) = response.duration_in_millis {
        println!("{}: {}ms", "Duration".bright_cyan(), duration);
    }

    println!("{}: {}", "Failures".bright_cyan(), response.failures);
    println!(
        "{}: {}/{}",
        "Shards".bright_cyan(),
        response.shards.successful,
        response.shards.total
    );

    if !response.shards.failed == 0 {
        println!(
            "{}: {}",
            "Failed Shards".bright_cyan(),
            response.shards.failed
        );
    }

    println!();
    println!("{}", "Metadata:".bright_cyan().bold());
    println!("{}: {}", "Version".bright_cyan(), response.metadata.version);
    println!(
        "{}: {}",
        "Creation Time".bright_cyan(),
        format_time(&response.metadata.creation_time)
    );

    if !response.metadata.user_metadata.is_empty() {
        println!("{}:", "User Metadata".bright_cyan());
        for (key, value) in &response.metadata.user_metadata {
            println!("  {}: {}", key.bright_yellow(), value);
        }
    }

    Ok(())
}

/// Create a snapshot
pub async fn create_snapshot(
    url: &str,
    repository: &str,
    snapshot: &str,
    indices: Vec<String>,
    wait_for_completion: bool,
) -> Result<()> {
    let client = crate::client::LexumClient::new(url.to_string());

    let request = serde_json::json!({
        "indices": indices,
        "wait_for_completion": wait_for_completion,
        "ignore_unavailable": false,
        "include_global_state": true
    });

    let response: serde_json::Value = client
        .put(&format!("/_snapshot/{}/{}", repository, snapshot), &request)
        .await?;

    if let Some(acknowledged) = response.get("acknowledged") {
        if acknowledged.as_bool().unwrap_or(false) {
            println!(
                "{} Snapshot '{}' creation {} in repository '{}'",
                "✓".bright_green().bold(),
                snapshot.bright_cyan(),
                if wait_for_completion {
                    "completed".bright_green()
                } else {
                    "started".bright_yellow()
                },
                repository.bright_cyan()
            );
        } else {
            println!(
                "{} Failed to create snapshot '{}'",
                "✗".bright_red().bold(),
                snapshot.bright_cyan()
            );
        }
    }

    Ok(())
}

/// Delete a snapshot
pub async fn delete_snapshot(url: &str, repository: &str, snapshot: &str) -> Result<()> {
    let client = crate::client::LexumClient::new(url.to_string());
    client
        .delete(&format!("/_snapshot/{}/{}", repository, snapshot))
        .await?;

    println!(
        "{} Snapshot '{}' deleted from repository '{}'",
        "✓".bright_green().bold(),
        snapshot.bright_cyan(),
        repository.bright_cyan()
    );

    Ok(())
}

/// Get repository information
pub async fn get_repository(url: &str, repository: &str) -> Result<()> {
    let client = crate::client::LexumClient::new(url.to_string());
    let response: RepositoryInfo = client
        .get(&format!("/_snapshot/{}", repository))
        .await?;

    println!("{}", "Repository Information:".bright_cyan().bold());
    println!();

    println!("{}: {}", "Name".bright_cyan(), response.name);
    println!("{}: {}", "Type".bright_cyan(), response.repository_type);
    println!("{}: {}", "Snapshots".bright_cyan(), response.snapshot_count);
    println!("{}: {}", "Total Size".bright_cyan(), format_size(response.total_size));

    if !response.settings.is_empty() {
        println!();
        println!("{}:", "Settings".bright_cyan().bold());
        for (key, value) in &response.settings {
            println!("  {}: {}", key.bright_yellow(), value);
        }
    }

    Ok(())
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Format timestamp for display
fn format_time(timestamp: &str) -> String {
    // Try to parse as ISO 8601 and format nicely
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        timestamp.to_string()
    }
}