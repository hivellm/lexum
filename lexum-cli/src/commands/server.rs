//! Server management commands

use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::process::{Command, Stdio};
use tokio::fs;

/// Start the Lexum server
pub async fn start(config_path: &str, daemon: bool) -> Result<()> {
    println!("{}", "Starting Lexum server...".bright_cyan().bold());

    // Check if config file exists
    if fs::metadata(config_path).await.is_err() {
        println!(
            "{}",
            format!("Warning: Config file '{config_path}' not found, using defaults")
                .bright_yellow()
        );
    }

    if daemon {
        println!("{}", "Starting server in daemon mode...".bright_yellow());

        // Start server in background
        let child = Command::new("lexum-server")
            .arg("--config")
            .arg(config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start lexum-server daemon")?;

        // Store PID for later use
        let pid = child.id();
        println!(
            "{}",
            format!("Server started with PID: {pid}").bright_green()
        );
        println!(
            "{}",
            "Use 'lexum server stop' to stop the server".bright_cyan()
        );
    } else {
        println!("{}", "Starting server in foreground...".bright_yellow());
        println!("{}", "Press Ctrl+C to stop the server".bright_cyan());

        // Start server in foreground
        let status = Command::new("lexum-server")
            .arg("--config")
            .arg(config_path)
            .status()
            .context("Failed to start lexum-server")?;

        if !status.success() {
            anyhow::bail!("Server exited with error code: {:?}", status.code());
        }
    }

    Ok(())
}

/// Stop the Lexum server
pub async fn stop(_server_url: &str) -> Result<()> {
    println!("{}", "Stopping Lexum server...".bright_cyan().bold());

    // Try to find running server process
    let output = Command::new("pgrep")
        .arg("-f")
        .arg("lexum-server")
        .output()
        .context("Failed to find server process")?;

    if output.stdout.is_empty() {
        println!("{}", "No running Lexum server found".bright_yellow());
        return Ok(());
    }

    let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!(
        "{}",
        format!("Found server process with PID: {pid}").bright_green()
    );

    // Send SIGTERM to gracefully shutdown
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(&pid)
        .status()
        .context("Failed to send SIGTERM to server")?;

    if !status.success() {
        anyhow::bail!("Failed to stop server process");
    }

    // Wait a bit for graceful shutdown
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Check if process is still running
    let check_output = Command::new("pgrep")
        .arg("-f")
        .arg("lexum-server")
        .output()
        .context("Failed to check server status")?;

    if check_output.stdout.is_empty() {
        println!("{}", "Server stopped successfully".bright_green());
    } else {
        println!(
            "{}",
            "Server still running, forcing shutdown...".bright_yellow()
        );

        // Force kill if still running
        let _ = Command::new("kill").arg("-KILL").arg(&pid).status();

        println!("{}", "Server force stopped".bright_green());
    }

    Ok(())
}

/// Get server status
pub async fn status(server_url: &str) -> Result<()> {
    println!("{}", "Checking Lexum server status...".bright_cyan().bold());

    // Check if server process is running
    let output = Command::new("pgrep")
        .arg("-f")
        .arg("lexum-server")
        .output()
        .context("Failed to check server process")?;

    if output.stdout.is_empty() {
        println!("{}", "❌ Server is not running".bright_red());
        return Ok(());
    }

    let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!(
        "{}",
        format!("✅ Server is running (PID: {pid})").bright_green()
    );

    // Try to connect to server
    let client = reqwest::Client::new();
    match client
        .get(format!("{server_url}/health"))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                println!("{}", "✅ Server is responding to requests".bright_green());

                // Try to get server info
                if let Ok(info_response) = client
                    .get(format!("{server_url}/"))
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                {
                    if let Ok(info) = info_response.json::<Value>().await {
                        if let Some(version) = info.get("version") {
                            println!("{}", format!("📦 Version: {version}").bright_cyan());
                        }
                        if let Some(uptime) = info.get("uptime") {
                            println!("{}", format!("⏱️  Uptime: {uptime}s").bright_cyan());
                        }
                    }
                }
            } else {
                println!(
                    "{}",
                    format!("⚠️  Server responded with status: {}", response.status())
                        .bright_yellow()
                );
            }
        }
        Err(e) => {
            println!(
                "{}",
                format!("❌ Server is not responding: {e}").bright_red()
            );
        }
    }

    Ok(())
}

/// Validate configuration file
pub async fn validate_config(config_path: &str) -> Result<()> {
    println!(
        "{}",
        format!("Validating configuration file: {config_path}")
            .bright_cyan()
            .bold()
    );

    // Check if file exists
    if fs::metadata(config_path).await.is_err() {
        anyhow::bail!("Configuration file '{config_path}' not found");
    }

    // Read and parse YAML
    let content = fs::read_to_string(config_path)
        .await
        .context("Failed to read configuration file")?;

    let config: serde_yaml::Value =
        serde_yaml::from_str(&content).context("Failed to parse YAML configuration")?;

    // Basic validation
    let mut errors: Vec<String> = Vec::new();

    // Check required fields
    if config.get("server").is_none() {
        errors.push("Missing 'server' section".to_string());
    }

    if let Some(server) = config.get("server") {
        if server.get("host").is_none() {
            errors.push("Missing 'server.host'".to_string());
        }
        if server.get("port").is_none() {
            errors.push("Missing 'server.port'".to_string());
        }
    }

    if config.get("storage").is_none() {
        errors.push("Missing 'storage' section".to_string());
    }

    if config.get("logging").is_none() {
        errors.push("Missing 'logging' section".to_string());
    }

    // Check for unknown fields
    let known_sections = ["server", "storage", "logging", "indices", "security"];
    if let Some(map) = config.as_mapping() {
        for key in map.keys() {
            if let Some(key_str) = key.as_str() {
                if !known_sections.contains(&key_str) {
                    errors.push(format!("Unknown section: '{key_str}'"));
                }
            }
        }
    }

    if errors.is_empty() {
        println!("{}", "✅ Configuration file is valid".bright_green());

        // Show configuration summary
        if let Some(server) = config.get("server") {
            if let Some(host) = server.get("host") {
                if let Some(port) = server.get("port") {
                    println!(
                        "{}",
                        format!(
                            "🌐 Server: {}:{}",
                            host.as_str().unwrap_or("unknown"),
                            port.as_str().unwrap_or("unknown")
                        )
                        .bright_cyan()
                    );
                }
            }
        }

        if let Some(storage) = config.get("storage") {
            if let Some(path) = storage.get("path") {
                println!(
                    "{}",
                    format!("💾 Storage: {}", path.as_str().unwrap_or("unknown")).bright_cyan()
                );
            }
        }
    } else {
        println!("{}", "❌ Configuration validation failed:".bright_red());
        for error in errors {
            println!("  • {}", error.bright_red());
        }
        anyhow::bail!("Configuration validation failed");
    }

    Ok(())
}
