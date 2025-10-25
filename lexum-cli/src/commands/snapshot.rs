//! Snapshot management commands

use anyhow::Result;
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};
// Note: RepositoryName and SnapshotName are used in type definitions but not directly in this module
use serde::{Deserialize, Serialize};

/// Snapshot information response
#[derive(Debug, Deserialize, Serialize)]
pub struct SnapshotInfo {
    /// The name of the snapshot
    pub name: String,
    /// The repository name
    pub repository: String,
    /// The current state of the snapshot
    pub state: String,
    /// List of indices included in the snapshot
    pub indices: Vec<String>,
    /// When the snapshot was started
    pub start_time: String,
    /// When the snapshot was completed (if finished)
    pub end_time: Option<String>,
    /// Duration of the snapshot in milliseconds
    pub duration_in_millis: Option<u64>,
    /// Number of failures during snapshot creation
    pub failures: u32,
    /// Information about shards
    pub shards: ShardInfo,
    /// Additional metadata about the snapshot
    pub metadata: SnapshotMetadata,
}

/// Shard information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShardInfo {
    /// Total number of shards
    pub total: u32,
    /// Number of successful shards
    pub successful: u32,
    /// Number of failed shards
    pub failed: u32,
}

/// Snapshot metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SnapshotMetadata {
    /// User-defined metadata key-value pairs
    pub user_metadata: std::collections::HashMap<String, String>,
    /// Version of the snapshot format
    pub version: String,
    /// When the snapshot was created
    pub creation_time: String,
}

/// Snapshot list response
#[derive(Debug, Deserialize, Serialize)]
pub struct SnapshotListResponse {
    /// List of snapshots
    pub snapshots: Vec<SnapshotInfo>,
}

