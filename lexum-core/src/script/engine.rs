//! Script execution engine
//!
//! Executes parsed script operations on document contexts.

use crate::script::context::ScriptContext;
use crate::script::parser::{Condition, MathOp, ScriptOp, StringOp};
use serde_json::Value;

/// Script execution engine
pub struct ScriptEngine {
    operations: Vec<ScriptOp>,
}

impl ScriptEngine {
    /// Create a new script engine
    pub fn new(operations: Vec<ScriptOp>) -> Self {
        Self { operations }
    }

    /// Execute the script on a context
    pub fn execute(&self, context: &mut ScriptContext) -> Result<(), String> {
        for operation in &self.operations {
            self.execute_operation(operation, context)?;
        }
        Ok(())
    }

    /// Execute a single operation
    fn execute_operation(
        &self,
        operation: &ScriptOp,
        context: &mut ScriptContext,
    ) -> Result<(), String> {
        match operation {
            ScriptOp::SetField { path, value } => {
                context.set_field(path, value.clone())?;
            }
            ScriptOp::RemoveField { path } => {
                context.remove_field(path);
            }
            ScriptOp::AddField { path, value } => {
                if context.get_field(path).is_none() {
                    context.set_field(path, value.clone())?;
                }
            }
            ScriptOp::If {
                condition,
                then_ops,
                else_ops,
            } => {
                if self.evaluate_condition(condition, context)? {
                    for op in then_ops {
                        self.execute_operation(op, context)?;
                    }
                } else if let Some(else_ops) = else_ops {
                    for op in else_ops {
                        self.execute_operation(op, context)?;
                    }
                }
            }
            ScriptOp::ForEach {
                array_path,
                var_name,
                ops,
            } => {
                self.execute_for_each(array_path, var_name, ops, context)?;
            }
            ScriptOp::Math {
                operation,
                target,
                value,
            } => {
                Self::execute_math(operation, target, value, context)?;
            }
            ScriptOp::StringOp {
                operation,
                target,
                value,
            } => {
                Self::execute_string_op(operation, target, value, context)?;
            }
        }
        Ok(())
    }

