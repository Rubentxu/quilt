# Quilt — Rust AI-First Knowledge Graph

> Análisis profundo del ecosistema Rust + MCP + AI
> Fecha: 2026-05-02 | Nivel: detalhado

---

## 0. Principios de Diseño

```
TODO en Rust es un MCP Resource.
Toda operación es un MCP Tool.
Toda acción notifica via MCP Notification.
Humanos y AI Agents usan la MISMA API.
```

| Principio | Implicación |
|-----------|-------------|
| **MCP-first** | La API pública es MCP. La UI consume la misma API. |
| **Rust safety** | Zero panics en runtime. OwnerShip previene data races. |
| **WASM target** | Compila a WASM para browser sin cambios estructurales. |
| **Agent-native** | Los AI agents son first-class citizens, no afterthought. |
| **Observability** | Tracing + métricas desde el día 0. |

---

## 1. Data Layer — SQLite + Rkyv

### 1.1 Schema Rust (tipos seguros)

```rust
// src/core/model.rs

use rkyv::{Archive, Deserialize, Serialize};
use uuid::Uuid;

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Block {
    pub id: Uuid,
    pub page_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub order: f64,           // Lexicographic order (fractional indexing)
    pub level: u8,            // Indentation level
    pub format: BlockFormat,
    pub marker: Option<TaskMarker>,
    pub priority: Option<Priority>,
    pub content: String,
    pub properties: HashMap<String, PropertyValue>,
    pub refs: Vec<Uuid>,      // References to other blocks
    pub tags: Vec<String>,
    pub scheduled: Option<DateTime<Utc>>,
    pub deadline: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Page {
    pub id: Uuid,
    pub name: String,         // Canonical name (lowercase)
    pub title: Option<String>,
    pub namespace_id: Option<Uuid>,
    pub journal_day: Option<JournalDay>,
    pub format: BlockFormat,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct JournalDay(pub i32);

impl JournalDay {
    pub fn to_date(&self) -> Option<NaiveDate> {
        let s = self.0.to_string();
        NaiveDate::parse_from_str(&s, "%Y%m%d").ok()
    }

    pub fn from_date(date: NaiveDate) -> Self {
        JournalDay(date.format("%Y%m%d").to_string().parse().unwrap())
    }
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum BlockFormat {
    Markdown,
    Org,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskMarker {
    Now,
    Later,
    Todo,
    Done,
    Cancelled,
}

// Los bloques son entidades inmutables - cada cambio es una nueva versión
// El "estado" se calcula como snapshot del último cambio
```

### 1.2 SQLite Migration System

