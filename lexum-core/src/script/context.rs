//! Script execution context
//!
//! Provides the execution context for script transformations, including
//! access to the source document and transformation parameters.

use serde_json::Value;
use std::collections::HashMap;

/// Script execution context
#[derive(Debug, Clone)]
pub struct ScriptContext {
    /// The source document being transformed
    pub source: Value,
    /// Script parameters
    pub params: HashMap<String, Value>,
    /// Document metadata
    pub metadata: DocumentMetadata,
}

/// Document metadata available in script context
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// Document ID
    pub id: String,
    /// Document index
    pub index: String,
    /// Document type (if applicable)
    pub doc_type: Option<String>,
    /// Document version
    pub version: Option<u64>,
    /// Document routing
    pub routing: Option<String>,
}

impl ScriptContext {
    /// Create a new script context
    pub fn new(source: Value, params: HashMap<String, Value>, metadata: DocumentMetadata) -> Self {
        Self {
            source,
            params,
            metadata,
        }
    }

    /// Get a parameter value
    pub fn get_param(&self, key: &str) -> Option<&Value> {
        self.params.get(key)
    }

    /// Get a field from the source document
    pub fn get_field(&self, path: &str) -> Option<&Value> {
        Self::get_field_by_path(&self.source, path)
    }

    /// Set a field in the source document
    pub fn set_field(&mut self, path: &str, value: Value) -> Result<(), String> {
        let source = &mut self.source;
        Self::set_field_by_path(source, path, value)
    }

    /// Remove a field from the source document
    pub fn remove_field(&mut self, path: &str) -> bool {
        let source = &mut self.source;
        Self::remove_field_by_path(source, path)
    }

    /// Get field by dot-notation path
    fn get_field_by_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                }
                Value::Array(arr) => {
                    let index: usize = part.parse().ok()?;
                    current = arr.get(index)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Set field by dot-notation path
    #[allow(dead_code)]
    fn set_field_by_path(value: &mut Value, path: &str, new_value: Value) -> Result<(), String> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - set the value
                match current {
                    Value::Object(map) => {
                        map.insert(part.to_string(), new_value);
                        return Ok(());
                    }
                    Value::Array(arr) => {
                        let index: usize = part
                            .parse()
                            .map_err(|_| format!("Invalid array index: {part}"))?;
                        if index >= arr.len() {
                            return Err(format!("Array index {index} out of bounds"));
                        }
                        arr[index] = new_value;
                        return Ok(());
                    }
                    _ => {
                        return Err("Cannot set field on non-object/non-array value".to_string());
                    }
                }
            } else {
                // Navigate deeper
                match current {
                    Value::Object(map) => {
                        let part_str = part.to_string();
                        if !map.contains_key(&part_str) {
                            map.insert(part_str.clone(), Value::Object(serde_json::Map::new()));
                        }
                        current = map.get_mut(&part_str).unwrap();
                    }
                    Value::Array(arr) => {
                        let index: usize = part
                            .parse()
                            .map_err(|_| format!("Invalid array index: {part}"))?;
                        if index >= arr.len() {
                            return Err(format!("Array index {index} out of bounds"));
                        }
                        current = &mut arr[index];
                    }
                    _ => {
                        return Err("Cannot navigate into non-object/non-array value".to_string());
                    }
                }
            }
        }

        Ok(())
    }

    /// Remove field by dot-notation path
    #[allow(dead_code)]
    fn remove_field_by_path(value: &mut Value, path: &str) -> bool {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - remove the value
                match current {
                    Value::Object(map) => {
                        return map.remove(part as &str).is_some();
                    }
                    Value::Array(arr) => {
                        if let Ok(index) = part.parse::<usize>() {
                            if index < arr.len() {
                                arr.remove(index);
                                return true;
                            }
                        }
                        return false;
                    }
                    _ => return false,
                }
            } else {
                // Navigate deeper
                match current {
                    Value::Object(map) => {
                        if let Some(next) = map.get_mut(part as &str) {
                            current = next;
                        } else {
                            return false;
                        }
                    }
                    Value::Array(arr) => {
                        if let Ok(index) = part.parse::<usize>() {
                            if let Some(next) = arr.get_mut(index) {
                                current = next;
                            } else {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_field() {
        let source = json!({
            "user": {
                "name": "John",
                "age": 30
            },
            "tags": ["rust", "search"]
        });

        let context = ScriptContext::new(
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

        assert_eq!(context.get_field("user.name"), Some(&json!("John")));
        assert_eq!(context.get_field("user.age"), Some(&json!(30)));
        assert_eq!(context.get_field("tags.0"), Some(&json!("rust")));
        assert_eq!(context.get_field("nonexistent"), None);
    }

    #[test]
    fn test_set_field() {
        let source = json!({
            "user": {
                "name": "John"
            }
        });

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

        context.set_field("user.age", json!(30)).unwrap();
        assert_eq!(context.get_field("user.age"), Some(&json!(30)));

        context
            .set_field("user.address.city", json!("New York"))
            .unwrap();
        assert_eq!(
            context.get_field("user.address.city"),
            Some(&json!("New York"))
        );
    }

    #[test]
    fn test_remove_field() {
        let source = json!({
            "user": {
                "name": "John",
                "age": 30
            },
            "tags": ["rust", "search"]
        });

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

        assert!(context.remove_field("user.age"));
        assert_eq!(context.get_field("user.age"), None);

        assert!(context.remove_field("tags.0"));
        assert_eq!(context.get_field("tags.0"), Some(&json!("search")));
    }
}
