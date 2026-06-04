//! Search use cases
//!
//! Implements [`SearchUseCases`] trait for search and DSL query operations.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::entities::Block;
use quilt_domain::repositories::BlockRepository;
use quilt_query::{QueryExecutor, QueryParser};
use quilt_search::{
    SearchError as QuiltSearchError, SearchResult as QuiltSearchResult, SearchServiceTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

/// A no-op search service used when no real search service is configured.
struct NoopSearchService;

#[async_trait]
impl SearchServiceTrait for NoopSearchService {
    async fn search(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<quilt_search::SearchResult>, QuiltSearchError> {
        Err(QuiltSearchError::Sanitization(
            "Search service not configured".to_string(),
        ))
    }

    async fn fuzzy_search(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<quilt_search::SearchResult>, QuiltSearchError> {
        Err(QuiltSearchError::Sanitization(
            "Search service not configured".to_string(),
        ))
    }
}

/// Search use cases trait - search and DSL query operations.
///
/// This trait is object-safe (`Send + Sync`) and uses `#[async_trait]`
/// for async ergonomics.
#[async_trait]
pub trait SearchUseCases: Send + Sync {
    /// Execute a full-text search.
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, ApplicationError>;

    /// Parse a DSL query string and generate SQL with parameters.
    async fn query(&self, dsl: &str, limit: usize) -> Result<QueryPlan, ApplicationError>;
}

/// Query plan returned by [`SearchUseCases::query`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    /// The parsed Abstract Syntax Tree representation
    pub ast: String,
    /// The generated SQL statement with `?` placeholders
    pub sql: String,
    /// Parameter values to be bound to the SQL statement
    pub params: Vec<String>,
    /// The resulting blocks from executing the query (if block_repo is configured)
    pub blocks: Option<Vec<Block>>,
}

/// Search result with ranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Block ID
    pub block_id: String,
    /// Page name
    pub page_name: String,
    /// Snippet of matching content
    pub snippet: String,
    /// Relevance score
    pub score: f64,
}

impl From<QuiltSearchResult> for SearchResult {
    fn from(r: QuiltSearchResult) -> Self {
        Self {
            block_id: r.block_id,
            page_name: r.page_name,
            snippet: r.snippet,
            score: r.score,
        }
    }
}

/// Wrapper around SearchService that implements the SearchUseCases trait.
pub struct SearchUseCasesImpl {
    parser: QueryParser,
    executor: QueryExecutor,
    search_service: Arc<dyn SearchServiceTrait>,
    block_repo: Option<Arc<dyn BlockRepository>>,
}

impl SearchUseCasesImpl {
    /// Create a new SearchUseCasesImpl instance.
    pub fn new() -> Self {
        Self {
            parser: QueryParser,
            executor: QueryExecutor::new(),
            search_service: Arc::new(NoopSearchService),
            block_repo: None,
        }
    }

    /// Create a new SearchUseCasesImpl with a search service.
    pub fn with_search_service(mut self, service: Arc<dyn SearchServiceTrait>) -> Self {
        self.search_service = service;
        self
    }

    /// Create a new SearchUseCasesImpl with a block repository.
    pub fn with_block_repo(mut self, repo: Arc<dyn BlockRepository>) -> Self {
        self.block_repo = Some(repo);
        self
    }
}

impl Default for SearchUseCasesImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchUseCases for SearchUseCasesImpl {
    #[instrument(skip(self))]
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, ApplicationError> {
        let results = self
            .search_service
            .search(query, limit)
            .await
            .map_err(|e| ApplicationError::Validation(format!("Search error: {}", e)))?;
        Ok(results.into_iter().map(SearchResult::from).collect())
    }

    #[instrument(skip(self))]
    async fn query(&self, dsl: &str, limit: usize) -> Result<QueryPlan, ApplicationError> {
        let ast = self
            .parser
            .parse(dsl)
            .map_err(|e| ApplicationError::Validation(format!("Parse error: {}", e)))?;

        let (sql, params) = self
            .executor
            .build_sql(&ast, limit)
            .map_err(|e| ApplicationError::Validation(format!("Build SQL error: {}", e)))?;
        let param_strings: Vec<String> = params.iter().map(|p| p.as_string()).collect();

        let blocks = if let Some(ref repo) = self.block_repo {
            let blocks = repo
                .query_dsl(&sql, &param_strings)
                .await
                .map_err(ApplicationError::Domain)?;
            Some(blocks)
        } else {
            None
        };

        Ok(QueryPlan {
            ast: format!("{:?}", ast),
            sql,
            params: param_strings,
            blocks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_simple_task() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("(task todo)", 100).await.unwrap();

        assert!(result.sql.contains("marker IN"));
        assert!(result.sql.contains("LIMIT 100"));
        assert!(result.ast.contains("Task"));
    }

    #[tokio::test]
    async fn test_query_complex_query() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_
            .query("(and (task todo) (priority a))", 50)
            .await
            .unwrap();

        assert!(result.sql.contains("AND"));
        assert!(result.sql.contains("marker IN"));
        assert!(result.sql.contains("priority IN"));
        assert_eq!(result.params.len(), 2);
    }

    #[tokio::test]
    async fn test_query_page_query() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("(page \"Test Page\")", 100).await.unwrap();

        assert!(result.sql.contains("pages"));
        assert!(result.ast.contains("Page"));
    }

    #[tokio::test]
    async fn test_query_invalid_query() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("", 100).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_between_query() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("(between 1000 2000)", 100).await.unwrap();

        assert!(result.sql.contains("BETWEEN"));
        assert!(result.ast.contains("Between"));
    }

    #[tokio::test]
    async fn test_query_property_query() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_
            .query("(property \"author\" \"John\")", 100)
            .await
            .unwrap();

        assert!(result.sql.contains("json_extract"));
        assert!(result.ast.contains("Property"));
    }

    #[tokio::test]
    async fn test_query_or_query() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_
            .query("(or (task todo) (task done))", 100)
            .await
            .unwrap();

        assert!(result.sql.contains(" OR "));
        assert!(result.ast.contains("Or"));
        assert_eq!(result.params.len(), 2);
    }

    #[tokio::test]
    async fn test_query_not_query() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("(not (task done))", 100).await.unwrap();

        assert!(result.sql.contains("NOT"));
        assert!(result.ast.contains("Not"));
    }

    #[tokio::test]
    async fn test_query_page_ref_query() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("[[Some Page]]", 100).await.unwrap();

        assert!(result.sql.contains("LIKE"));
        assert!(result.ast.contains("PageRef"));
    }

    #[tokio::test]
    async fn test_query_fts_query() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_
            .query("(full-text-search \"keyword\")", 100)
            .await
            .unwrap();

        assert!(result.sql.contains("blocks_fts") || result.sql.contains("MATCH"));
        assert!(result.ast.contains("BlockContent"));
    }

    #[tokio::test]
    async fn test_query_tags_query() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("(tags \"important\")", 100).await.unwrap();

        assert!(result.sql.contains("tags"));
        assert!(result.ast.contains("Tags"));
    }

    #[tokio::test]
    async fn test_query_self_ref() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("self", 100).await.unwrap();

        assert!(result.sql.contains("1 = 1"));
        assert!(result.ast.contains("SelfRef"));
    }

    #[tokio::test]
    async fn test_query_sample() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("(sample 5)", 100).await.unwrap();

        assert!(result.sql.contains("ORDER BY RANDOM()") || result.ast.contains("Sample"));
    }

    #[tokio::test]
    async fn test_query_deeply_nested() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_
            .query(
                "(and (not (task done)) (or (priority a) (priority b)))",
                100,
            )
            .await
            .unwrap();

        assert!(result.sql.contains("NOT"));
        assert!(result.sql.contains(" OR "));
        assert!(result.sql.contains("AND"));
        assert!(result.ast.contains("And"));
        assert!(result.ast.contains("Not"));
        assert!(result.ast.contains("Or"));
    }

    #[tokio::test]
    async fn test_query_invalid_syntax() {
        let impl_ = SearchUseCasesImpl::new();
        // Unclosed parenthesis
        let result = impl_.query("(task", 100).await;
        assert!(result.is_err(), "Should fail on unclosed parenthesis");
    }

    #[tokio::test]
    async fn test_query_multiple_markers() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("(task todo done)", 100).await.unwrap();

        assert!(result.ast.contains("Task"));
        assert!(result.params.len() >= 2);
    }

    #[tokio::test]
    async fn test_query_multiple_priorities() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.query("(priority a b c)", 100).await.unwrap();

        assert!(result.ast.contains("Priority"));
        assert!(result.params.len() >= 3);
    }

    #[tokio::test]
    async fn test_search_without_service_returns_error() {
        let impl_ = SearchUseCasesImpl::new();
        let result = impl_.search("test", 10).await;
        assert!(result.is_err());
    }
}