```rust
// src/db/migrations.rs

use refinery::include_migration_mods;

// Definimos migraciones versionadas

pub fn initial_schema() -> String {
    r#"
    CREATE TABLE pages (
        id BLOB PRIMARY KEY NOT NULL,       -- UUID como bytes
        name TEXT NOT NULL UNIQUE,          -- canonical name
        title TEXT,
        namespace_id BLOB,
        journal_day INTEGER,
        format TEXT NOT NULL DEFAULT 'markdown',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        FOREIGN KEY (namespace_id) REFERENCES pages(id)
    );

    CREATE INDEX idx_pages_name ON pages(name);
    CREATE INDEX idx_pages_journal_day ON pages(journal_day);

    CREATE TABLE blocks (
        id BLOB PRIMARY KEY NOT NULL,
        page_id BLOB NOT NULL,
        parent_id BLOB,
        "order" REAL NOT NULL DEFAULT 0,
        level INTEGER NOT NULL DEFAULT 1,
        format TEXT NOT NULL DEFAULT 'markdown',
        marker TEXT,
        priority TEXT,
        content TEXT NOT NULL DEFAULT '',
        properties BLOB NOT NULL DEFAULT '{}',  -- JSON/Rkyv
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE,
        FOREIGN KEY (parent_id) REFERENCES blocks(id) ON DELETE SET NULL
    );

    CREATE INDEX idx_blocks_page_id ON blocks(page_id);
    CREATE INDEX idx_blocks_parent_id ON blocks(parent_id);
    CREATE INDEX idx_blocks_marker ON blocks(marker);
    CREATE INDEX idx_blocks_priority ON blocks(priority);
    CREATE INDEX idx_blocks_updated ON blocks(updated_at);

    -- Full-text search via FTS5
    CREATE VIRTUAL TABLE blocks_fts USING fts5(
        content,
        content=blocks, content_rowid=rowid
    );

    -- Triggers para mantener FTS sincronizado
    CREATE TRIGGER blocks_ai AFTER INSERT ON blocks BEGIN
        INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
    END;

    CREATE TRIGGER blocks_ad AFTER DELETE ON blocks BEGIN
        INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    END;

    CREATE TRIGGER blocks_au AFTER UPDATE ON blocks BEGIN
        INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
        INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
    END;

    CREATE TABLE refs (
        source_id BLOB NOT NULL,
        target_id BLOB NOT NULL,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (source_id, target_id),
        FOREIGN KEY (source_id) REFERENCES blocks(id) ON DELETE CASCADE,
        FOREIGN KEY (target_id) REFERENCES blocks(id) ON DELETE CASCADE
    );

    CREATE INDEX idx_refs_target ON refs(target_id);

    CREATE TABLE tags (
        page_id BLOB NOT NULL,
        tag TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (page_id, tag),
        FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
    );

    CREATE TABLE aliases (
        page_id BLOB NOT NULL,
        alias TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (page_id, alias),
        FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
    );

    CREATE TABLE files (
        id BLOB PRIMARY KEY NOT NULL,
        path TEXT NOT NULL UNIQUE,
        hash BLOB NOT NULL,
        size_bytes INTEGER NOT NULL,
        mime_type TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    );

    CREATE TABLE assets (
        block_id BLOB NOT NULL,
        file_id BLOB NOT NULL,
        width INTEGER,
        height INTEGER,
        align TEXT DEFAULT 'center',
        external_url TEXT,
        PRIMARY KEY (block_id, file_id),
        FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE,
        FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
    );

    CREATE TABLE kv_store (
        key TEXT PRIMARY KEY NOT NULL,
        value BLOB NOT NULL,
        updated_at INTEGER NOT NULL
    );

    -- Journal pages cache
    CREATE TABLE journals (
        journal_day INTEGER PRIMARY KEY,
        page_id BLOB NOT NULL UNIQUE,
        created_at INTEGER NOT NULL,
        FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
    );

    CREATE TABLE config (
        key TEXT PRIMARY KEY NOT NULL,
        value BLOB NOT NULL,
        updated_at INTEGER NOT NULL
    );
    "#
    .to_string()
}
```

### 1.3 Repository Pattern

```rust
// src/db/repository.rs

use async_trait::async_trait;

#[async_trait]
pub trait BlockRepository: Send + Sync {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Block>>;
    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>>;
    async fn get_children(&self, parent_id: Uuid) -> Result<Vec<Block>>;
    async fn get_with_refs(&self, id: Uuid) -> Result<(Block, Vec<Uuid>)>;
    async fn search(&self, query: &str) -> Result<Vec<Block>>;
    async fn insert(&self, block: &Block) -> Result<()>;
    async fn update(&self, block: &Block) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn move_block(&self, id: Uuid, new_parent: Uuid, order: f64) -> Result<()>;
}

#[async_trait]
pub trait PageRepository: Send + Sync {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Page>>;
    async fn get_by_name(&self, name: &str) -> Result<Option<Page>>;
    async fn get_journal(&self, day: JournalDay) -> Result<Option<Page>>;
    async fn get_all(&self) -> Result<Vec<Page>>;
    async fn create(&self, page: &Page) -> Result<()>;
    async fn rename(&self, id: Uuid, new_name: &str) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}
```

---

## 2. Query Engine — DSL Parse + Execute

### 2.1 Grammar (rust-peg o similar)

```rust
// src/query/dsl/parser.rs

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "src/query/dsl/query.pest"]  // Gramática PEG
pub struct QueryDslParser;

pub fn parse_query(input: &str) -> Result<QueryExpr, ParseError> {
    let pairs = QueryDslParser::parse(Rule::query, input)?;
    build_ast(pairs)
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryExpr {
    And(Vec<QueryExpr>),
    Or(Vec<QueryExpr>),
    Not(Box<QueryExpr>),
    Between {
        field: String,
        start: QueryValue,
        end: QueryValue,
    },
    Property {
        key: String,
        op: PropertyOp,
        value: QueryValue,
    },
    Task(Vec<TaskMarker>),
    Priority(Vec<Priority>),
    Page(String),
    Tags(Vec<String>),
    PageRef(String),
    SelfRef,
    BlockContent(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyOp {
    Equals,
    NotEquals,
    Contains,
    GreaterThan,
    LessThan,
}
```

### 2.2 Query Execution

