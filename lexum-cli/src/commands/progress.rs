//! Progress tracking commands

use crate::client::LexumClient;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

/// Progress tracking information
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub id: String,
    pub operation_type: String,
    pub status: String,
    pub description: String,
    pub total: u64,
    pub completed: u64,
    pub failed: u64,
    pub percentage: f64,
    pub rate: f64,
    pub estimated_remaining: Option<u64>,
}

/// List all progress sessions
pub async fn list_progress(
    client: &LexumClient,
    operation_type: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
) -> Result<()> {
    let mut url = "/api/v1/progress".to_string();
    let mut params = Vec::new();

    if let Some(op_type) = operation_type {
        params.push(format!("operation_type={}", op_type));
    }
    if let Some(status) = status {
        params.push(format!("status={}", status));
    }
    if let Some(limit) = limit {
        params.push(format!("limit={}", limit));
    }

    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    let progress_sessions: Vec<Value> = client.get(&url).await?;

    if progress_sessions.is_empty() {
        println!("{}", "No progress sessions found".bright_yellow());
        return Ok(());
    }

    println!("{}", "Progress Sessions".bright_cyan().bold());
    println!("{}", "=".repeat(80).bright_cyan());

    for session in progress_sessions {
        let id = session["id"].as_str().unwrap_or("N/A");
        let op_type = session["operation_type"].as_str().unwrap_or("N/A");
        let status = session["status"].as_str().unwrap_or("N/A");
        let description = session["description"].as_str().unwrap_or("N/A");
        let metrics = &session["metrics"];
        let total = metrics["total"].as_u64().unwrap_or(0);
        let completed = metrics["completed"].as_u64().unwrap_or(0);
        let failed = metrics["failed"].as_u64().unwrap_or(0);
        let percentage = metrics["percentage"].as_f64().unwrap_or(0.0);
        let rate = metrics["rate"].as_f64().unwrap_or(0.0);

        let status_color = match status {
            "Running" => "bright_green",
            "Completed" => "bright_blue",
            "Failed" => "bright_red",
            "Cancelled" => "bright_yellow",
            "Paused" => "bright_magenta",
            _ => "white",
        };

        println!("ID: {}", id.bright_cyan());
        println!("Type: {}", op_type.bright_white());
        println!("Status: {}", status.color(status_color));
        println!("Description: {}", description.bright_white());
        println!("Progress: {}/{} ({:.1}%)", completed, total, percentage);
        println!("Rate: {:.1} ops/sec", rate);
        if failed > 0 {
            println!("Failed: {}", failed.to_string().bright_red());
        }
        println!("{}", "-".repeat(40).bright_black());
    }

    Ok(())
}

/// Get detailed progress information for a specific session
pub async fn get_progress(client: &LexumClient, progress_id: &str) -> Result<()> {
    let url = format!("/api/v1/progress/{}", progress_id);
    match client.get::<Value>(&url).await {
        Ok(session) => {
            print_progress_details(&session);
        }
        Err(_) => {
            println!("{}", "Progress session not found".bright_red());
        }
    }

    Ok(())
}

/// Monitor progress in real-time
pub async fn monitor_progress(
    client: &LexumClient,
    progress_id: &str,
    refresh_interval: Option<u64>,
) -> Result<()> {
    let interval = Duration::from_millis(refresh_interval.unwrap_or(1000));

    println!(
        "{}",
        "Monitoring progress (Press Ctrl+C to stop)"
            .bright_cyan()
            .bold()
    );
    println!("Progress ID: {}", progress_id.bright_yellow());
    println!();

    loop {
        let url = format!("/api/v1/progress/{}", progress_id);
        match client.get::<Value>(&url).await {
            Ok(session) => {
                let status = session["status"].as_str().unwrap_or("Unknown");

                // Clear screen and move cursor to top
                print!("\x1B[2J\x1B[1;1H");
                print_progress_details(&session);

                // Check if operation is complete
                if matches!(status, "Completed" | "Failed" | "Cancelled") {
                    println!();
                    println!("{}", "Operation finished!".bright_green().bold());
                    break;
                }
            }
            Err(e) => {
                println!("{}", format!("Error fetching progress: {}", e).bright_red());
                break;
            }
        }

        sleep(interval).await;
    }

    Ok(())
}

/// Cancel a progress operation
pub async fn cancel_progress(client: &LexumClient, progress_id: &str) -> Result<()> {
    let url = format!("/api/v1/progress/{}/cancel", progress_id);
    match client.post::<Value, Value>(&url, &Value::Null).await {
        Ok(_) => {
            println!("{}", "Operation cancelled successfully".bright_green());
        }
        Err(_) => {
            println!("{}", "Failed to cancel operation".bright_red());
        }
    }

    Ok(())
}

/// Pause a progress operation
pub async fn pause_progress(client: &LexumClient, progress_id: &str) -> Result<()> {
    let url = format!("/api/v1/progress/{}/pause", progress_id);
    match client.post::<Value, Value>(&url, &Value::Null).await {
        Ok(_) => {
            println!("{}", "Operation paused successfully".bright_green());
        }
        Err(_) => {
            println!("{}", "Failed to pause operation".bright_red());
        }
    }

    Ok(())
}

