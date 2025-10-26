//! Alias management commands for Lexum CLI

use anyhow::Result;
use colored::Colorize;
use crate::client::LexumClient;
use serde_json::json;

/// List all aliases
pub async fn list_aliases(url: &str) -> Result<()> {
    let client = LexumClient::new(url.to_string());
    let body: serde_json::Value = client.get("/_aliases").await?;
    
    println!("{}", "Aliases:".bright_blue().bold());
    
    if let Some(aliases) = body.as_object() {
        if aliases.is_empty() {
            println!("  No aliases found");
        } else {
            for (name, alias_info) in aliases {
                println!("  {} -> {}", name.bright_cyan(), 
                    alias_info["indices"].as_array()
                        .map(|indices| indices.iter()
                            .map(|i| i.as_str().unwrap_or(""))
                            .collect::<Vec<_>>()
                            .join(", "))
                        .unwrap_or_else(|| "[]".to_string())
                );
            }
        }
    }
    
    Ok(())
}

/// Get aliases for a specific index
pub async fn get_index_aliases(url: &str, index: &str) -> Result<()> {
    let client = LexumClient::new(url.to_string());
    let body: serde_json::Value = client.get(&format!("/{}/_alias", index)).await?;
    
    println!("{}", format!("Aliases for index '{}':", index).bright_blue().bold());
    
    if let Some(aliases) = body.as_object() {
        if aliases.is_empty() {
            println!("  No aliases found for this index");
        } else {
            for (name, alias_info) in aliases {
                println!("  {} -> {}", name.bright_cyan(), 
                    alias_info["indices"].as_array()
                        .map(|indices| indices.iter()
                            .map(|i| i.as_str().unwrap_or(""))
                            .collect::<Vec<_>>()
                            .join(", "))
                        .unwrap_or_else(|| "[]".to_string())
                );
            }
        }
    }
    
    Ok(())
}

/// Create an alias
pub async fn create_alias(url: &str, index: &str, alias: &str, config: Option<serde_json::Value>) -> Result<()> {
    let client = LexumClient::new(url.to_string());
    
    let request_body = json!({
        "actions": [{
            "action": "add",
            "index": index,
            "alias": alias,
            "filter": config.as_ref().and_then(|c| c.get("filter")),
            "routing": config.as_ref().and_then(|c| c.get("routing")),
            "search_routing": config.as_ref().and_then(|c| c.get("search_routing")),
            "index_routing": config.as_ref().and_then(|c| c.get("index_routing")),
            "is_write_index": config.as_ref().and_then(|c| c.get("is_write_index"))
        }]
    });
    
    let _response: serde_json::Value = client.post("/_aliases", &request_body).await?;
    println!("{}", format!("Alias '{}' created successfully for index '{}'", alias, index).bright_green());
    
    Ok(())
}

/// Delete an alias
pub async fn delete_alias(url: &str, index: &str, alias: &str) -> Result<()> {
    let client = LexumClient::new(url.to_string());
    client.delete(&format!("/{}/_alias/{}", index, alias)).await?;
    
    println!("{}", format!("Alias '{}' deleted successfully from index '{}'", alias, index).bright_green());
    
    Ok(())
}

/// Perform atomic alias operations
pub async fn atomic_operations(url: &str, operations: serde_json::Value) -> Result<()> {
    let client = LexumClient::new(url.to_string());
    let body: serde_json::Value = client.post("/_aliases/atomic", &operations).await?;
    
    println!("{}", "Atomic alias operations completed successfully".bright_green());
    
    if let Some(executed) = body.get("executed_operations") {
        println!("  Executed {} operations", executed);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    #[tokio::test]
    async fn test_list_aliases_success() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/_aliases")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"alias1": {"indices": ["index1", "index2"]}, "alias2": {"indices": ["index3"]}}"#)
            .create();

        let result = list_aliases(&server.url()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_aliases_empty() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/_aliases")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create();

        let result = list_aliases(&server.url()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_index_aliases_success() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("GET", "/test_index/_alias")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"alias1": {"indices": ["test_index"]}}"#)
            .create();

        let result = get_index_aliases(&server.url(), "test_index").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_alias_success() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/_aliases")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"acknowledged": true}"#)
            .create();

        let result = create_alias(&server.url(), "test_index", "test_alias", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_alias_with_config() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/_aliases")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"acknowledged": true}"#)
            .create();

        let config = json!({
            "filter": {"term": {"status": "active"}},
            "routing": "user1",
            "is_write_index": true
        });

        let result = create_alias(&server.url(), "test_index", "test_alias", Some(config)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_alias_success() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("DELETE", "/test_index/_alias/test_alias")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"acknowledged": true}"#)
            .create();

        let result = delete_alias(&server.url(), "test_index", "test_alias").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_atomic_operations_success() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/_aliases/atomic")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"acknowledged": true, "executed_operations": 2}"#)
            .create();

        let operations = json!({
            "actions": [
                {"action": "add", "index": "index1", "alias": "alias1"},
                {"action": "add", "index": "index2", "alias": "alias2"}
            ]
        });

        let result = atomic_operations(&server.url(), operations).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_alias_error() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/_aliases")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Alias already exists"}"#)
            .create();

        let result = create_alias(&server.url(), "test_index", "test_alias", None).await;
        assert!(result.is_ok()); // Function should handle error gracefully
    }

    #[tokio::test]
    async fn test_delete_alias_error() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("DELETE", "/test_index/_alias/test_alias")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Alias not found"}"#)
            .create();

        let result = delete_alias(&server.url(), "test_index", "test_alias").await;
        assert!(result.is_ok()); // Function should handle error gracefully
    }

    #[tokio::test]
    async fn test_atomic_operations_error() {
        let mut server = Server::new_async().await;
        let _m = server
            .mock("POST", "/_aliases/atomic")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid operation"}"#)
            .create();

        let operations = json!({
            "actions": [
                {"action": "invalid", "index": "index1", "alias": "alias1"}
            ]
        });

        let result = atomic_operations(&server.url(), operations).await;
        assert!(result.is_ok()); // Function should handle error gracefully
    }
}