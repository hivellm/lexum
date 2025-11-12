//! Script parser for Painless-like syntax
//!
//! Parses simple transformation scripts with Painless-like syntax
//! for document transformation operations.

use serde_json::Value;

/// Script operation types
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptOp {
    /// Set a field value
    SetField {
        /// Field path to set
        path: String,
        /// Value to set
        value: Value,
    },
    /// Remove a field
    RemoveField {
        /// Field path to remove
        path: String,
    },
    /// Add a field with a value
    AddField {
        /// Field path to add
        path: String,
        /// Value to add
        value: Value,
    },
    /// Conditional operation
    If {
        /// Condition to evaluate
        condition: Condition,
        /// Operations to execute if condition is true
        then_ops: Vec<ScriptOp>,
        /// Operations to execute if condition is false (optional)
        else_ops: Option<Vec<ScriptOp>>,
    },
    /// For each operation
    ForEach {
        /// Array path to iterate over
        array_path: String,
        /// Variable name for current element
        var_name: String,
        /// Operations to execute for each element
        ops: Vec<ScriptOp>,
    },
    /// Mathematical operations
    Math {
        /// Mathematical operation to perform
        operation: MathOp,
        /// Target field path
        target: String,
        /// Value to use in operation
        value: Value,
    },
    /// String operations
    StringOp {
        /// String operation to perform
        operation: StringOp,
        /// Target field path
        target: String,
        /// Optional value for operation
        value: Option<String>,
    },
}

/// Condition types for conditional operations
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// Field exists
    FieldExists {
        /// Field path to check
        path: String,
    },
    /// Field equals value
    FieldEquals {
        /// Field path to check
        path: String,
        /// Value to compare
        value: Value,
    },
    /// Field contains value
    FieldContains {
        /// Field path to check
        path: String,
        /// Value to check for
        value: Value,
    },
    /// Field matches regex
    FieldMatches {
        /// Field path to check
        path: String,
        /// Regex pattern to match
        pattern: String,
    },
    /// Field is greater than value
    FieldGt {
        /// Field path to check
        path: String,
        /// Value to compare
        value: Value,
    },
    /// Field is less than value
    FieldLt {
        /// Field path to check
        path: String,
        /// Value to compare
        value: Value,
    },
    /// Logical AND
    And {
        /// Conditions to AND together
        conditions: Vec<Condition>,
    },
    /// Logical OR
    Or {
        /// Conditions to OR together
        conditions: Vec<Condition>,
    },
    /// Logical NOT
    Not {
        /// Condition to negate
        condition: Box<Condition>,
    },
}

/// Mathematical operations
#[derive(Debug, Clone, PartialEq)]
pub enum MathOp {
    /// Addition operation
    Add,
    /// Subtraction operation
    Subtract,
    /// Multiplication operation
    Multiply,
    /// Division operation
    Divide,
    /// Modulo operation
    Modulo,
    /// Power operation
    Power,
}

/// String operations
#[derive(Debug, Clone, PartialEq)]
pub enum StringOp {
    /// Convert to lowercase
    ToLowerCase,
    /// Convert to uppercase
    ToUpperCase,
    /// Trim whitespace
    Trim,
    /// Replace string
    Replace {
        /// String to replace
        from: String,
        /// Replacement string
        to: String,
    },
    /// Concatenate string
    Concat {
        /// Value to concatenate
        value: String,
    },
    /// Extract substring
    Substring {
        /// Start position
        start: usize,
        /// End position (optional)
        end: Option<usize>,
    },
}

/// Script parser
pub struct ScriptParser {
    source: String,
    position: usize,
}

impl ScriptParser {
    /// Create a new script parser
    pub fn new(source: String) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    /// Parse the script into operations
    pub fn parse(&mut self) -> Result<Vec<ScriptOp>, String> {
        let mut operations = Vec::new();

        while self.position < self.source.len() {
            self.skip_whitespace();
            if self.position >= self.source.len() {
                break;
            }

            let op = self.parse_operation()?;
            operations.push(op);

            self.skip_whitespace();
            if self.position < self.source.len() && self.peek() == ';' {
                self.advance(); // Skip semicolon
            }
        }

        Ok(operations)
    }

    /// Parse a single operation
    fn parse_operation(&mut self) -> Result<ScriptOp, String> {
        self.skip_whitespace();

        if self.starts_with("if") {
            self.parse_if()
        } else if self.starts_with("for") {
            self.parse_for_each()
        } else if self.starts_with("ctx._source") {
            self.parse_ctx_operation()
        } else if self.starts_with("ctx._source.") {
            self.parse_field_operation()
        } else {
            Err(format!("Unknown operation at position {}", self.position))
        }
    }

    /// Parse if statement
    fn parse_if(&mut self) -> Result<ScriptOp, String> {
        self.expect("if")?;
        self.skip_whitespace();
        self.expect("(")?;

        let condition = self.parse_condition()?;
        self.expect(")")?;
        self.skip_whitespace();
        self.expect("{")?;

        let then_ops = self.parse_operation_block()?;
        self.expect("}")?;

        let else_ops = if self.starts_with("else") {
            self.expect("else")?;
            self.skip_whitespace();
            self.expect("{")?;
            let ops = self.parse_operation_block()?;
            self.expect("}")?;
            Some(ops)
        } else {
            None
        };

        Ok(ScriptOp::If {
            condition,
            then_ops,
            else_ops,
        })
    }