/// Repository information response
#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoryInfo {
    /// The name of the repository
    pub name: String,
    /// The type of the repository
    #[serde(rename = "type")]
    pub repository_type: String,
    /// Repository configuration settings
    pub settings: std::collections::HashMap<String, String>,
    /// Number of snapshots in the repository
    pub snapshot_count: u32,
    /// Total size of all snapshots in bytes
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
    let response: SnapshotListResponse =
        client.get(&format!("/_snapshot/{repository}/_all")).await?;

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
            .map(|d| format!("{d}ms"))
            .unwrap_or_else(|| "N/A".to_string());

        let shards_str = format!("{}/{}", snapshot.shards.successful, snapshot.shards.total);

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
        .get(&format!("/_snapshot/{repository}/{snapshot}"))
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

    println!(
        "{}: {}",
        "Start Time".bright_cyan(),
        format_time(&response.start_time)
    );

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
        .put(&format!("/_snapshot/{repository}/{snapshot}"), &request)
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
        .delete(&format!("/_snapshot/{repository}/{snapshot}"))
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
    let response: RepositoryInfo = client.get(&format!("/_snapshot/{repository}")).await?;

    println!("{}", "Repository Information:".bright_cyan().bold());
    println!();

    println!("{}: {}", "Name".bright_cyan(), response.name);
    println!("{}: {}", "Type".bright_cyan(), response.repository_type);
    println!("{}: {}", "Snapshots".bright_cyan(), response.snapshot_count);
    println!(
        "{}: {}",
        "Total Size".bright_cyan(),
        format_size(response.total_size)
    );

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_snapshot_info_serialization() {
        let mut user_metadata = HashMap::new();
        user_metadata.insert("description".to_string(), "test snapshot".to_string());
        
        let metadata = SnapshotMetadata {
            user_metadata,
            version: "1.0".to_string(),
            creation_time: "2024-01-01T00:00:00Z".to_string(),
        };
        
        let shards = ShardInfo {
            total: 5,
            successful: 5,
            failed: 0,
        };
        
        let snapshot = SnapshotInfo {
            name: "test_snapshot".to_string(),
            repository: "test_repo".to_string(),
            state: "Success".to_string(),
            indices: vec!["index1".to_string(), "index2".to_string()],
            start_time: "2024-01-01T00:00:00Z".to_string(),
            end_time: Some("2024-01-01T00:01:00Z".to_string()),
            duration_in_millis: Some(60000),
            failures: 0,
            shards,
            metadata,
        };
        
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: SnapshotInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(snapshot.name, deserialized.name);
        assert_eq!(snapshot.repository, deserialized.repository);
        assert_eq!(snapshot.state, deserialized.state);
        assert_eq!(snapshot.indices, deserialized.indices);
        assert_eq!(snapshot.shards.total, deserialized.shards.total);
        assert_eq!(snapshot.metadata.version, deserialized.metadata.version);
    }

    #[test]
    fn test_repository_info_serialization() {
        let mut settings = HashMap::new();
        settings.insert("location".to_string(), "/backup".to_string());
        
        let repo = RepositoryInfo {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings,
            snapshot_count: 5,
            total_size: 1024000,
        };
        
        let json = serde_json::to_string(&repo).unwrap();
        let deserialized: RepositoryInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(repo.name, deserialized.name);
        assert_eq!(repo.repository_type, deserialized.repository_type);
        assert_eq!(repo.snapshot_count, deserialized.snapshot_count);
        assert_eq!(repo.total_size, deserialized.total_size);
    }

    #[test]
    fn test_snapshot_list_response_serialization() {
        let mut user_metadata = HashMap::new();
        user_metadata.insert("description".to_string(), "test".to_string());
        
        let metadata = SnapshotMetadata {
            user_metadata,
            version: "1.0".to_string(),
            creation_time: "2024-01-01T00:00:00Z".to_string(),
        };
        
        let shards = ShardInfo {
            total: 3,
            successful: 3,
            failed: 0,
        };
        
        let snapshot = SnapshotInfo {
            name: "snapshot1".to_string(),
            repository: "repo1".to_string(),
            state: "Success".to_string(),
            indices: vec!["index1".to_string()],
            start_time: "2024-01-01T00:00:00Z".to_string(),
            end_time: Some("2024-01-01T00:00:30Z".to_string()),
            duration_in_millis: Some(30000),
            failures: 0,
            shards,
            metadata,
        };
        
        let response = SnapshotListResponse {
            snapshots: vec![snapshot],
        };
        
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: SnapshotListResponse = serde_json::from_str(&json).unwrap();
        
        assert_eq!(response.snapshots.len(), deserialized.snapshots.len());
        assert_eq!(response.snapshots[0].name, deserialized.snapshots[0].name);
    }

    #[test]
    fn test_shard_info_serialization() {
        let shards = ShardInfo {
            total: 10,
            successful: 8,
            failed: 2,
        };
        
        let json = serde_json::to_string(&shards).unwrap();
        let deserialized: ShardInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(shards.total, deserialized.total);
        assert_eq!(shards.successful, deserialized.successful);
        assert_eq!(shards.failed, deserialized.failed);
    }

    #[test]
    fn test_snapshot_metadata_serialization() {
        let mut user_metadata = HashMap::new();
        user_metadata.insert("env".to_string(), "production".to_string());
        user_metadata.insert("backup_type".to_string(), "full".to_string());
        
        let metadata = SnapshotMetadata {
            user_metadata,
            version: "2.0".to_string(),
            creation_time: "2024-01-15T12:30:00Z".to_string(),
        };
        
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: SnapshotMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(metadata.version, deserialized.version);
        assert_eq!(metadata.creation_time, deserialized.creation_time);
        assert_eq!(metadata.user_metadata.len(), deserialized.user_metadata.len());
        assert_eq!(metadata.user_metadata.get("env"), deserialized.user_metadata.get("env"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
        assert_eq!(format_size(1099511627776), "1.0 TB");
    }

    #[test]
    fn test_format_time() {
        // Test with valid ISO 8601 timestamp
        let iso_timestamp = "2024-01-01T12:30:45Z";
        let formatted = format_time(iso_timestamp);
        assert!(formatted.contains("2024-01-01"));
        assert!(formatted.contains("12:30:45"));
        assert!(formatted.contains("UTC"));
        
        // Test with invalid timestamp (should return as-is)
        let invalid_timestamp = "not-a-timestamp";
        let formatted = format_time(invalid_timestamp);
        assert_eq!(formatted, invalid_timestamp);
        
        // Test with different timezone
        let tz_timestamp = "2024-01-01T12:30:45+00:00";
        let formatted = format_time(tz_timestamp);
        assert!(formatted.contains("2024-01-01"));
        assert!(formatted.contains("12:30:45"));
    }

    #[test]
    fn test_snapshot_info_with_optional_fields() {
        let mut user_metadata = HashMap::new();
        user_metadata.insert("test".to_string(), "value".to_string());
        
        let metadata = SnapshotMetadata {
            user_metadata,
            version: "1.0".to_string(),
            creation_time: "2024-01-01T00:00:00Z".to_string(),
        };
        
        let shards = ShardInfo {
            total: 1,
            successful: 1,
            failed: 0,
        };
        
        // Test with all optional fields present
        let snapshot_with_optionals = SnapshotInfo {
            name: "test".to_string(),
            repository: "repo".to_string(),
            state: "Success".to_string(),
            indices: vec!["index1".to_string()],
            start_time: "2024-01-01T00:00:00Z".to_string(),
            end_time: Some("2024-01-01T00:01:00Z".to_string()),
            duration_in_millis: Some(60000),
            failures: 0,
            shards: shards.clone(),
            metadata: metadata.clone(),
        };
        
        // Test with optional fields missing
        let snapshot_without_optionals = SnapshotInfo {
            name: "test2".to_string(),
            repository: "repo2".to_string(),
            state: "InProgress".to_string(),
            indices: vec![],
            start_time: "2024-01-01T00:00:00Z".to_string(),
            end_time: None,
            duration_in_millis: None,
            failures: 0,
            shards,
            metadata,
        };
        
        assert!(snapshot_with_optionals.end_time.is_some());
        assert!(snapshot_with_optionals.duration_in_millis.is_some());
        assert!(snapshot_without_optionals.end_time.is_none());
        assert!(snapshot_without_optionals.duration_in_millis.is_none());
    }

    #[test]
    fn test_repository_info_with_empty_settings() {
        let repo = RepositoryInfo {
            name: "empty_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: HashMap::new(),
            snapshot_count: 0,
            total_size: 0,
        };
        
        let json = serde_json::to_string(&repo).unwrap();
        let deserialized: RepositoryInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(repo.settings.len(), 0);
        assert_eq!(deserialized.settings.len(), 0);
        assert_eq!(repo.snapshot_count, 0);
        assert_eq!(repo.total_size, 0);
    }

    #[test]
    fn test_snapshot_states() {
        let states = vec!["Success", "Failed", "InProgress", "Partial", "Unknown"];
        
        for state in states {
            let mut user_metadata = HashMap::new();
            user_metadata.insert("state".to_string(), state.to_string());
            
            let metadata = SnapshotMetadata {
                user_metadata,
                version: "1.0".to_string(),
                creation_time: "2024-01-01T00:00:00Z".to_string(),
            };
            
            let shards = ShardInfo {
                total: 1,
                successful: 1,
                failed: 0,
            };
            
            let snapshot = SnapshotInfo {
                name: format!("snapshot_{}", state.to_lowercase()),
                repository: "test_repo".to_string(),
                state: state.to_string(),
                indices: vec!["test_index".to_string()],
                start_time: "2024-01-01T00:00:00Z".to_string(),
                end_time: if state == "InProgress" { None } else { Some("2024-01-01T00:01:00Z".to_string()) },
                duration_in_millis: if state == "InProgress" { None } else { Some(60000) },
                failures: if state == "Failed" { 1 } else { 0 },
                shards: if state == "Failed" { 
                    ShardInfo { total: 1, successful: 0, failed: 1 } 
                } else { 
                    shards 
                },
                metadata,
            };
            
            let json = serde_json::to_string(&snapshot).unwrap();
            let deserialized: SnapshotInfo = serde_json::from_str(&json).unwrap();
            
            assert_eq!(snapshot.state, deserialized.state);
            assert_eq!(snapshot.failures, deserialized.failures);
        }
    }
}