```rust
// src/query/dsl/executor.rs

pub struct QueryExecutor {
    db: SqlitePool,
}

impl QueryExecutor {
    pub async fn execute(
        &self,
        expr: &QueryExpr,
        opts: QueryOptions,
    ) -> Result<QueryResult, QueryError> {
        let mut sql = String::from(
            "SELECT id, page_id, parent_id, order, level, format, marker, \
             priority, content, properties, created_at, updated_at \
             FROM blocks WHERE "
        );

        let (where_clause, params) = self.build_where(expr, 0)?;
        sql.push_str(&where_clause);

        if let Some(sort) = &opts.sort_by {
            sql.push_str(&format!(" ORDER BY {}", sort));
        }
        if let Some(limit) = opts.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let rows = sqlx::query_as::<_, BlockRow>(&sql)
            .fetch_all(&self.db)
            .await?;

        Ok(QueryResult {
            blocks: rows.into_iter().map(|r| r.into()).collect(),
            count: rows.len(),
        })
    }

    fn build_where(&self, expr: &QueryExpr, depth: u32) -> Result<(String, Vec<Param>), QueryError> {
        if depth > 20 {
            return Err(QueryError::MaxDepthExceeded);
        }

        match expr {
            QueryExpr::And(children) => {
                let clauses: Vec<_> = children.iter()
                    .map(|c| self.build_where(c, depth + 1))
                    .collect::<Result<_, _>>()?;
                Ok(("AND ".to_string() + &clauses.iter()
                    .map(|(c, _)| format!("({})", c))
                    .collect::<Vec<_>>()
                    .join(" AND ")))
            }
            QueryExpr::Or(children) => {
                // Similar a AND pero con OR
            }
            QueryExpr::Not(inner) => {
                let (clause, _) = self.build_where(inner, depth + 1)?;
                Ok((format!("NOT ({})", clause)))
            }
            QueryExpr::Property { key, op, value } => {
                // json_extract(properties, '$.key') = value
            }
            QueryExpr::Task(markers) => {
                let marker_list: Vec<_> = markers.iter()
                    .map(|m| format!("'{}'", m.as_str()))
                    .collect();
                Ok((format!("marker IN ({})", marker_list.join(","))))
            }
            QueryExpr::BlockContent(q) => {
                // FTS5: blocks_fts MATCH ?
                Ok(("EXISTS (SELECT 1 FROM blocks_fts WHERE blocks_fts MATCH ? AND blocks_fts.rowid = blocks.rowid)".to_string()))
            }
            // ... más operadores
        }
    }
}
```

---

## 3. MCP Server — Capa de Integración AI

### 3.1 MCP Server Definition