    /// Parse for each statement
    fn parse_for_each(&mut self) -> Result<ScriptOp, String> {
        self.expect("for")?;
        self.skip_whitespace();
        self.expect("(")?;

        let var_name = self.parse_identifier()?;
        self.skip_whitespace();
        self.expect(":")?;
        self.skip_whitespace();

        let array_path = self.parse_field_path()?;
        self.expect(")")?;
        self.skip_whitespace();
        self.expect("{")?;

        let ops = self.parse_operation_block()?;
        self.expect("}")?;

        Ok(ScriptOp::ForEach {
            array_path,
            var_name,
            ops,
        })
    }

    /// Parse ctx._source operations
    fn parse_ctx_operation(&mut self) -> Result<ScriptOp, String> {
        self.expect("ctx._source")?;

        if self.peek() == '[' {
            // Array access
            self.advance(); // Skip '['
            let path = self.parse_quoted_string()?;
            self.expect("]")?;
            self.skip_whitespace();

            if self.peek() == '=' {
                self.advance(); // Skip '='
                self.skip_whitespace();
                let value = self.parse_value()?;
                Ok(ScriptOp::SetField { path, value })
            } else {
                Err("Expected '=' after field access".to_string())
            }
        } else if self.peek() == '.' {
            self.advance(); // Skip '.'
            self.parse_field_operation()
        } else {
            Err("Expected '.' or '[' after ctx._source".to_string())
        }
    }

    /// Parse field operations
    fn parse_field_operation(&mut self) -> Result<ScriptOp, String> {
        let path = self.parse_field_path()?;
        self.skip_whitespace();

        // Skip optional '.' before remove() or add()
        if self.peek() == '.' {
            self.advance();
            self.skip_whitespace();
        }

        if self.peek() == '=' {
            self.advance(); // Skip '='
            self.skip_whitespace();
            let value = self.parse_value()?;
            Ok(ScriptOp::SetField { path, value })
        } else if self.starts_with("remove()") {
            self.expect("remove()")?;
            Ok(ScriptOp::RemoveField { path })
        } else if self.starts_with("add(") {
            self.expect("add(")?;
            let value = self.parse_value()?;
            self.expect(")")?;
            Ok(ScriptOp::AddField { path, value })
        } else {
            Err("Expected '=', 'remove()', or 'add()' after field path".to_string())
        }
    }

    /// Parse condition
    fn parse_condition(&mut self) -> Result<Condition, String> {
        self.skip_whitespace();

        if self.starts_with("ctx._source.") {
            self.expect("ctx._source.")?;
            let path = self.parse_field_path()?;
            self.skip_whitespace();

            if self.starts_with("==") {
                self.expect("==")?;
                self.skip_whitespace();
                let value = self.parse_value()?;
                Ok(Condition::FieldEquals { path, value })
            } else if self.starts_with("!=") {
                self.expect("!=")?;
                self.skip_whitespace();
                let value = self.parse_value()?;
                Ok(Condition::Not {
                    condition: Box::new(Condition::FieldEquals { path, value }),
                })
            } else if self.starts_with(">") {
                self.advance();
                self.skip_whitespace();
                let value = self.parse_value()?;
                Ok(Condition::FieldGt { path, value })
            } else if self.starts_with("<") {
                self.advance();
                self.skip_whitespace();
                let value = self.parse_value()?;
                Ok(Condition::FieldLt { path, value })
            } else if self.starts_with(".contains(") {
                self.expect(".contains(")?;
                let value = self.parse_value()?;
                self.expect(")")?;
                Ok(Condition::FieldContains { path, value })
            } else if self.starts_with(".matches(") {
                self.expect(".matches(")?;
                let pattern = self.parse_quoted_string()?;
                self.expect(")")?;
                Ok(Condition::FieldMatches { path, pattern })
            } else {
                Ok(Condition::FieldExists { path })
            }
        } else if self.starts_with("!") {
            self.advance();
            let condition = self.parse_condition()?;
            Ok(Condition::Not {
                condition: Box::new(condition),
            })
        } else if self.starts_with("(") {
            self.advance();
            let condition = self.parse_condition()?;
            self.expect(")")?;
            Ok(condition)
        } else {
            Err("Invalid condition".to_string())
        }
    }

    /// Parse operation block
    fn parse_operation_block(&mut self) -> Result<Vec<ScriptOp>, String> {
        let mut operations = Vec::new();

        while self.position < self.source.len() {
            self.skip_whitespace();
            if self.peek() == '}' {
                break;
            }

            let op = self.parse_operation()?;
            operations.push(op);

            self.skip_whitespace();
            if self.peek() == ';' {
                self.advance();
            }
        }

        Ok(operations)
    }

