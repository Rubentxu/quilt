//! Query executor — F18: execute a QueryAst against the block store.
//!
//! Takes a `QueryAst` from the F18 pipeline, compiles it to SQL via
//! `quilt_query::QueryExecutor`, and executes it against the `BlockRepository`.
//!
//! This is the Rust side of the F18 pipeline: client builds AST → server executes.

use quilt_domain::entities::Block;
use quilt_domain::repositories::BlockRepository;
use quilt_query::ast::QueryAst;
use quilt_query::executor::QueryExecutor;
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

/// Errors specific to query execution.
#[derive(Debug, Error)]
pub enum QueryExecutorError {
    #[error("Query compilation failed: {0}")]
    Compilation(String),

    #[error("Query execution failed: {0}")]
    Execution(String),
}

/// Query executor service.
///
/// Executes a `QueryAst` against the block store, returning matching blocks.
/// This is the application-level entry point for the F18 query pipeline.
pub struct QueryExecutorService<BR: BlockRepository> {
    block_repo: Arc<BR>,
    dsl_executor: QueryExecutor,
}

impl<BR: BlockRepository> QueryExecutorService<BR> {
    /// Create a new QueryExecutorService.
    pub fn new(block_repo: Arc<BR>) -> Self {
        Self {
            block_repo,
            dsl_executor: QueryExecutor::new(),
        }
    }