```rust
// src/mcp/server.rs

use mcp_server::{
    McpServer, McpTool, McpResource, McpNotification,
    McpConnection, McpSubscription,
};

pub struct LogseqMcpServer {
    blocks: Arc<BlockService>,
    pages: Arc<PageService>,
    search: Arc<SearchService>,
    graph: Arc<GraphService>,
    query: Arc<QueryService>,
}

#[mcp_tools]
impl LogseqMcpServer {
    /// Execute a Logseq DSL query against the current graph
    #[tool(name = "logseq_query")]
    async fn query(
        &self,
        #[param(description = "DSL query string")]
        dsl: String,
        #[param(description = "Max results", default = 100)]
        limit: u32,
    ) -> McpResult<QueryResultDto> {
        let expr = parse_query(&dsl)
            .map_err(|e| McpError::parse_error(e))?;

        let result = self.query.execute(expr, QueryOptions::new().limit(limit)).await?;

        Ok(QueryResultDto::from(result))
    }

    /// Create a new block on a page
    #[tool(name = "logseq_create_block")]
    async fn create_block(
        &self,
        #[param(description = "Page name")]
        page_name: String,
        #[param(description = "Block content (markdown)")]
        content: String,
        #[param(description = "Parent block UUID (optional)")]
        parent_id: Option<Uuid>,
        #[param(description = "Block marker (optional)")]
        marker: Option<TaskMarker>,
    ) -> McpResult<BlockDto> {
        let block = self.blocks.create(
            CreateBlockRequest {
                page_name,
                content,
                parent_id,
                marker,
                ..Default::default()
            }
        ).await?;

        // Notify subscribers
        self.notify_block_changed(BlockChangedEvent {
            block_id: block.id,
            change_type: ChangeType::Created,
        }).await;

        Ok(BlockDto::from(block))
    }

    /// Search across all pages and blocks
    #[tool(name = "logseq_search")]
    async fn search(
        &self,
        #[param(description = "Search query")]
        query: String,
        #[param(description = "Max results", default = 50)]
        limit: u32,
    ) -> McpResult<Vec<SearchResultDto>> {
        let results = self.search.full_text(&query, limit).await?;
        Ok(results.into_iter().map(SearchResultDto::from).collect())
    }

    /// Get a specific block with all its children recursively
    #[tool(name = "logseq_get_block_tree")]
    async fn get_block_tree(
        &self,
        #[param(description = "Block UUID (root)")]
        block_id: Uuid,
    ) -> McpResult<BlockTreeDto> {
        let tree = self.blocks.get_tree(block_id).await?;
        Ok(BlockTreeDto::from(tree))
    }

    /// Get all blocks on a page
    #[tool(name = "logseq_get_page_blocks")]
    async fn get_page_blocks(
        &self,
        #[param(description = "Page name")]
        page_name: String,
        #[param(description = "Format (markdown/org)")]
        format: Option<BlockFormat>,
    ) -> McpResult<Vec<BlockDto>> {
        let blocks = self.blocks.get_by_page(&page_name, format).await?;
        Ok(blocks.into_iter().map(BlockDto::from).collect())
    }

    /// List all pages in the graph
    #[tool(name = "logseq_list_pages")]
    async fn list_pages(&self) -> McpResult<Vec<PageDto>> {
        let pages = self.pages.get_all().await?;
        Ok(pages.into_iter().map(PageDto::from).collect())
    }

    /// Create or get a journal page for a specific date
    #[tool(name = "logseq_get_journal")]
    async fn get_journal(
        &self,
        #[param(description = "Date in YYYY-MM-DD format")]
        date: String,
    ) -> McpResult<PageDto> {
        let day = JournalDay::from_date(date.parse().unwrap());
        let page = self.pages.get_or_create_journal(day).await?;
        Ok(PageDto::from(page))
    }

    /// Create a task with optional deadline
    #[tool(name = "logseq_create_task")]
    async fn create_task(
        &self,
        #[param(description = "Page name")]
        page_name: String,
        #[param(description = "Task content")]
        content: String,
        #[param(description = "Deadline date (YYYY-MM-DD)")]
        deadline: Option<String>,
        #[param(description = "Priority")]
        priority: Option<Priority>,
    ) -> McpResult<BlockDto> {
        let block = self.blocks.create(
            CreateBlockRequest {
                page_name,
                content,
                marker: Some(TaskMarker::Todo),
                priority,
                deadline: deadline.map(|d| d.parse().unwrap()),
                ..Default::default()
            }
        ).await?;

        Ok(BlockDto::from(block))
    }

    /// Link one block to another (create a reference)
    #[tool(name = "logseq_link_blocks")]
    async fn link_blocks(
        &self,
        #[param(description = "Source block UUID")]
        source_id: Uuid,
        #[param(description = "Target block UUID")]
        target_id: Uuid,
    ) -> McpResult<()> {
        self.blocks.create_ref(source_id, target_id).await?;
        Ok(())
    }

    /// Get all backlinks pointing to a block
    #[tool(name = "logseq_get_backlinks")]
    async fn get_backlinks(
        &self,
        #[param(description = "Target block UUID")]
        target_id: Uuid,
    ) -> McpResult<Vec<BlockDto>> {
        let backlinks = self.blocks.get_backlinks(target_id).await?;
        Ok(backlinks.into_iter().map(BlockDto::from).collect())
    }

    /// Delete a block (soft-delete to recycle bin)
    #[tool(name = "logseq_delete_block")]
    async fn delete_block(
        &self,
        #[param(description = "Block UUID")]
        block_id: Uuid,
    ) -> McpResult<()> {
        self.blocks.soft_delete(block_id).await?;
        Ok(())
    }
}

#[mcp_resources]
impl LogseqMcpServer {
    /// Full graph data as a resource
    #[resource(
        uri = "logseq://graph",
        name = "Current Graph",
        mime = "application/json"
    )]
    async fn graph(&self) -> McpResult<GraphDto> {
        let graph = self.graph.get_current().await?;
        Ok(GraphDto::from(graph))
    }

    /// All pages as a list resource
    #[resource(
        uri = "logseq://pages",
        name = "All Pages",
        mime = "application/json"
    )]
    async fn pages(&self) -> McpResult<Vec<PageDto>> {
        let pages = self.pages.get_all().await?;
        Ok(pages.into_iter().map(PageDto::from).collect())
    }

    /// Specific page resource
    #[resource(
        uri = "logseq://pages/{name}",
        name = "Page by Name",
        mime = "application/json"
    )]
    async fn page_by_name(
        &self,
        #[param(name = "name")]
        name: String,
    ) -> McpResult<PageDto> {
        let page = self.pages.get_by_name(&name).await?
            .ok_or(McpError::not_found("Page not found"))?;
        Ok(PageDto::from(page))
    }

    /// Journal page for a date
    #[resource(
        uri = "logseq://journal/{date}",
        name = "Journal Page",
        mime = "application/json"
    )]
    async fn journal_page(
        &self,
        #[param(name = "date")]
        date: String,
    ) -> McpResult<PageDto> {
        let day = JournalDay::from_date(date.parse().unwrap());
        let page = self.pages.get_journal(day).await?
            .ok_or(McpError::not_found("Journal not found"))?;
        Ok(PageDto::from(page))
    }
}

#[mcp_notifications]
impl LogseqMcpServer {
    /// Notified when a block is created, updated, or deleted
    #[notification(name = "logseq_block_changed")]
    async fn block_changed(&self, event: BlockChangedEvent) {
        for subscriber in self.get_subscribers("logseq://blocks/*").await {
            self.send_notification(&subscriber, &event).await;
        }
    }

    /// Notified when a new page is created
    #[notification(name = "logseq_page_created")]
    async fn page_created(&self, event: PageCreatedEvent) {
        for subscriber in self.get_subscribers("logseq://pages/*").await {
            self.send_notification(&subscriber, &event).await;
        }
    }

    /// Notified when backlinks change
    #[notification(name = "logseq_backlinks_changed")]
    async fn backlinks_changed(&self, event: BacklinksChangedEvent) {
        let uri = format!("logseq://blocks/{}/backlinks", event.block_id);
        for subscriber in self.get_subscribers(&uri).await {
            self.send_notification(&subscriber, &event).await;
        }
    }
}
```

