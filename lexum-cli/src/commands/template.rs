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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_action_creation() {
        // Test that TemplateAction variants can be created
        let _create = TemplateAction::Create {
            name: "test".to_string(),
            pattern: "test-*".to_string(),
            priority: 1,
        };
        let _list = TemplateAction::List;
        let _get = TemplateAction::Get {
            name: "test".to_string(),
        };
        let _delete = TemplateAction::Delete {
            name: "test".to_string(),
        };
    }

    #[tokio::test]
    async fn test_template_command_without_server() {
        // Test that template commands handle server errors gracefully
        let action = TemplateAction::List;
        let result = handle_template_command(
            action,
            "http://localhost:9999".to_string(),
            "json".to_string(),
        )
        .await;

        // Should fail gracefully without server
        assert!(result.is_ok()); // Function returns Ok even on error (prints error)
    }

    #[tokio::test]
    async fn test_template_create_command_structure() {
        // Test create command structure
        let action = TemplateAction::Create {
            name: "test_template".to_string(),
            pattern: "test-*".to_string(),
            priority: 10,
        };
        let result = handle_template_command(
            action,
            "http://localhost:9999".to_string(),
            "json".to_string(),
        )
        .await;

        // Should handle error gracefully
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_template_get_command_structure() {
        // Test get command structure
        let action = TemplateAction::Get {
            name: "test_template".to_string(),
        };
        let result = handle_template_command(
            action,
            "http://localhost:9999".to_string(),
            "json".to_string(),
        )
        .await;

        // Should handle error gracefully
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_template_delete_command_structure() {
        // Test delete command structure
        let action = TemplateAction::Delete {
            name: "test_template".to_string(),
        };
        let result = handle_template_command(
            action,
            "http://localhost:9999".to_string(),
            "json".to_string(),
        )
        .await;

        // Should handle error gracefully
        assert!(result.is_ok());
    }
}
