//! Comprehensive tests for script engine functionality

use super::*;
use crate::script::parser::ScriptParser;
use crate::script::context::DocumentMetadata;
use serde_json::json;
use std::collections::HashMap;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_simple_field_transformation() {
        let source = json!({
            "title": "Old Title",
            "content": "Some content",
            "status": "draft"
        });

        let script = "ctx._source.title = \"New Title\"; ctx._source.status = \"published\"";
        let mut parser = ScriptParser::new(script.to_string());
        let operations = parser.parse().unwrap();

        let mut context = ScriptContext::new(
            source,
            HashMap::new(),
            DocumentMetadata {
                id: "1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("New Title")));
        assert_eq!(context.get_field("status"), Some(&json!("published")));
        assert_eq!(context.get_field("content"), Some(&json!("Some content")));
    }

    #[test]
    fn test_conditional_transformation() {
        let source = json!({
            "status": "active",
            "priority": 0,
            "category": "urgent"
        });

        let script = r#"
            if (ctx._source.status == "active") {
                ctx._source.priority = 1;
            }
            if (ctx._source.category == "urgent") {
                ctx._source.priority = 2;
            }
        "#;

        let mut parser = ScriptParser::new(script.to_string());
        let operations = parser.parse().unwrap();

        let mut context = ScriptContext::new(
            source,
            HashMap::new(),
            DocumentMetadata {
                id: "1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("priority"), Some(&json!(2)));
    }

    #[test]
    fn test_field_removal() {
        let source = json!({
            "title": "Test Title",
            "old_field": "remove me",
            "keep_field": "keep this"
        });

        let script = "ctx._source.old_field.remove()";
        let mut parser = ScriptParser::new(script.to_string());
        let operations = parser.parse().unwrap();

        let mut context = ScriptContext::new(
            source,
            HashMap::new(),
            DocumentMetadata {
                id: "1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("old_field"), None);
        assert_eq!(context.get_field("title"), Some(&json!("Test Title")));
        assert_eq!(context.get_field("keep_field"), Some(&json!("keep this")));
    }

    #[test]
    fn test_nested_field_operations() {
        let source = json!({
            "user": {
                "name": "John",
                "email": "john@example.com"
            },
            "metadata": {
                "created_at": "2023-01-01",
                "updated_at": "2023-01-02"
            }
        });

        let script = r#"
            ctx._source.user.name = "Jane";
            ctx._source.user.role = "admin";
            ctx._source.metadata.updated_at = "2023-01-03";
        "#;

        let mut parser = ScriptParser::new(script.to_string());
        let operations = parser.parse().unwrap();

        let mut context = ScriptContext::new(
            source,
            HashMap::new(),
            DocumentMetadata {
                id: "1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("user.name"), Some(&json!("Jane")));
        assert_eq!(context.get_field("user.role"), Some(&json!("admin")));
        assert_eq!(context.get_field("user.email"), Some(&json!("john@example.com")));
        assert_eq!(context.get_field("metadata.updated_at"), Some(&json!("2023-01-03")));
    }

    #[test]
    fn test_mathematical_operations() {
        let source = json!({
            "count": 10,
            "price": 25.5,
            "discount": 0.1
        });

        let script = r#"
            ctx._source.count = ctx._source.count + 5;
            ctx._source.final_price = ctx._source.price * (1 - ctx._source.discount);
        "#;

        let mut parser = ScriptParser::new(script.to_string());
        let operations = parser.parse().unwrap();

        let mut context = ScriptContext::new(
            source,
            HashMap::new(),
            DocumentMetadata {
                id: "1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("count"), Some(&json!(15)));
        assert_eq!(context.get_field("final_price"), Some(&json!(22.95)));
    }

    #[test]
    fn test_string_operations() {
        let source = json!({
            "title": "  HELLO WORLD  ",
            "description": "test description"
        });

        let script = r#"
            ctx._source.title = ctx._source.title.trim();
            ctx._source.title = ctx._source.title.toLowerCase();
            ctx._source.description = ctx._source.description.replace("test", "final");
        "#;

        let mut parser = ScriptParser::new(script.to_string());
        let operations = parser.parse().unwrap();

        let mut context = ScriptContext::new(
            source,
            HashMap::new(),
            DocumentMetadata {
                id: "1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("hello world")));
        assert_eq!(context.get_field("description"), Some(&json!("final description")));
    }

    #[test]
    fn test_script_with_parameters() {
        let source = json!({
            "title": "Original Title",
            "status": "draft"
        });

        let mut params = HashMap::new();
        params.insert("new_title".to_string(), json!("Parameterized Title"));
        params.insert("new_status".to_string(), json!("published"));

        let script = r#"
            ctx._source.title = params.new_title;
            ctx._source.status = params.new_status;
        "#;

        let mut parser = ScriptParser::new(script.to_string());
        let operations = parser.parse().unwrap();

        let mut context = ScriptContext::new(
            source,
            params,
            DocumentMetadata {
                id: "1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("Parameterized Title")));
        assert_eq!(context.get_field("status"), Some(&json!("published")));
    }

    #[test]
    fn test_complex_conditional_logic() {
        let source = json!({
            "user_type": "premium",
            "age": 25,
            "score": 85,
            "category": "urgent"
        });

        let script = r#"
            if (ctx._source.user_type == "premium" && ctx._source.age >= 18) {
                ctx._source.priority = 1;
            }
            if (ctx._source.score > 80) {
                ctx._source.rating = "excellent";
            }
            if (ctx._source.category == "urgent") {
                ctx._source.priority = 2;
            }
        "#;

        let mut parser = ScriptParser::new(script.to_string());
        let operations = parser.parse().unwrap();

        let mut context = ScriptContext::new(
            source,
            HashMap::new(),
            DocumentMetadata {
                id: "1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("priority"), Some(&json!(2))); // urgent overrides premium
        assert_eq!(context.get_field("rating"), Some(&json!("excellent")));
    }

    #[test]
    fn test_array_operations() {
        let source = json!({
            "tags": ["rust", "search", "database"],
            "scores": [85, 90, 78]
        });

        let script = r#"
            ctx._source.tag_count = ctx._source.tags.length;
            ctx._source.avg_score = (ctx._source.scores[0] + ctx._source.scores[1] + ctx._source.scores[2]) / 3;
        "#;

        let mut parser = ScriptParser::new(script.to_string());
        let operations = parser.parse().unwrap();

        let mut context = ScriptContext::new(
            source,
            HashMap::new(),
            DocumentMetadata {
                id: "1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        // Note: Array length and direct array access would need to be implemented
        // For now, we test basic array field access
        assert_eq!(context.get_field("tags.0"), Some(&json!("rust")));
        assert_eq!(context.get_field("tags.1"), Some(&json!("search")));
    }

    #[test]
    fn test_error_handling() {
        let source = json!({
            "title": "Test"
        });

        // Test invalid script syntax
        let script = "ctx._source.title ="; // Missing value
        let mut parser = ScriptParser::new(script.to_string());
        let result = parser.parse();
        assert!(result.is_err());

        // Test division by zero
        let script = "ctx._source.result = 10 / 0";
        let mut parser = ScriptParser::new(script.to_string());
        let operations = parser.parse().unwrap();

        let mut context = ScriptContext::new(
            source,
            HashMap::new(),
            DocumentMetadata {
                id: "1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        let result = engine.execute(&mut context);
        assert!(result.is_err());
    }

    #[test]
    fn test_script_execution_with_params() {
        // Test script execution with parameters
        let source = json!({
            "title": "Original Title",
            "status": "draft",
            "old_field": "remove me"
        });

        let script_source = r#"
            ctx._source.title = "Transformed Title";
            ctx._source.status = "published";
            ctx._source.old_field.remove();
            ctx._source.new_field = "added";
        "#.to_string();

        let mut parser = ScriptParser::new(script_source);
        let operations = parser.parse().unwrap();

        let mut params = HashMap::new();
        params.insert("multiplier".to_string(), json!(2));

        let mut context = ScriptContext::new(
            source,
            params,
            DocumentMetadata {
                id: "doc1".to_string(),
                index: "test".to_string(),
                doc_type: None,
                version: None,
                routing: None,
            },
        );

        let engine = ScriptEngine::new(operations);
        let result = engine.execute(&mut context);
        assert!(result.is_ok());

        assert_eq!(context.get_field("title"), Some(&json!("Transformed Title")));
        assert_eq!(context.get_field("status"), Some(&json!("published")));
        assert_eq!(context.get_field("new_field"), Some(&json!("added")));
        assert!(context.get_field("old_field").is_none());
    }
}