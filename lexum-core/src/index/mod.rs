//! Index management module
//!
//! Provides functionality for creating, managing, and deleting search indices.

pub mod alias;
pub mod manager;
pub mod rollover;
pub mod settings;
pub mod template;
pub mod template_manager;

pub use alias::{
    AliasAction, AliasConfig, AliasManager, AliasName, AliasOperationsRequest,
    AliasOperationsResponse, IndexAlias,
};
pub use manager::{Index, IndexManager, IndexState, IndexStats};
pub use rollover::{
    RolloverConditions, RolloverConfig, RolloverInfo, RolloverResult, generate_next_index_name,
    should_rollover,
};
pub use settings::{IndexSettings, IndexSettingsUpdate, StorageSettings};
pub use template::{IndexPattern, IndexTemplate, TemplateMappings, TemplateName, TemplateSettings};
pub use template_manager::TemplateManager;