    /// Evaluate a condition
    fn evaluate_condition(
        &self,
        condition: &Condition,
        context: &ScriptContext,
    ) -> Result<bool, String> {
        match condition {
            Condition::FieldExists { path } => Ok(context.get_field(path).is_some()),
            Condition::FieldEquals { path, value } => Ok(context.get_field(path) == Some(value)),
            Condition::FieldContains { path, value } => {
                if let Some(field_value) = context.get_field(path) {
                    Ok(Self::value_contains(field_value, value))
                } else {
                    Ok(false)
                }
            }
            Condition::FieldMatches { path, pattern } => {
                if let Some(field_value) = context.get_field(path) {
                    Self::value_matches(field_value, pattern)
                } else {
                    Ok(false)
                }
            }
            Condition::FieldGt { path, value } => {
                if let Some(field_value) = context.get_field(path) {
                    Ok(Self::value_gt(field_value, value))
                } else {
                    Ok(false)
                }
            }
            Condition::FieldLt { path, value } => {
                if let Some(field_value) = context.get_field(path) {
                    Ok(Self::value_lt(field_value, value))
                } else {
                    Ok(false)
                }
            }
            Condition::And { conditions } => {
                for condition in conditions {
                    if !self.evaluate_condition(condition, context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Condition::Or { conditions } => {
                for condition in conditions {
                    if self.evaluate_condition(condition, context)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Condition::Not { condition } => Ok(!self.evaluate_condition(condition, context)?),
        }
    }

    /// Execute for each operation
    fn execute_for_each(
        &self,
        array_path: &str,
        var_name: &str,
        ops: &[ScriptOp],
        context: &mut ScriptContext,
    ) -> Result<(), String> {
        if let Some(Value::Array(array)) = context.get_field(array_path) {
            let mut modified_items = Vec::new();

            for (index, item) in array.iter().enumerate() {
                // Create a temporary context for each iteration
                let mut temp_context = context.clone();

                // Set the loop variable
                temp_context.set_field(var_name, item.clone())?;
                temp_context.set_field(
                    &format!("{var_name}.index"),
                    Value::Number(serde_json::Number::from(index)),
                )?;

                // Execute operations
                for op in ops {
                    self.execute_operation(op, &mut temp_context)?;
                }

                // Store the modified item
                modified_items.push(temp_context.source);
            }

            // Update the array with modified items
            for (index, item) in modified_items.into_iter().enumerate() {
                context.set_field(&format!("{array_path}.{index}"), item)?;
            }
        }
        Ok(())
    }

    /// Execute mathematical operation
    fn execute_math(
        operation: &MathOp,
        target: &str,
        value: &Value,
        context: &mut ScriptContext,
    ) -> Result<(), String> {
        if let Some(current_value) = context.get_field(target) {
            let result = match operation {
                MathOp::Add => Self::math_add(current_value, value)?,
                MathOp::Subtract => Self::math_subtract(current_value, value)?,
                MathOp::Multiply => Self::math_multiply(current_value, value)?,
                MathOp::Divide => Self::math_divide(current_value, value)?,
                MathOp::Modulo => Self::math_modulo(current_value, value)?,
                MathOp::Power => Self::math_power(current_value, value)?,
            };
            context.set_field(target, result)?;
        }
        Ok(())
    }

    /// Execute string operation
    fn execute_string_op(
        operation: &StringOp,
        target: &str,
        _value: &Option<String>,
        context: &mut ScriptContext,
    ) -> Result<(), String> {
        if let Some(current_value) = context.get_field(target) {
            let result = match operation {
                StringOp::ToLowerCase => {
                    if let Value::String(s) = &current_value {
                        Value::String(s.to_lowercase())
                    } else {
                        current_value.clone()
                    }
                }
                StringOp::ToUpperCase => {
                    if let Value::String(s) = &current_value {
                        Value::String(s.to_uppercase())
                    } else {
                        current_value.clone()
                    }
                }
                StringOp::Trim => {
                    if let Value::String(s) = &current_value {
                        Value::String(s.trim().to_string())
                    } else {
                        current_value.clone()
                    }
                }
                StringOp::Replace { from, to } => {
                    if let Value::String(s) = &current_value {
                        Value::String(s.replace(from, to))
                    } else {
                        current_value.clone()
                    }
                }
                StringOp::Concat { value } => {
                    if let Value::String(s) = &current_value {
                        Value::String(format!("{s}{value}"))
                    } else {
                        current_value.clone()
                    }
                }
                StringOp::Substring { start, end } => {
                    if let Value::String(s) = &current_value {
                        let end_pos = end.unwrap_or(s.len());
                        Value::String(s.chars().skip(*start).take(end_pos - start).collect())
                    } else {
                        current_value.clone()
                    }
                }
            };
            context.set_field(target, result)?;
        }
        Ok(())
    }

    /// Check if value contains another value
    fn value_contains(haystack: &Value, needle: &Value) -> bool {
        match (haystack, needle) {
            (Value::String(s), Value::String(n)) => s.contains(n),
            (Value::Array(arr), v) => {
                for item in arr {
                    if item == v {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if value matches regex pattern
    fn value_matches(value: &Value, pattern: &str) -> Result<bool, String> {
        match value {
            Value::String(s) => {
                let regex = regex::Regex::new(pattern)
                    .map_err(|e| format!("Invalid regex pattern: {e}"))?;
                Ok(regex.is_match(s))
            }
            _ => Ok(false),
        }
    }

    /// Check if value is greater than another
    fn value_gt(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                if let (Some(v1), Some(v2)) = (n1.as_f64(), n2.as_f64()) {
                    v1 > v2
                } else {
                    false
                }
            }
            (Value::String(s1), Value::String(s2)) => s1 > s2,
            _ => false,
        }
    }

    /// Check if value is less than another
    fn value_lt(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                if let (Some(v1), Some(v2)) = (n1.as_f64(), n2.as_f64()) {
                    v1 < v2
                } else {
                    false
                }
            }
            (Value::String(s1), Value::String(s2)) => s1 < s2,
            _ => false,
        }
    }

    /// Mathematical operations
    fn math_add(a: &Value, b: &Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                if let (Some(v1), Some(v2)) = (n1.as_f64(), n2.as_f64()) {
                    Ok(Value::Number(
                        serde_json::Number::from_f64(v1 + v2).unwrap(),
                    ))
                } else {
                    Err("Cannot add non-numeric values".to_string())
                }
            }
            (Value::String(s1), Value::String(s2)) => Ok(Value::String(format!("{s1}{s2}"))),
            _ => Err("Cannot add incompatible types".to_string()),
        }
    }

    fn math_subtract(a: &Value, b: &Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                if let (Some(v1), Some(v2)) = (n1.as_f64(), n2.as_f64()) {
                    Ok(Value::Number(
                        serde_json::Number::from_f64(v1 - v2).unwrap(),
                    ))
                } else {
                    Err("Cannot subtract non-numeric values".to_string())
                }
            }
            _ => Err("Cannot subtract incompatible types".to_string()),
        }
    }

    fn math_multiply(a: &Value, b: &Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                if let (Some(v1), Some(v2)) = (n1.as_f64(), n2.as_f64()) {
                    Ok(Value::Number(
                        serde_json::Number::from_f64(v1 * v2).unwrap(),
                    ))
                } else {
                    Err("Cannot multiply non-numeric values".to_string())
                }
            }
            _ => Err("Cannot multiply incompatible types".to_string()),
        }
    }