### 3.2 MCP Types (Dto Layer)

```rust
// src/mcp/dto.rs

#[derive(Serialize, Deserialize)]
pub struct BlockDto {
    pub id: Uuid,
    pub page_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub level: u8,
    pub content: String,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub properties: HashMap<String, serde_json::Value>,
    pub refs: Vec<Uuid>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct PageDto {
    pub id: Uuid,
    pub name: String,
    pub title: Option<String>,
    pub namespace: Option<String>,
    pub journal_day: Option<String>,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub block_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct QueryResultDto {
    pub blocks: Vec<BlockDto>,
    pub count: usize,
    pub query: String,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResultDto {
    pub block: BlockDto,
    pub page_name: String,
    pub score: f64,
    pub snippet: String,
}

#[derive(Serialize, Deserialize)]
pub struct BlockTreeDto {
    pub block: BlockDto,
    pub children: Vec<BlockTreeDto>,
    pub total_blocks: usize,
}

impl From<Block> for BlockDto {
    fn from(b: Block) -> Self {
        Self {
            id: b.id,
            page_id: b.page_id,
            parent_id: b.parent_id,
            level: b.level,
            content: b.content,
            marker: b.marker.map(|m| m.as_str().to_string()),
            priority: b.priority.map(|p| format!("{:?}", p)),
            properties: b.properties.into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            refs: b.refs,
            tags: b.tags,
            created_at: b.created_at.to_rfc3339(),
            updated_at: b.updated_at.to_rfc3339(),
        }
    }
}
```

---

## 4. AI Agent Interaction Patterns

### 4.1 Research Assistant Pattern

```
User: "Summarize my research notes on Quantum Computing"
AI (Claude via MCP):
  → logseq_query("page=\"Quantum Computing\"")
  → [blocks]
  → logseq_get_block_tree(current_id)
  → [full tree]
  → Synthesize summary
  → logseq_create_block("Summary", summary_text)
  → [ok]
User: sees summary in UI
```

### 4.2 Auto-Tagging Pattern

```
AI (background agent):
  → logseq_list_pages()
  → [all pages]
  → For each page:
    → logseq_get_page_blocks(page.name)
    → Validate content
    → AI detects topic: "rust webassembly"
    → logseq_create_block(topicpage, "[[rust]] [[wasm]]", tag)
    → logseq_link_blocks(content_block, tagpage)
User: opens page, sees auto-tags
```

### 4.3 Cross-Reference Discovery

```
AI (on page load):
  → logseq_get_page_blocks("Rust async")
  → logseq_search("tokio + runtime + async")
  → [related pages]
  → logseq_notify "New connections found"
User: sees suggestion bar
```

