//! Scroll API - Efficient pagination for large result sets

use crate::error::Result;
use crate::index::Index;
use crate::query::Query;
use crate::search::executor::SearchExecutor;
use crate::search::result::{SearchHit, SortOption};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use utoipa::ToSchema;
use uuid::Uuid;

/// Scroll context ID
pub type ScrollId = String;

/// Scroll context containing search state
#[derive(Debug, Clone)]
pub struct ScrollContext {
    /// Query to execute
    query: Query,
    /// Sort options
    sort: Option<SortOption>,
    /// Current offset
    offset: usize,
    /// Batch size
    size: usize,
    /// Index name
    #[allow(dead_code)]
    index_name: String,
    /// Created timestamp
    #[allow(dead_code)]
    created_at: Instant,
    /// Last accessed timestamp
    last_accessed: Instant,
    /// Timeout duration
    timeout: Duration,
}

/// Scroll context manager
pub struct ScrollManager {
    /// Active scroll contexts
    contexts: Arc<RwLock<HashMap<ScrollId, ScrollContext>>>,
    /// Default scroll timeout
    default_timeout: Duration,
    /// Cleanup interval
    cleanup_interval: Duration,
}

impl ScrollManager {
    /// Create new scroll manager
    pub fn new(default_timeout: Duration) -> Self {
        let manager = Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            default_timeout,
            cleanup_interval: Duration::from_secs(60),
        };

        // Start cleanup task
        let contexts_clone = manager.contexts.clone();
        let cleanup_interval = manager.cleanup_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                Self::cleanup_expired_contexts(&contexts_clone).await;
            }
        });

        manager
    }

    /// Create scroll context
    pub async fn create_scroll(
        &self,
        query: Query,
        sort: Option<SortOption>,
        size: usize,
        index_name: String,
        timeout: Option<Duration>,
    ) -> ScrollId {
        let scroll_id = format!("scroll_{}", Uuid::new_v4().to_string().replace('-', ""));
        let timeout = timeout.unwrap_or(self.default_timeout);

        let context = ScrollContext {
            query,
            sort,
            offset: 0,
            size,
            index_name,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            timeout,
        };

        self.contexts
            .write()
            .await
            .insert(scroll_id.clone(), context);
        scroll_id
    }

    /// Get next batch from scroll context
    pub async fn scroll(&self, scroll_id: &ScrollId) -> Result<Option<ScrollContext>> {
        let mut contexts = self.contexts.write().await;

        if let Some(mut context) = contexts.remove(scroll_id) {
            // Check if expired
            if context.last_accessed.elapsed() > context.timeout {
                return Ok(None);
            }

            // Update last accessed
            context.last_accessed = Instant::now();
            context.offset += context.size;

            // Re-insert context
            contexts.insert(scroll_id.clone(), context.clone());
            Ok(Some(context))
        } else {
            Ok(None)
        }
    }

    /// Delete scroll context
    pub async fn delete_scroll(&self, scroll_id: &ScrollId) -> bool {
        self.contexts.write().await.remove(scroll_id).is_some()
    }

    /// Clear all scroll contexts
    pub async fn clear_all(&self) {
        self.contexts.write().await.clear();
    }

    /// Cleanup expired contexts
    async fn cleanup_expired_contexts(contexts: &Arc<RwLock<HashMap<ScrollId, ScrollContext>>>) {
        let now = Instant::now();
        let mut contexts = contexts.write().await;
        contexts.retain(|_, context| now.duration_since(context.last_accessed) < context.timeout);
    }
}

/// Scroll request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScrollRequest {
    /// Scroll ID from previous request
    pub scroll_id: ScrollId,
    /// Scroll timeout (e.g., "1m", "30s")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll: Option<String>,
}

/// Scroll response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScrollResponse {
    /// Scroll ID for next request
    #[serde(rename = "_scroll_id")]
    pub scroll_id: ScrollId,
    /// Search results
    pub hits: Vec<SearchHit>,
    /// Total number of hits
    pub total: usize,
    /// Time taken in milliseconds
    pub took_ms: u64,
}

