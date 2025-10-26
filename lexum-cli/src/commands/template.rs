//! Template management commands

use crate::client::LexumClient;
use crate::formatter::{OutputFormat, format_output, print_error, print_success};
use anyhow::Result;

/// Template action subcommands
#[derive(clap::Subcommand)]
pub enum TemplateAction {
    /// Create a new template
    Create {
        /// Template name
        name: String,
        /// Template pattern
        pattern: String,
        /// Priority (higher = more important)
        #[arg(short, long, default_value = "0")]
        priority: i32,
    },
    /// List all templates
    List,
    /// Get template details
    Get {
        /// Template name
        name: String,
    },
    /// Delete a template
    Delete {
        /// Template name
        name: String,
    },
}

/// Handle template commands
pub async fn handle_template_command(
    action: TemplateAction,
    url: String,
    format: String,
) -> Result<()> {
    let client = LexumClient::new(url);
    let output_format = OutputFormat::parse(&format).unwrap_or(OutputFormat::JsonPretty);

    match action {
        TemplateAction::Create {
            name,
            pattern,
            priority,
        } => {
            let template = serde_json::json!({
                "name": name,
                "pattern": pattern,
                "priority": priority,
                "settings": {}
            });

            match client
                .post::<serde_json::Value, serde_json::Value>("/_template", &template)
                .await
            {
                Ok(_response) => {
                    print_success(&format!("Template '{name}' created successfully"));
                }
                Err(e) => {
                    print_error(&format!("Error creating template: {e}"));
                }
            }
        }
        TemplateAction::List => match client.get::<serde_json::Value>("/_template").await {
            Ok(templates) => {
                println!("{}", format_output(&templates, output_format)?);
            }
            Err(e) => {
                print_error(&format!("Error listing templates: {e}"));
            }
        },
        TemplateAction::Get { name } => {
            match client
                .get::<serde_json::Value>(&format!("/_template/{name}"))
                .await
            {
                Ok(template) => {
                    println!("{}", format_output(&template, output_format)?);
                }
                Err(e) => {
                    print_error(&format!("Error getting template: {e}"));
                }
            }
        }
        TemplateAction::Delete { name } => {
            match client.delete(&format!("/_template/{name}")).await {
                Ok(_response) => {
                    print_success(&format!("Template '{name}' deleted successfully"));
                }
                Err(e) => {
                    print_error(&format!("Error deleting template: {e}"));
                }
            }
        }
    }

    Ok(())
}

// Tests temporarily disabled due to mockito API changes
// TODO: Re-enable template tests with updated mockito API