### 4.4 Agent Subscription Flow

```rust
// AI agent se subscribe a cambios en un página

impl LogseqMcpServer {
    async fn subscribe_to_page_changes(
        &self,
        agent_id: AgentId,
        page_name: &str,
    ) -> McpResult<()> {
        let uri = format!("logseq://pages/{}", page_name);

        self.add_subscription(agent_id, Subscription {
            uri,
            events: vec![
                "logseq_block_changed".into(),
                "logseq_page_created".into(),
            ],
            callback: Box::new(move |event| {
                // AI recibe notificación en tiempo real
                notify_agent(agent_id, event)
            }),
        }).await?;

        Ok(())
    }
}
```

---

## 5. Event System — Tokio

### 5.1 Event Bus Architecture

```rust
// src/events/event_bus.rs

use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum AppEvent {
    // Graph events
    GraphLoaded { graph_id: Uuid },
    GraphSwitched { from: Uuid, to: Uuid },
    GraphDeleted { graph_id: Uuid },

    // Block events
    BlockCreated(BlockChanged),
    BlockUpdated(BlockChanged),
    BlockDeleted { block_id: Uuid, page_id: Uuid },
    BlockMoved { block_id: Uuid, from_parent: Uuid, to_parent: Uuid },

    // Page events
    PageCreated { page_id: Uuid, name: String },
    PageRenamed { page_id: Uuid, old_name: String, new_name: String },
    PageDeleted { page_id: Uuid, name: String },

    // UI events
    SearchIndexReady,
    SyncStateChanged { state: SyncState },
    FileChanged { path: String },
}

pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
    handlers: Vec<HandlerRegistration>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self { tx, handlers: Vec::new() }
    }

    pub fn publish(&self, event: AppEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }
}

// Cada handler es un actor Tokio

pub struct BlockHandler {
    rx: broadcast::Receiver<AppEvent>,
    db: Arc<BlockService>,
    mcp: Arc<McpServer>,
    search: Arc<SearchService>,
}

impl BlockHandler {
    pub async fn run(mut self) {
        loop {
            match self.rx.recv().await {
                Ok(AppEvent::BlockCreated(change)) => {
                    // 1. Persistir a SQLite
                    // 2. Actualizar FTS index
                    // 3. Notificar MCP subscribers
                    // 4. Disparar background AI tagging
                    // 5. Aggiornar search index
                    self.handle_block_created(change).await;
                }
                Ok(AppEvent::BlockUpdated(change)) => {
                    self.handle_block_updated(change).await;
                }
                Ok(AppEvent::BlockDeleted { block_id, page_id }) => {
                    self.handle_block_deleted(block_id, page_id).await;
                }
                _ => {}
            }
        }
    }

    async fn handle_block_created(&self, change: BlockChanged) {
        // 1. DB persistence
        self.db.insert(&change.block).await;

        // 2. FTS update
        self.search.index_block(&change.block).await;

        // 3. MCP notification
        self.mcp.emit_block_changed(BlockChangedEvent {
            block_id: change.block.id,
            change_type: ChangeType::Created,
        }).await;

        // 4. Background AI processing (non-blocking)
        tokio::spawn(async move {
            ai_tagging_suggest(change.block).await;
            ai_link_discovery(change.block).await;
        });
    }
}
```

### 5.2 Background AI Tasks

```rust
// src/ai/background_tasks.rs

use tokio::time::{interval, Duration};

pub struct AITaggingService {
    blocks: Arc<BlockService>,
    ai_client: Arc<AIClient>,
}

impl AITaggingService {
    pub async fn run(self) {
        let mut ticker = interval(Duration::from_secs(10));

        loop {
            ticker.tick().await;

            // 1. Search blocks sin tags (limit 100)
            let untagged = self.blocks
                .query("NOT(tags)")
                .limit(100)
                .execute()
                .await
                .unwrap_or_default();

            // 2. Para cada bloque, sugerir tags via AI
            for block in untagged.blocks {
                let ai = self.ai_client.clone();
                let blocks_ref = self.blocks.clone();
                tokio::spawn(async move {
                    let tags = ai.suggest_tags(&block.content).await;
                    for tag in tags {
                        let _ = blocks_ref.add_tag(block.id, tag).await;
                    }
                });
            }
        }
    }
}

pub struct AILinkDiscovery {
    blocks: Arc<BlockService>,
    ai_client: Arc<AIClient>,
}

impl AILinkDiscovery {
    pub async fn run(self) {
        let mut ticker = interval(Duration::from_secs(60));

        loop {
            ticker.tick().await;

            // 1. Buscar bloques recién createados (últimos 5 minutos)
            let recent = self.blocks
                .query("between(created_at, -5m, now)")
                .execute()
                .await
                .unwrap_or_default();

            // 2. Para cada bloque reciente, buscar conexiones
            for block in recent.blocks {
                let similar = self.blocks
                    .search(&block.content, 5)
                    .await
                    .unwrap_or_default();

                for related in similar {
                    if related.id != block.id {
                        // 3. Crear backlink
                        let _ = self.blocks.create_ref(block.id, related.id).await;
                    }
                }
            }
        }
    }
}
```