    /// Parse field path
    fn parse_field_path(&mut self) -> Result<String, String> {
        let mut path = String::new();

        while self.position < self.source.len() {
            let ch = self.peek();
            if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                path.push(self.advance());
            } else {
                break;
            }
        }

        if path.is_empty() {
            Err("Expected field path".to_string())
        } else {
            Ok(path)
        }
    }

    /// Parse identifier
    fn parse_identifier(&mut self) -> Result<String, String> {
        let mut ident = String::new();

        while self.position < self.source.len() {
            let ch = self.peek();
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(self.advance());
            } else {
                break;
            }
        }

        if ident.is_empty() {
            Err("Expected identifier".to_string())
        } else {
            Ok(ident)
        }
    }

    /// Parse quoted string
    fn parse_quoted_string(&mut self) -> Result<String, String> {
        self.expect("\"")?;
        let mut string = String::new();

        while self.position < self.source.len() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance();
                if self.position < self.source.len() {
                    string.push(self.advance());
                }
            } else {
                string.push(self.advance());
            }
        }

        self.expect("\"")?;
        Ok(string)
    }

    /// Parse value (string, number, boolean, null)
    fn parse_value(&mut self) -> Result<Value, String> {
        self.skip_whitespace();

        if self.starts_with("\"") {
            let string = self.parse_quoted_string()?;
            Ok(Value::String(string))
        } else if self.starts_with("true") {
            self.expect("true")?;
            Ok(Value::Bool(true))
        } else if self.starts_with("false") {
            self.expect("false")?;
            Ok(Value::Bool(false))
        } else if self.starts_with("null") {
            self.expect("null")?;
            Ok(Value::Null)
        } else if self.peek().is_ascii_digit() || self.peek() == '-' {
            self.parse_number()
        } else {
            Err("Expected value".to_string())
        }
    }

    /// Parse number
    fn parse_number(&mut self) -> Result<Value, String> {
        let mut number = String::new();
        let mut is_float = false;

        if self.peek() == '-' {
            number.push(self.advance());
        }

        while self.position < self.source.len() {
            let ch = self.peek();
            if ch.is_ascii_digit() {
                number.push(self.advance());
            } else if ch == '.' && !is_float {
                is_float = true;
                number.push(self.advance());
            } else {
                break;
            }
        }

        if is_float {
            number
                .parse::<f64>()
                .map(|n| Value::Number(serde_json::Number::from_f64(n).unwrap()))
                .map_err(|_| "Invalid float".to_string())
        } else {
            number
                .parse::<i64>()
                .map(|n| Value::Number(serde_json::Number::from(n)))
                .map_err(|_| "Invalid integer".to_string())
        }
    }

    /// Check if current position starts with string
    fn starts_with(&self, s: &str) -> bool {
        self.source[self.position..].starts_with(s)
    }

    /// Expect a specific string
    fn expect(&mut self, s: &str) -> Result<(), String> {
        if self.starts_with(s) {
            self.position += s.len();
            Ok(())
        } else {
            Err(format!("Expected '{s}'"))
        }
    }

    /// Peek at current character
    fn peek(&self) -> char {
        if self.position < self.source.len() {
            self.source.chars().nth(self.position).unwrap()
        } else {
            '\0'
        }
    }

    /// Advance position and return current character
    fn advance(&mut self) -> char {
        let ch = self.peek();
        self.position += 1;
        ch
    }

    /// Skip whitespace
    fn skip_whitespace(&mut self) {
        while self.position < self.source.len() && self.peek().is_whitespace() {
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_set() {
        let source = "ctx._source.title = \"New Title\"".to_string();
        let mut parser = ScriptParser::new(source);
        let ops = parser.parse().unwrap();

        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0],
            ScriptOp::SetField {
                path: "title".to_string(),
                value: Value::String("New Title".to_string())
            }
        );
    }

    #[test]
    fn test_parse_remove_field() {
        let source = "ctx._source.old_field.remove()".to_string();
        let mut parser = ScriptParser::new(source);
        let ops = parser.parse().unwrap();

        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0],
            ScriptOp::RemoveField {
                path: "old_field".to_string()
            }
        );
    }

    #[test]
    fn test_parse_if_statement() {
        let source =
            "if (ctx._source.status == \"active\") { ctx._source.priority = 1 }".to_string();
        let mut parser = ScriptParser::new(source);
        let ops = parser.parse().unwrap();

        assert_eq!(ops.len(), 1);
        match &ops[0] {
            ScriptOp::If {
                condition,
                then_ops,
                else_ops,
            } => {
                assert_eq!(
                    *condition,
                    Condition::FieldEquals {
                        path: "status".to_string(),
                        value: Value::String("active".to_string())
                    }
                );
                assert_eq!(then_ops.len(), 1);
                assert!(else_ops.is_none());
            }
            _ => panic!("Expected If operation"),
        }
    }

    #[test]
    fn test_parse_nested_field() {
        let source = "ctx._source.user.name = \"John\"".to_string();
        let mut parser = ScriptParser::new(source);
        let ops = parser.parse().unwrap();

        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0],
            ScriptOp::SetField {
                path: "user.name".to_string(),
                value: Value::String("John".to_string())
            }
        );
    }
}