/// Scroll executor
pub struct ScrollExecutor {
    index: Arc<Index>,
    executor: Arc<SearchExecutor>,
    scroll_manager: Arc<ScrollManager>,
}

impl ScrollExecutor {
    /// Create new scroll executor
    pub fn new(index: Arc<Index>, scroll_manager: Arc<ScrollManager>) -> Self {
        let executor = Arc::new(SearchExecutor::new(index.clone()));
        Self {
            index,
            executor,
            scroll_manager,
        }
    }

    /// Create scroll context and return first batch
    pub async fn create_scroll(
        &self,
        query: Query,
        sort: Option<SortOption>,
        size: usize,
        scroll: Option<String>,
    ) -> Result<ScrollResponse> {
        let timeout = scroll
            .as_ref()
            .and_then(|s| parse_duration(s))
            .unwrap_or(Duration::from_secs(60));

        let scroll_id = self
            .scroll_manager
            .create_scroll(
                query.clone(),
                sort.clone(),
                size,
                self.index.name().to_string(),
                Some(timeout),
            )
            .await;

        // Execute first search
        let result = self.executor.search(query, size, 0, sort).await?;

        Ok(ScrollResponse {
            scroll_id,
            hits: result.hits,
            total: result.total,
            took_ms: result.took_ms,
        })
    }

    /// Continue scroll with scroll ID
    pub async fn scroll(&self, scroll_id: ScrollId) -> Result<Option<ScrollResponse>> {
        if let Some(context) = self.scroll_manager.scroll(&scroll_id).await? {
            // Execute search with updated offset
            let result = self
                .executor
                .search(
                    context.query.clone(),
                    context.size,
                    context.offset,
                    context.sort.clone(),
                )
                .await?;

            if result.hits.is_empty() {
                // No more results, delete scroll context
                self.scroll_manager.delete_scroll(&scroll_id).await;
                Ok(None)
            } else {
                Ok(Some(ScrollResponse {
                    scroll_id,
                    hits: result.hits,
                    total: result.total,
                    took_ms: result.took_ms,
                }))
            }
        } else {
            Ok(None)
        }
    }

    /// Delete scroll context
    pub async fn delete_scroll(&self, scroll_id: ScrollId) -> bool {
        self.scroll_manager.delete_scroll(&scroll_id).await
    }
}

/// Parse duration string (e.g., "1m", "30s", "1h")
fn parse_duration(s: &str) -> Option<Duration> {
    if s.is_empty() {
        return None;
    }

    let s = s.trim();
    let (num_str, unit) = if let Some(stripped) = s.strip_suffix('s') {
        (stripped, "s")
    } else if let Some(stripped) = s.strip_suffix('m') {
        (stripped, "m")
    } else if let Some(stripped) = s.strip_suffix('h') {
        (stripped, "h")
    } else if let Some(stripped) = s.strip_suffix('d') {
        (stripped, "d")
    } else {
        return None;
    };

    let num: u64 = num_str.parse().ok()?;

    match unit {
        "s" => Some(Duration::from_secs(num)),
        "m" => Some(Duration::from_secs(num * 60)),
        "h" => Some(Duration::from_secs(num * 3600)),
        "d" => Some(Duration::from_secs(num * 86400)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration("1m"), Some(Duration::from_secs(60)));
        assert_eq!(parse_duration("2h"), Some(Duration::from_secs(7200)));
        assert_eq!(parse_duration("1d"), Some(Duration::from_secs(86400)));
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("invalid"), None);
    }

    #[tokio::test]
    async fn test_scroll_manager() {
        let manager = ScrollManager::new(Duration::from_secs(60));
        let scroll_id = manager
            .create_scroll(
                crate::query::Query::MatchAll,
                None,
                10,
                "test_index".to_string(),
                None,
            )
            .await;

        assert!(!scroll_id.is_empty());
        assert!(manager.delete_scroll(&scroll_id).await);
    }
}