---

## 6. Sync — Current Implementation vs Planned

> **⚠️ IMPLEMENTATION STATUS**: This section describes the **PLANNED** sync architecture using Loro CRDT.
>
> **CURRENT IMPLEMENTATION**: The actual sync implementation in `quilt-sync` uses **custom LWW (Last-Write-Wins)** strategy,
> not true CRDT. This is documented as an architectural decision to be revisited.
>
> **Key mismatch**: The spec/design documents reference Loro CRDT integration, but the actual `quilt-sync` crate
> implements a simpler LWW approach. For details, see `docs/reversa/_reversa_sdd/LLM_FIRST_ROADMAP.md`.
>
> **Planned resolution**: Either adopt true Loro CRDT per the design, or formalize LWW as the intentional strategy.

### 6.1 Loro CRDT Integration (Planned)

```rust
// src/sync/crdt.rs

use loro::{LoroDoc, LoroText, LoroList, LoroMap};

pub struct CrdtSyncEngine {
    doc: LoroDoc,
    peer_id: uuid::Uuid,
    last_synced: Instant,
}

impl CrdtSyncEngine {
    pub fn new(peer_id: uuid::Uuid) -> Self {
        Self {
            doc: LoroDoc::new(),
            peer_id,
            last_synced: Instant::now(),
        }
    }

    // Convertir un Block a una operación CRDT
    pub fn block_to_crdt_op(&mut self, block: &Block) -> LoroText {
        let text = self.doc.get_text(&block.id.to_string());
        let bytes = rkyv::to_bytes::<_, 256>(block).unwrap();
        // text.update(bytes);
        text
    }

    // Aplicar cambios remotos
    pub fn apply_remote_change(
        &mut self,
        remote_bytes: &[u8],
    ) -> Result<(), SyncError> {
        self.doc.import(remote_bytes)
            .map_err(|e| SyncError::ImportFailed(e.to_string()))?;

        // Verificar conflictos
        if let Some(version) = self.doc.get_change_at_latest() {
            // Procesar posibles conflictos
            self.resolve_conflicts(version)?;
        }

        self.last_synced = Instant::now();
        Ok(())
    }

    fn resolve_conflicts(&mut self, version: LoroVersion) -> Result<(), SyncError> {
        // Loro automáticamente resuelve la mayoría de conflictos
        // Solo intervenimos en casos específicos

        // Ejemplo: si dos peers cambiaron el mismo bloque
        let conflicts = self.doc.get_deep_value();
        for conflict in conflicts.iter() {
            match conflict {
                Conflict::ConcurrentEdit { id, peer_a, peer_b } => {
                    // Estrategia: Last-Write-Wins por timestamp
                    let winner = if peer_a.timestamp > peer_b.timestamp {
                        peer_a
                    } else {
                        peer_b
                    };

                    // En caso de empate, el de mayor peer_id gana
                    let winner = if peer_a.timestamp == peer_b.timestamp {
                        if peer_a.peer_id > peer_b.peer_id { peer_a } else { peer_b }
                    } else {
                        winner
                    };

                    // Aplicar el winner
                    self.doc.apply(&winner.data)?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
```

### 6.2 Offline Queue

