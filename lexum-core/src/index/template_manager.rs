//! Template manager for CRUD operations on index templates

use crate::error::Result;
use crate::index::template::IndexTemplate;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Manages index templates
pub struct TemplateManager {
    templates: Arc<RwLock<HashMap<String, IndexTemplate>>>,
}

impl TemplateManager {
    /// Create a new template manager
    pub fn new() -> Self {
        Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create or update a template
    pub fn put_template(&self, template: IndexTemplate) -> Result<()> {
        // Validate template before storing
        template.validate()?;

        let name = template.name.as_str().to_string();
        let mut templates = self.templates.write();
        templates.insert(name, template);
        Ok(())
    }

    /// Get a template by name
    pub fn get_template(&self, name: &str) -> Result<Option<IndexTemplate>> {
        let templates = self.templates.read();
        Ok(templates.get(name).cloned())
    }

    /// Delete a template by name
    pub fn delete_template(&self, name: &str) -> Result<bool> {
        let mut templates = self.templates.write();
        Ok(templates.remove(name).is_some())
    }

    /// List all templates
    pub fn list_templates(&self) -> Vec<IndexTemplate> {
        let templates = self.templates.read();
        templates.values().cloned().collect()
    }

    /// Find templates that match an index name
    pub fn find_matching_templates(&self, index_name: &str) -> Vec<IndexTemplate> {
        let templates = self.templates.read();
        let mut matching: Vec<IndexTemplate> = templates
            .values()
            .filter(|template| template.matches_index(index_name))
            .cloned()
            .collect();

        // Sort by priority (descending) then by order (ascending)
        matching.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then(a.order.cmp(&b.order))
        });

        matching
    }

    /// Get template count
    pub fn template_count(&self) -> usize {
        let templates = self.templates.read();
        templates.len()
    }

    /// Check if template exists
    pub fn has_template(&self, name: &str) -> bool {
        let templates = self.templates.read();
        templates.contains_key(name)
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_manager_creation() {
        let manager = TemplateManager::new();
        assert_eq!(manager.template_count(), 0);
    }

    #[test]
    fn test_put_and_get_template() {
        let manager = TemplateManager::new();
        
        let template = IndexTemplate::new("test")
            .with_pattern("logs-*")
            .with_priority(100);

        // Put template
        assert!(manager.put_template(template.clone()).is_ok());

        // Get template
        let retrieved = manager.get_template("test").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name.as_str(), "test");

        // Check count
        assert_eq!(manager.template_count(), 1);
    }

    #[test]
    fn test_delete_template() {
        let manager = TemplateManager::new();
        
        let template = IndexTemplate::new("test")
            .with_pattern("logs-*");

        // Put template
        manager.put_template(template).unwrap();

        // Delete template
        assert!(manager.delete_template("test").unwrap());
        assert!(!manager.delete_template("nonexistent").unwrap());

        // Check count
        assert_eq!(manager.template_count(), 0);
    }

    #[test]
    fn test_find_matching_templates() {
        let manager = TemplateManager::new();
        
        // Create templates with different priorities
        let high_priority = IndexTemplate::new("high")
            .with_pattern("logs-*")
            .with_priority(200);

        let low_priority = IndexTemplate::new("low")
            .with_pattern("logs-*")
            .with_priority(100);

        let other_pattern = IndexTemplate::new("other")
            .with_pattern("events-*")
            .with_priority(150);

        // Put templates
        manager.put_template(high_priority).unwrap();
        manager.put_template(low_priority).unwrap();
        manager.put_template(other_pattern).unwrap();

        // Find matching templates for logs-2024
        let matching = manager.find_matching_templates("logs-2024");
        assert_eq!(matching.len(), 2);
        assert_eq!(matching[0].name.as_str(), "high"); // Higher priority first
        assert_eq!(matching[1].name.as_str(), "low");

        // Find matching templates for events-2024
        let matching = manager.find_matching_templates("events-2024");
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].name.as_str(), "other");

        // Find matching templates for no-match
        let matching = manager.find_matching_templates("no-match");
        assert_eq!(matching.len(), 0);
    }

    #[test]
    fn test_template_validation() {
        let manager = TemplateManager::new();
        
        // Create invalid template (no patterns)
        let invalid_template = IndexTemplate::new("invalid");

        // Should fail validation
        assert!(manager.put_template(invalid_template).is_err());
    }

    #[test]
    fn test_list_templates() {
        let manager = TemplateManager::new();
        
        let template1 = IndexTemplate::new("template1")
            .with_pattern("logs-*");
        let template2 = IndexTemplate::new("template2")
            .with_pattern("events-*");

        manager.put_template(template1).unwrap();
        manager.put_template(template2).unwrap();

        let templates = manager.list_templates();
        assert_eq!(templates.len(), 2);
    }

    #[test]
    fn test_has_template() {
        let manager = TemplateManager::new();
        
        let template = IndexTemplate::new("test")
            .with_pattern("logs-*");

        assert!(!manager.has_template("test"));
        
        manager.put_template(template).unwrap();
        
        assert!(manager.has_template("test"));
        assert!(!manager.has_template("nonexistent"));
    }
}