/// Resume a paused progress operation
pub async fn resume_progress(client: &LexumClient, progress_id: &str) -> Result<()> {
    let url = format!("/api/v1/progress/{}/resume", progress_id);
    match client.post::<Value, Value>(&url, &Value::Null).await {
        Ok(_) => {
            println!("{}", "Operation resumed successfully".bright_green());
        }
        Err(_) => {
            println!("{}", "Failed to resume operation".bright_red());
        }
    }

    Ok(())
}

/// Get progress statistics
pub async fn get_stats(client: &LexumClient) -> Result<()> {
    let stats: Value = client.get("/api/v1/progress/stats").await?;

    println!("{}", "Progress Statistics".bright_cyan().bold());
    println!("{}", "=".repeat(40).bright_cyan());
    println!(
        "Total Sessions: {}",
        stats["total_sessions"].as_u64().unwrap_or(0)
    );
    println!(
        "Active Sessions: {}",
        stats["active_sessions"].as_u64().unwrap_or(0)
    );
    println!(
        "Completed Sessions: {}",
        stats["completed_sessions"].as_u64().unwrap_or(0)
    );
    println!(
        "Failed Sessions: {}",
        stats["failed_sessions"].as_u64().unwrap_or(0)
    );

    if let Some(avg_time) = stats["avg_completion_time"].as_f64() {
        println!("Average Completion Time: {:.2}s", avg_time);
    }

    if let Some(common_op) = stats["most_common_operation"].as_str() {
        println!("Most Common Operation: {}", common_op);
    }

    Ok(())
}

/// Clean up old progress sessions
pub async fn cleanup_progress(client: &LexumClient, max_age_hours: Option<u64>) -> Result<()> {
    let url = if let Some(age) = max_age_hours {
        format!("/api/v1/progress/cleanup?max_age_hours={}", age)
    } else {
        "/api/v1/progress/cleanup".to_string()
    };

    let result: Value = client.post(&url, &Value::Null).await?;

    let cleaned = result["cleaned_sessions"].as_u64().unwrap_or(0);
    println!(
        "{}",
        format!("Cleaned up {} old progress sessions", cleaned).bright_green()
    );

    Ok(())
}

/// Print detailed progress information
fn print_progress_details(session: &Value) {
    let id = session["id"].as_str().unwrap_or("N/A");
    let op_type = session["operation_type"].as_str().unwrap_or("N/A");
    let status = session["status"].as_str().unwrap_or("N/A");
    let description = session["description"].as_str().unwrap_or("N/A");
    let start_time = session["start_time"].as_str().unwrap_or("N/A");
    let end_time = session["end_time"].as_str().unwrap_or("N/A");

    let metrics = &session["metrics"];
    let total = metrics["total"].as_u64().unwrap_or(0);
    let completed = metrics["completed"].as_u64().unwrap_or(0);
    let failed = metrics["failed"].as_u64().unwrap_or(0);
    let skipped = metrics["skipped"].as_u64().unwrap_or(0);
    let percentage = metrics["percentage"].as_f64().unwrap_or(0.0);
    let rate = metrics["rate"].as_f64().unwrap_or(0.0);
    let estimated_remaining = metrics["estimated_remaining"].as_u64();
    let current_phase = metrics["current_phase"].as_str();

    let status_color = match status {
        "Running" => "bright_green",
        "Completed" => "bright_blue",
        "Failed" => "bright_red",
        "Cancelled" => "bright_yellow",
        "Paused" => "bright_magenta",
        _ => "white",
    };

    println!("{}", "Progress Details".bright_cyan().bold());
    println!("{}", "=".repeat(50).bright_cyan());
    println!("ID: {}", id.bright_cyan());
    println!("Type: {}", op_type.bright_white());
    println!("Status: {}", status.color(status_color));
    println!("Description: {}", description.bright_white());
    println!("Started: {}", start_time.bright_white());
    if end_time != "N/A" {
        println!("Ended: {}", end_time.bright_white());
    }
    println!();

    // Progress bar
    let bar_width = 40;
    let filled = (percentage / 100.0 * bar_width as f64) as usize;
    let bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);
    println!(
        "Progress: [{}{}] {:.1}%",
        bar[..filled].bright_green(),
        bar[filled..].bright_black(),
        percentage
    );

    println!(
        "Completed: {}/{}",
        completed.to_string().bright_green(),
        total
    );
    if failed > 0 {
        println!("Failed: {}", failed.to_string().bright_red());
    }
    if skipped > 0 {
        println!("Skipped: {}", skipped.to_string().bright_yellow());
    }
    println!("Rate: {:.1} ops/sec", rate.to_string().bright_cyan());

    if let Some(remaining) = estimated_remaining {
        let hours = remaining / 3600;
        let minutes = (remaining % 3600) / 60;
        let seconds = remaining % 60;
        println!("ETA: {}h {}m {}s", hours, minutes, seconds);
    }

    if let Some(phase) = current_phase {
        println!("Current Phase: {}", phase.bright_magenta());
    }

    if let Some(error) = session["error"].as_str() {
        println!("Error: {}", error.bright_red());
    }
}