    /// Execute a query AST and return matching blocks.
    ///
    /// # Arguments
    /// * `ast` — The parsed query AST
    /// * `limit` — Maximum number of results (server may cap at 1000)
    ///
    /// # Returns
    /// Matching blocks, or an empty vec on success (no panic).
    #[instrument(skip(self))]
    pub async fn execute(
        &self,
        ast: &QueryAst,
        limit: usize,
    ) -> Result<Vec<Block>, QueryExecutorError> {
        // Cap limit at 1000 to match server-side constraint
        let effective_limit = limit.min(1000);

        // Compile AST to SQL
        let (sql, params) = self
            .dsl_executor
            .build_sql(ast, effective_limit)
            .map_err(|e| QueryExecutorError::Compilation(e.to_string()))?;

        // Convert SqlParam to String (query_dsl takes String params)
        let string_params: Vec<String> = params
            .into_iter()
            .map(|p| match p {
                quilt_query::executor::SqlParam::String(s) => s,
                quilt_query::executor::SqlParam::Integer(n) => n.to_string(),
                quilt_query::executor::SqlParam::Float(f) => f.to_string(),
                quilt_query::executor::SqlParam::Boolean(b) => b.to_string(),
            })
            .collect();

        // Execute against the block repository
        self.block_repo
            .query_dsl(&sql, &string_params)
            .await
            .map_err(|e| QueryExecutorError::Execution(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use quilt_domain::entities::{Block, BlockCreate};
    use quilt_domain::errors::DomainError;
    use quilt_domain::value_objects::Uuid;
    use quilt_domain::value_objects::{BlockFormat, BlockType};
    use std::collections::HashMap;

    /// In-memory block repository for testing.
    struct InMemoryBlockRepo {
        blocks: Vec<Block>,
    }

    impl InMemoryBlockRepo {
        fn new(blocks: Vec<Block>) -> Self {
            Self { blocks }
        }
    }

    #[async_trait]
    impl BlockRepository for InMemoryBlockRepo {
        async fn get_by_id(&self, _id: Uuid) -> Result<Option<Block>, DomainError> {
            unimplemented!()
        }
        async fn get_by_page(&self, _page_id: Uuid) -> Result<Vec<Block>, DomainError> {
            unimplemented!()
        }
        async fn get_children(&self, _parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
            unimplemented!()
        }
        async fn get_with_refs(&self, _id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
            unimplemented!()
        }
        async fn insert(&self, _block: &Block) -> Result<(), DomainError> {
            unimplemented!()
        }
        async fn update(&self, _block: &Block) -> Result<(), DomainError> {
            unimplemented!()
        }
        async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
            unimplemented!()
        }
        async fn move_block(
            &self,
            _id: Uuid,
            _new_parent: Option<Uuid>,
            _new_order: f64,
        ) -> Result<(), DomainError> {
            unimplemented!()
        }
        async fn get_backlinks(&self, _block_id: Uuid) -> Result<Vec<Block>, DomainError> {
            unimplemented!()
        }
        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Block>, DomainError> {
            unimplemented!()
        }
        async fn get_updated_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            unimplemented!()
        }
        async fn count_by_page(&self, _page_id: Uuid) -> Result<usize, DomainError> {
            unimplemented!()
        }
        async fn count_all(&self) -> Result<usize, DomainError> {
            Ok(self.blocks.len())
        }
        async fn query_dsl(
            &self,
            sql: &str,
            _params: &[String],
        ) -> Result<Vec<Block>, DomainError> {
            // Return all blocks if SQL contains basic SELECT
            if sql.contains("SELECT") {
                Ok(self.blocks.clone())
            } else {
                Ok(vec![])
            }
        }
        async fn list_by_property(
            &self,
            _key: &str,
            _value: &str,
            _limit: usize,
        ) -> Result<Vec<Block>, DomainError> {
            unimplemented!()
        }

        async fn list_distinct_keys(
            &self,
            _cursor: Option<&str>,
            _limit: u32,
        ) -> Result<Vec<String>, DomainError> {
            // The query executor's test mock doesn't model property
            // keys — the property-keys-endpoint handler never routes
            // through this impl. Return an empty result so the trait
            // is satisfied.
            Ok(Vec::new())
        }

        async fn list_by_property_key(
            &self,
            _key: &str,
            _limit: u32,
        ) -> Result<Vec<Block>, DomainError> {
            // Same rationale as `list_distinct_keys` above — the
            // query executor's mock doesn't model property keys, and
            // the lens handler never routes through this impl.
            Ok(Vec::new())
        }

        async fn list_distinct_authors(
            &self,
            _prefix: Option<&str>,
        ) -> Result<Vec<String>, DomainError> {
            Ok(Vec::new())
        }
    }

    fn make_block(page_id: Uuid, content: &str) -> Block {
        Block::new(BlockCreate {
            page_id,
            content: content.to_string(),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            properties: HashMap::new(),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn execute_empty_ast_returns_empty() {
        let repo = InMemoryBlockRepo::new(vec![]);
        let svc = QueryExecutorService::new(Arc::new(repo));

        let ast = QueryAst::Page("test".to_string());
        let result = svc.execute(&ast, 100).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn execute_task_query_returns_blocks() {
        let block = make_block(Uuid::new_v4(), "content");
        let repo = InMemoryBlockRepo::new(vec![block.clone()]);
        let svc = QueryExecutorService::new(Arc::new(repo));

        let ast = QueryAst::Task(vec!["todo".to_string(), "done".to_string()]);
        let result = svc.execute(&ast, 100).await;
        assert!(result.is_ok());
        // In-memory repo returns all blocks for SELECT queries
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn execute_property_query_compiles() {
        let block = make_block(Uuid::new_v4(), "content");
        let repo = InMemoryBlockRepo::new(vec![block.clone()]);
        let svc = QueryExecutorService::new(Arc::new(repo));

        let ast = QueryAst::Property {
            key: "status".to_string(),
            op: quilt_query::property_op::PropertyOp::Equals,
            value: quilt_query::ast::QueryValue::String("active".to_string()),
            value2: None,
        };
        let result = svc.execute(&ast, 100).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_and_query_compiles() {
        let block = make_block(Uuid::new_v4(), "content");
        let repo = InMemoryBlockRepo::new(vec![block.clone()]);
        let svc = QueryExecutorService::new(Arc::new(repo));

        let ast = QueryAst::And(vec![
            QueryAst::Property {
                key: "status".to_string(),
                op: quilt_query::property_op::PropertyOp::Equals,
                value: quilt_query::ast::QueryValue::String("active".to_string()),
                value2: None,
            },
            QueryAst::Priority(vec!["a".to_string()]),
        ]);
        let result = svc.execute(&ast, 100).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_limit_capped_at_1000() {
        let repo = InMemoryBlockRepo::new(vec![]);
        let svc = QueryExecutorService::new(Arc::new(repo));

        let ast = QueryAst::Task(vec!["todo".to_string()]);
        // Request 5000, should be capped
        let result = svc.execute(&ast, 5000).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_empty_and_returns_empty() {
        let repo = InMemoryBlockRepo::new(vec![]);
        let svc = QueryExecutorService::new(Arc::new(repo));

        let ast = QueryAst::And(vec![]);
        let result = svc.execute(&ast, 100).await;
        assert!(result.is_ok());
    }
}