```rust
// src/sync/offline.rs

pub struct OfflineQueue {
    queue: WAL<ChangeRecord>,
    sync_engine: Arc<CrdtSyncEngine>,
}

impl OfflineQueue {
    pub async fn enqueue(&mut self, change: ChangeRecord) -> Result<(), SyncError> {
        // 1. Guardar en WAL local
        self.queue.append(change.clone())?;

        // 2. Intentar sync inmediato
        if self.sync_engine.is_online().await {
            if let Err(e) = self.sync_engine.push_change(change).await {
                // Marcar como pending para sync futuro
                self.queue.mark_pending()?;
            }
        }

        Ok(())
    }

    pub async fn flush_pending(&mut self) -> Result<usize, SyncError> {
        let pending = self.queue.get_pending().await?;
        let mut synced = 0;

        for change in pending {
            match self.sync_engine.push_change(change).await {
                Ok(_) => {
                    self.queue.remove(&change.id).await?;
                    synced += 1;
                }
                Err(SyncError::Conflict(e)) => {
                    // Conflictos se resuelven automáticamente via CRDT
                }
                Err(e) => {
                    log::error!("Sync failed: {}", e);
                    // Dejar en queue para próximo intento
                }
            }
        }

        Ok(synced)
    }
}
```

---

## 7. Tauri Desktop Shell

```rust
// src-tauri/main.rs

use tauri::{App, Manager, Window};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Inicializar servicios Rust
            let db = BlockService::new(app.path().app_data_dir().unwrap());
            let search = SearchService::new(db.clone());
            let mcp = McpServer::new(db.clone());

            // Exponer como comandos Tauri
            app.manage(db);
            app.manage(search);
            app.manage(mcp);

            // Iniciar MCP server en background
            let mcp_server = app.state::<McpServer>().clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(mcp_server.serve(3541));
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            query_blocks,
            create_block,
            search_blocks,
            get_page,
            list_pages,
            get_journal,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}

#[tauri::command]
async fn query_blocks(
    db: State<'_, Arc<BlockService>>,
    dsl: String,
) -> Result<Vec<BlockDto>, String> {
    let result = db.query(&dsl).await.map_err(|e| e.to_string())?;
    Ok(result.blocks.into_iter().map(BlockDto::from).collect())
}

#[tauri::command]
async fn create_block(
    db: State<'_, Arc<BlockService>>,
    page_name: String,
    content: String,
) -> Result<BlockDto, String> {
    let block = db.create(CreateBlockRequest {
        page_name,
        content,
        ..Default::default()
    }).await.map_err(|e| e.to_string())?;

    Ok(BlockDto::from(block))
}
```

---

## 8. Cargo Manifest

```toml
# Cargo.toml

[package]
name = "logseq-rs"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
rkyv = { version = "0.8", features = ["validation"] }

# Async
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Query
pest = "2.7"
pest_derive = "2.7"

# Sync (CRDT)
loro = "0.2"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }

# Date
chrono = "0.4"

# Search
sonic-channel = "1.3"

# MCP
mcp-sdk = "0.1"

# Error handling
thiserror = "1"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Metrics
metrics = "0.23"
metrics-exporter-prometheus = "0.15"

# Desktop
tauri = { version = "2", features = ["build"] }

# Backup policy
backoff = "0.4"

# Watch
notify = "7"

# LRU cache
lru = "0.13"

# Image processing
image = "0.25"
```

---

## 9. Resumen de la Arquitectura

```
┌──────────────────────────────────────────────────────────────┐
│                     AI Agent (Claude/GPT)                     │
│   Usa logseq_query, logseq_create_block, logseq_search, etc  │
└──────────────────────┬───────────────────────────────────────┘
                       │ MCP Protocol (JSON-RPC over HTTP/WS)
┌──────────────────────┴───────────────────────────────────────┐
│                    MCP Server (Rust)                          │
│  - 10 Tools (query, create, search, get_tree, etc.)          │
│  - 4 Resource types (graph, pages, journal, blocks)          │
│  - 3 Notifications (block_changed, page_created, backlinks)  │
└──────────────────────┬───────────────────────────────────────┘
                       │
┌──────────────────────┴───────────────────────────────────────┐
│                  Application Service Layer                    │
│  BlockService, PageService, SearchService, QueryService      │
│  GraphService, SyncService, AITaggingService                 │
└──────────────────────┬───────────────────────────────────────┘
                       │
┌──────────────────────┴───────────────────────────────────────┐
│                     Data Layer (SQLite)                       │
│  10 tables, FTS5 full-text, triggers, WAL journal            │
└──────────────────────────────────────────────────────────────┘
                       │
┌──────────────────────┴───────────────────────────────────────┐
│                     Platform Layer                            │
│  Tauri Desktop │ Leptos/Yew WASM │ Clap CLI                  │
└──────────────────────────────────────────────────────────────┘
```

**Principio clave:** Todo el sistema es MCP-first. La UI es una skin sobre MCP. Los AI agents son first-class citizens. Los humanos usan la misma API (via UI) que los AI agents (via MCP).
