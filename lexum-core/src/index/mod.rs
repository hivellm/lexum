//! Index management module
//!
//! Provides functionality for creating, managing, and deleting search indices.

pub mod alias;
pub mod manager;
pub mod settings;
pub mod template;
pub mod template_manager;

pub use alias::{
    AliasAction, AliasConfig, AliasManager, AliasName, AliasOperationsRequest,
    AliasOperationsResponse, IndexAlias,
};
pub use manager::{Index, IndexManager, IndexStats};
pub use settings::{IndexSettings, StorageSettings};
pub use template::{IndexPattern, IndexTemplate, TemplateMappings, TemplateName, TemplateSettings};
pub use template_manager::TemplateManager;