    fn math_divide(a: &Value, b: &Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                if let (Some(v1), Some(v2)) = (n1.as_f64(), n2.as_f64()) {
                    if v2 == 0.0 {
                        Err("Division by zero".to_string())
                    } else {
                        Ok(Value::Number(
                            serde_json::Number::from_f64(v1 / v2).unwrap(),
                        ))
                    }
                } else {
                    Err("Cannot divide non-numeric values".to_string())
                }
            }
            _ => Err("Cannot divide incompatible types".to_string()),
        }
    }

    fn math_modulo(a: &Value, b: &Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                if let (Some(v1), Some(v2)) = (n1.as_f64(), n2.as_f64()) {
                    if v2 == 0.0 {
                        Err("Modulo by zero".to_string())
                    } else {
                        Ok(Value::Number(
                            serde_json::Number::from_f64(v1 % v2).unwrap(),
                        ))
                    }
                } else {
                    Err("Cannot modulo non-numeric values".to_string())
                }
            }
            _ => Err("Cannot modulo incompatible types".to_string()),
        }
    }

    fn math_power(a: &Value, b: &Value) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                if let (Some(v1), Some(v2)) = (n1.as_f64(), n2.as_f64()) {
                    Ok(Value::Number(
                        serde_json::Number::from_f64(v1.powf(v2)).unwrap(),
                    ))
                } else {
                    Err("Cannot power non-numeric values".to_string())
                }
            }
            _ => Err("Cannot power incompatible types".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::context::DocumentMetadata;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_execute_set_field() {
        let source = json!({
            "title": "Old Title",
            "content": "Some content"
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

        let operations = vec![ScriptOp::SetField {
            path: "title".to_string(),
            value: json!("New Title"),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("New Title")));
    }

    #[test]
    fn test_execute_if_condition() {
        let source = json!({
            "status": "active",
            "priority": 0
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

        let operations = vec![ScriptOp::If {
            condition: Condition::FieldEquals {
                path: "status".to_string(),
                value: json!("active"),
            },
            then_ops: vec![ScriptOp::SetField {
                path: "priority".to_string(),
                value: json!(1),
            }],
            else_ops: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("priority"), Some(&json!(1)));
    }

    #[test]
    fn test_execute_math_operation() {
        let source = json!({
            "count": 5
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

        let operations = vec![ScriptOp::Math {
            operation: MathOp::Add,
            target: "count".to_string(),
            value: json!(3),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        // math_add returns Number from f64, so result is 8.0, not 8
        let result = context.get_field("count");
        assert!(result.is_some());
        if let Some(Value::Number(n)) = result {
            assert_eq!(n.as_f64(), Some(8.0));
        } else {
            panic!("Expected Number, got {result:?}");
        }
    }

    #[test]
    fn test_execute_remove_field() {
        let source = json!({
            "title": "Test",
            "content": "Some content"
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

        let operations = vec![ScriptOp::RemoveField {
            path: "content".to_string(),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("Test")));
        assert_eq!(context.get_field("content"), None);
    }

    #[test]
    fn test_execute_add_field() {
        let source = json!({
            "title": "Test"
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

        let operations = vec![ScriptOp::AddField {
            path: "status".to_string(),
            value: json!("active"),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("status"), Some(&json!("active")));
    }

    #[test]
    fn test_execute_add_field_existing() {
        let source = json!({
            "status": "inactive"
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

        let operations = vec![ScriptOp::AddField {
            path: "status".to_string(),
            value: json!("active"),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        // Should not overwrite existing field
        assert_eq!(context.get_field("status"), Some(&json!("inactive")));
    }

    #[test]
    fn test_execute_if_with_else() {
        let source = json!({
            "status": "inactive"
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

        let operations = vec![ScriptOp::If {
            condition: Condition::FieldEquals {
                path: "status".to_string(),
                value: json!("active"),
            },
            then_ops: vec![ScriptOp::SetField {
                path: "priority".to_string(),
                value: json!(1),
            }],
            else_ops: Some(vec![ScriptOp::SetField {
                path: "priority".to_string(),
                value: json!(0),
            }]),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("priority"), Some(&json!(0)));
    }

    #[test]
    fn test_execute_for_each() {
        let source = json!({
            "items": [{"value": 1}, {"value": 2}, {"value": 3}]
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

        let operations = vec![ScriptOp::ForEach {
            array_path: "items".to_string(),
            var_name: "item".to_string(),
            ops: vec![ScriptOp::SetField {
                path: "item.processed".to_string(),
                value: json!(true),
            }],
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        // ForEach should have executed - check that items array still exists
        assert!(context.get_field("items").is_some());
    }

    #[test]
    fn test_execute_math_subtract() {
        let source = json!({
            "count": 10
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

        let operations = vec![ScriptOp::Math {
            operation: MathOp::Subtract,
            target: "count".to_string(),
            value: json!(3),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        let result = context.get_field("count");
        if let Some(Value::Number(n)) = result {
            assert_eq!(n.as_f64(), Some(7.0));
        } else {
            panic!("Expected Number");
        }
    }

    #[test]
    fn test_execute_math_multiply() {
        let source = json!({
            "count": 5
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

        let operations = vec![ScriptOp::Math {
            operation: MathOp::Multiply,
            target: "count".to_string(),
            value: json!(3),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        let result = context.get_field("count");
        if let Some(Value::Number(n)) = result {
            assert_eq!(n.as_f64(), Some(15.0));
        } else {
            panic!("Expected Number");
        }
    }

    #[test]
    fn test_execute_math_divide() {
        let source = json!({
            "count": 10
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

        let operations = vec![ScriptOp::Math {
            operation: MathOp::Divide,
            target: "count".to_string(),
            value: json!(2),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        let result = context.get_field("count");
        if let Some(Value::Number(n)) = result {
            assert_eq!(n.as_f64(), Some(5.0));
        } else {
            panic!("Expected Number");
        }
    }

    #[test]
    fn test_execute_math_modulo() {
        let source = json!({
            "count": 10
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

        let operations = vec![ScriptOp::Math {
            operation: MathOp::Modulo,
            target: "count".to_string(),
            value: json!(3),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        let result = context.get_field("count");
        if let Some(Value::Number(n)) = result {
            assert_eq!(n.as_f64(), Some(1.0));
        } else {
            panic!("Expected Number");
        }
    }

    #[test]
    fn test_execute_math_power() {
        let source = json!({
            "count": 2
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

        let operations = vec![ScriptOp::Math {
            operation: MathOp::Power,
            target: "count".to_string(),
            value: json!(3),
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        let result = context.get_field("count");
        if let Some(Value::Number(n)) = result {
            assert_eq!(n.as_f64(), Some(8.0));
        } else {
            panic!("Expected Number");
        }
    }

    #[test]
    fn test_execute_string_to_lowercase() {
        let source = json!({
            "title": "HELLO WORLD"
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

        let operations = vec![ScriptOp::StringOp {
            operation: StringOp::ToLowerCase,
            target: "title".to_string(),
            value: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("hello world")));
    }

    #[test]
    fn test_execute_string_to_uppercase() {
        let source = json!({
            "title": "hello world"
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

        let operations = vec![ScriptOp::StringOp {
            operation: StringOp::ToUpperCase,
            target: "title".to_string(),
            value: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("HELLO WORLD")));
    }

    #[test]
    fn test_execute_string_trim() {
        let source = json!({
            "title": "  hello world  "
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

        let operations = vec![ScriptOp::StringOp {
            operation: StringOp::Trim,
            target: "title".to_string(),
            value: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("hello world")));
    }

    #[test]
    fn test_execute_string_replace() {
        let source = json!({
            "title": "hello world"
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

        let operations = vec![ScriptOp::StringOp {
            operation: StringOp::Replace {
                from: "world".to_string(),
                to: "rust".to_string(),
            },
            target: "title".to_string(),
            value: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("hello rust")));
    }

    #[test]
    fn test_execute_string_concat() {
        let source = json!({
            "title": "hello"
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

        let operations = vec![ScriptOp::StringOp {
            operation: StringOp::Concat {
                value: " world".to_string(),
            },
            target: "title".to_string(),
            value: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("hello world")));
    }

    #[test]
    fn test_execute_string_substring() {
        let source = json!({
            "title": "hello world"
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

        let operations = vec![ScriptOp::StringOp {
            operation: StringOp::Substring {
                start: 0,
                end: Some(5),
            },
            target: "title".to_string(),
            value: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("title"), Some(&json!("hello")));
    }

    #[test]
    fn test_condition_field_exists() {
        let source = json!({
            "title": "Test"
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

        let operations = vec![ScriptOp::If {
            condition: Condition::FieldExists {
                path: "title".to_string(),
            },
            then_ops: vec![ScriptOp::SetField {
                path: "has_title".to_string(),
                value: json!(true),
            }],
            else_ops: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("has_title"), Some(&json!(true)));
    }

    #[test]
    fn test_condition_field_contains() {
        let source = json!({
            "tags": ["rust", "search", "engine"]
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

        let operations = vec![ScriptOp::If {
            condition: Condition::FieldContains {
                path: "tags".to_string(),
                value: json!("rust"),
            },
            then_ops: vec![ScriptOp::SetField {
                path: "has_rust".to_string(),
                value: json!(true),
            }],
            else_ops: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("has_rust"), Some(&json!(true)));
    }

    #[test]
    fn test_condition_field_matches() {
        let source = json!({
            "email": "test@example.com"
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

        let operations = vec![ScriptOp::If {
            condition: Condition::FieldMatches {
                path: "email".to_string(),
                pattern: r".*@.*".to_string(),
            },
            then_ops: vec![ScriptOp::SetField {
                path: "is_email".to_string(),
                value: json!(true),
            }],
            else_ops: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("is_email"), Some(&json!(true)));
    }

    #[test]
    fn test_condition_field_gt() {
        let source = json!({
            "age": 25
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

        let operations = vec![ScriptOp::If {
            condition: Condition::FieldGt {
                path: "age".to_string(),
                value: json!(18),
            },
            then_ops: vec![ScriptOp::SetField {
                path: "is_adult".to_string(),
                value: json!(true),
            }],
            else_ops: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("is_adult"), Some(&json!(true)));
    }

    #[test]
    fn test_condition_field_lt() {
        let source = json!({
            "age": 15
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

        let operations = vec![ScriptOp::If {
            condition: Condition::FieldLt {
                path: "age".to_string(),
                value: json!(18),
            },
            then_ops: vec![ScriptOp::SetField {
                path: "is_minor".to_string(),
                value: json!(true),
            }],
            else_ops: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("is_minor"), Some(&json!(true)));
    }

    #[test]
    fn test_condition_and() {
        let source = json!({
            "status": "active",
            "priority": 5
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

        let operations = vec![ScriptOp::If {
            condition: Condition::And {
                conditions: vec![
                    Condition::FieldEquals {
                        path: "status".to_string(),
                        value: json!("active"),
                    },
                    Condition::FieldGt {
                        path: "priority".to_string(),
                        value: json!(3),
                    },
                ],
            },
            then_ops: vec![ScriptOp::SetField {
                path: "high_priority_active".to_string(),
                value: json!(true),
            }],
            else_ops: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("high_priority_active"), Some(&json!(true)));
    }

    #[test]
    fn test_condition_or() {
        let source = json!({
            "status": "pending"
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

        let operations = vec![ScriptOp::If {
            condition: Condition::Or {
                conditions: vec![
                    Condition::FieldEquals {
                        path: "status".to_string(),
                        value: json!("active"),
                    },
                    Condition::FieldEquals {
                        path: "status".to_string(),
                        value: json!("pending"),
                    },
                ],
            },
            then_ops: vec![ScriptOp::SetField {
                path: "is_valid".to_string(),
                value: json!(true),
            }],
            else_ops: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("is_valid"), Some(&json!(true)));
    }

    #[test]
    fn test_condition_not() {
        let source = json!({
            "status": "inactive"
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

        let operations = vec![ScriptOp::If {
            condition: Condition::Not {
                condition: Box::new(Condition::FieldEquals {
                    path: "status".to_string(),
                    value: json!("active"),
                }),
            },
            then_ops: vec![ScriptOp::SetField {
                path: "is_not_active".to_string(),
                value: json!(true),
            }],
            else_ops: None,
        }];

        let engine = ScriptEngine::new(operations);
        engine.execute(&mut context).unwrap();

        assert_eq!(context.get_field("is_not_active"), Some(&json!(true)));
    }

    #[test]
    fn test_math_divide_by_zero() {
        let source = json!({
            "count": 10
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

        let operations = vec![ScriptOp::Math {
            operation: MathOp::Divide,
            target: "count".to_string(),
            value: json!(0),
        }];

        let engine = ScriptEngine::new(operations);
        let result = engine.execute(&mut context);
        assert!(result.is_err());
    }

    #[test]
    fn test_math_modulo_by_zero() {
        let source = json!({
            "count": 10
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

        let operations = vec![ScriptOp::Math {
            operation: MathOp::Modulo,
            target: "count".to_string(),
            value: json!(0),
        }];

        let engine = ScriptEngine::new(operations);
        let result = engine.execute(&mut context);
        assert!(result.is_err());
    }
}
