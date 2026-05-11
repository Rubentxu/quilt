# Quilt — Roadmap de Implementación

> **Proyecto**: Clon de Logseq DB en Rust con MCP-first architecture
> **Fecha**: 2026-05-02
> **Versión**: 1.0
> **Estado**: Planning

---

## 0. Visión y Principios

### 0.1 Visión

Quilt es un sistema PKM (Personal Knowledge Management) AI-first que reimplementa las features core de Logseq en Rust, con MCP como capa de integración nativa para agentes AI.

**Diferenciación**: Logseq es un PKM con AI añadida. Quilt es un sistema cognitivo donde AI y humano co-evolucionan conocimiento.

### 0.2 Principios de Diseño

```
1. MCP-first: La API pública es MCP. UI consume la misma API.
2. Zero panics: Error handling estructurado, nunca panics en runtime.
3. WASM target: Compila a WASM para browser sin cambios.
4. AI-native: Agentes AI son first-class citizens.
5. Observability: Tracing + métricas desde día 0.
```

### 0.3 Stack Técnico

| Componente | Tecnología | Justificación |
|------------|------------|---------------|
| Language | Rust 2024 | Performance, memory safety, WASM |
| Async | Tokio | Runtime async estándar en Rust |
| Database | SQLite + Rkyv | Embeddable, ACID, zero-copy serialization |
| Search | FTS5 | Full-text search integrado en SQLite |
| Serialization | Rkyv | Zero-copy Rust serialization |
| WebAssembly | Leptos o Yew | UI framework Rust-native |
| Desktop | Tauri | Lightweight, Rust-native |
| MCP | Official MCP Rust SDK | Standard AI agent protocol |
| Sync | Loro CRDT | Conflict-free replicated data types |
| CLI | Clap | Standard Rust CLI |

---

## 1. Arquitectura DDD

> ⚠️ **VERSIÓN 2.0**: El proyecto ahora sigue **Arquitectura DDD** con crates por bounded context.
> Ver documento completo: `docs/architecture-ddd.md`

### 1.1 Estructura de Crates

```
quilt/
├── Cargo.toml                      # Workspace root - dependencias centralizadas
├── src/
│   └── main.rs                    # Binary entry
├── crates/
│   ├── quilt-domain/              # 🟢 Pure domain - ZERO dependencias externas
│   ├── quilt-application/         # 🟡 Use cases - depende solo de domain
│   ├── quilt-infrastructure/      # 🔴 Implementaciones - implementa traits de domain
│   ├── quilt-query/               # Query DSL parsing + execution
│   ├── quilt-search/              # Full-text search
│   ├── quilt-sync/                # CRDT sync engine
│   ├── quilt-mcp/                # MCP protocol layer
│   └── quilt-platform/           # Tauri + CLI adapters
└── tests/
```

### 1.2 Principios DDD Aplicados

| Principio | Implementación |
|-----------|---------------|
| **Bounded Contexts** | Cada crate = un contexto delimitado |
| **Dependence Rule** | Domain ← Application ← Infrastructure |
| **Zero Dependencies Domain** | `quilt-domain` no tiene deps externas |
| **Traits como Contratos** | Repositorios son traits, no implementaciones |

## 2. Fases de Implementación

```
FASE 0: Foundation DDD (Semanas 1-4)
├── 0.1 Scaffold workspace + Cargo.toml raíz
├── 0.2 Crear crates skeleton (8 crates)
├── 0.3 quilt-domain: entities + value objects
├── 0.4 quilt-domain: repository traits
└── 0.5 Tests domain layer

FASE 1: Application + Infrastructure (Semanas 5-10)
├── 1.1 quilt-application: command/query handlers
├── 1.2 quilt-infrastructure: SQLite setup + migrations
├── 1.3 quilt-infrastructure: SqliteBlockRepository
├── 1.4 quilt-infrastructure: SqlitePageRepository
└── 1.5 Integration tests

FASE 2: Query System (Semanas 11-16)
├── 2.1 quilt-query: grammar + parser (PEG)
├── 2.2 quilt-query: AST + visitor
├── 2.3 quilt-query: executor
├── 2.4 Time helpers
└── 2.5 Query integration tests

FASE 3: Search (Semanas 17-20)
├── 3.1 FTS5 setup + index management
├── 3.2 quilt-search: indexing
├── 3.3 quilt-search: fuzzy search + ranking
└── 3.4 Search integration tests

FASE 4: MCP Layer (Semanas 21-26)
├── 4.1 quilt-mcp: server scaffold
├── 4.2 Tools (10+): query, create_block, search, etc
├── 4.3 Resources (4 types): graph, pages, journal, blocks
├── 4.4 Notifications (3 types): block_changed, page_created, backlinks
└── 4.5 MCP conformance tests

FASE 5: Sync Engine (Semanas 27-32)
├── 5.1 quilt-sync: Loro CRDT integration
├── 5.2 Conflict resolution strategies
├── 5.3 Offline queue + WAL
├── 5.4 Sync state machine
└── 5.5 E2E encryption (v2)

FASE 6: Platform (Semanas 33-40)
├── 6.1 quilt-platform: Tauri setup
├── 6.2 CLI commands (clap)
├── 6.3 File system watcher
├── 6.4 Deep link handling
└── 6.5 System tray + notifications

FASE 7: UI (Semanas 41-48) [v2]
├── 7.1 Leptos/Yew scaffold
├── 7.2 Journal view + briefing
├── 7.3 Graph view (cognitive map)
├── 7.4 Focus mode editor
└── 7.5 Query builder UI

FASE 8: AI Cognitive (Semanas 49-56) [v2]
├── 8.1 Cognitive Mirror
├── 8.2 Serendipity Engine
├── 8.3 Argument Cartographer
├── 8.4 Mental Model Gardener
└── 8.5 Agent Memory

FASE 9: Polish + Hardening (Semanas 57-60)
├── 9.1 Performance optimization
├── 9.2 Error handling hardening
├── 9.3 Full test coverage
└── 9.4 Documentation
```

---

## 3. Detalle de Fases (DDD)

> ⚠️ El detalle técnico completo de cada fase está en `docs/architecture-ddd.md`

### FASE 0: Foundation DDD (Semanas 1-4)

#### 0.1 Scaffold Workspace + Cargo.toml Raíz

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/quilt-domain",
    "crates/quilt-application",
    "crates/quilt-infrastructure",
    "crates/quilt-query",
    "crates/quilt-search",
    "crates/quilt-sync",
    "crates/quilt-mcp",
    "crates/quilt-platform",
]

[workspace.dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
# ... todas las deps centralizadas aquí
```

#### 0.2-0.5 Ver `docs/architecture-ddd.md` Sección 4-6

**Cargo.toml dependencies**:
```toml
[dependencies]
# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate"] }
rkyv = { version = "0.8", features = ["validation"] }

# Async
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }

# Date
chrono = { version = "0.4", features = ["serde"] }

# Error handling
thiserror = "1"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Utilities
once_cell = "1"
parking_lot = "0.12"
```

#### 1.2 Schema SQLite Completo (10 tablas)

Basado en el ERD de Logseq con 8 entidades + kv_store + journals cache.

```sql
-- Tablas core
CREATE TABLE blocks (
    id BLOB PRIMARY KEY NOT NULL,       -- UUID como bytes
    page_id BLOB NOT NULL,
    parent_id BLOB,
    "order" REAL NOT NULL DEFAULT 0,   -- Lexicographic order (fractional indexing)
    level INTEGER NOT NULL DEFAULT 1,
    format TEXT NOT NULL DEFAULT 'markdown',
    marker TEXT,
    priority TEXT,
    content TEXT NOT NULL DEFAULT '',
    properties BLOB NOT NULL DEFAULT '{}',
    scheduled INTEGER,
    deadline INTEGER,
    start_time INTEGER,
    repeated INTEGER,
    logbook INTEGER,
    collapsed INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    refs BLOB NOT NULL DEFAULT '[]',   -- JSON array de Uuid
    tags BLOB NOT NULL DEFAULT '[]',
    FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES blocks(id) ON DELETE SET NULL
);

CREATE TABLE pages (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    title TEXT,
    namespace_id BLOB,
    journal_day INTEGER,
    format TEXT NOT NULL DEFAULT 'markdown',
    file_id BLOB,
    original_name TEXT,
    journal INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (namespace_id) REFERENCES pages(id) ON DELETE SET NULL,
    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE SET NULL
);

CREATE TABLE files (
    id BLOB PRIMARY KEY NOT NULL,
    path TEXT NOT NULL UNIQUE,
    content TEXT,
    hash BLOB NOT NULL,
    size_bytes INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    last_modified_at INTEGER NOT NULL
);

CREATE TABLE tags (
    page_id BLOB PRIMARY KEY NOT NULL,
    tag TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
);

CREATE TABLE aliases (
    page_id BLOB NOT NULL,
    alias TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (page_id, alias),
    FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
);

CREATE TABLE refs (
    source_id BLOB NOT NULL,
    target_id BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (source_id, target_id),
    FOREIGN KEY (source_id) REFERENCES blocks(id) ON DELETE CASCADE,
    FOREIGN KEY (target_id) REFERENCES blocks(id) ON DELETE CASCADE
);

CREATE TABLE assets (
    block_id BLOB NOT NULL,
    file_id BLOB NOT NULL,
    asset_type TEXT NOT NULL,
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

-- Índices
CREATE INDEX idx_blocks_page_id ON blocks(page_id);
CREATE INDEX idx_blocks_parent_id ON blocks(parent_id);
CREATE INDEX idx_blocks_marker ON blocks(marker);
CREATE INDEX idx_blocks_priority ON blocks(priority);
CREATE INDEX idx_blocks_updated_at ON blocks(updated_at);
CREATE INDEX idx_blocks_journal_day ON blocks(journal_day);
CREATE INDEX idx_pages_name ON pages(name);
CREATE INDEX idx_pages_journal_day ON pages(journal_day);
CREATE INDEX idx_pages_namespace ON pages(namespace_id);
CREATE INDEX idx_refs_target_id ON refs(target_id);
CREATE INDEX idx_tags_tag ON tags(tag);

-- FTS5 para full-text search
CREATE VIRTUAL TABLE blocks_fts USING fts5(
    content,
    content=blocks,
    content_rowid=rowid
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
```

#### 1.3 Repository Pattern + Basic CRUD

```rust
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
    async fn move_block(&self, id: Uuid, new_parent: Option<Uuid>, new_order: f64) -> Result<()>;
}

#[async_trait]
pub trait PageRepository: Send + Sync {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Page>>;
    async fn get_by_name(&self, name: &str) -> Result<Option<Page>>;
    async fn get_journal(&self, day: JournalDay) -> Result<Option<Page>>;
    async fn get_all(&self) -> Result<Vec<Page>>;
    async fn get_namespace_pages(&self, namespace_id: Uuid) -> Result<Vec<Page>>;
    async fn create(&self, page: &Page) -> Result<()>;
    async fn rename(&self, id: Uuid, new_name: &str) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}

#[async_trait]
pub trait TagRepository: Send + Sync {
    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<String>>;
    async fn get_pages_with_tag(&self, tag: &str) -> Result<Vec<Uuid>>;
    async fn add_tag(&self, page_id: Uuid, tag: &str) -> Result<()>;
    async fn remove_tag(&self, page_id: Uuid, tag: &str) -> Result<()>;
}
```

#### 1.4 Block Entity Completa

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    pub page_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub order: f64,              // Lexicographic order (fractional indexing)
    pub level: u8,               // Indentation level
    pub format: BlockFormat,
    pub marker: Option<TaskMarker>,
    pub priority: Option<Priority>,
    pub content: String,
    pub properties: HashMap<String, PropertyValue>,
    pub refs: Vec<Uuid>,         // References to other blocks
    pub tags: Vec<String>,
    pub scheduled: Option<DateTime<Utc>>,
    pub deadline: Option<DateTime<Utc>>,
    pub start_time: Option<DateTime<Utc>>,
    pub repeated: Option<DateTime<Utc>>,
    pub logbook: Option<DateTime<Utc>>,
    pub collapsed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JournalDay(i32);  // YYYYMMDD

impl JournalDay {
    pub fn from_ymd(year: u16, month: u8, day: u8) -> Option<Self> { ... }
    pub fn as_int(&self) -> i32 { self.0 }
    pub fn as_string(&self) -> String { format!("{}", self.0) }
    pub fn to_naive_date(&self) -> Option<NaiveDate> { ... }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockFormat {
    Markdown,
    Org,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskMarker {
    Now,
    Later,
    Todo,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    A,
    B,
    C,
}
```

**Reglas de Negocio a Implementar**:
- UUID de bloque no puede cambiar una vez creado
- Bloques no pueden ser movidos a sus propios descendientes (validación circular)
- Orden lexicográfico para siblings
- Block properties se normalizan: lowercase, `/` → `-`, spaces → `-`

---

### FASE 1: Core Entities (Semanas 5-8)

#### 1.5 Page Entity + Namespaces

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: Uuid,
    pub name: String,           // Canonical name (lowercase)
    pub title: Option<String>,
    pub namespace_id: Option<Uuid>,
    pub journal_day: Option<JournalDay>,
    pub format: BlockFormat,
    pub file_id: Option<Uuid>,
    pub original_name: Option<String>,
    pub journal: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Jerarquía de namespaces**:
```
Namespace (parent)
  └── Page (child with namespace)
        └── Block
```

#### 1.6 Journal Pages

```rust
impl Page {
    pub fn is_journal(&self) -> bool { self.journal_day.is_some() }

    pub fn journal_name(day: JournalDay) -> String {
        day.to_naive_date()
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| day.as_int().to_string())
    }
}

pub struct JournalService {
    page_repo: Arc<dyn PageRepository>,
}

impl JournalService {
    pub async fn get_or_create_journal(&self, day: JournalDay) -> Result<Page> {
        // Check si existe
        if let Some(page) = self.page_repo.get_journal(day).await? {
            return Ok(page);
        }
        // Crear nuevo journal
        let page = Page {
            id: Uuid::new_v4(),
            name: Self::journal_name(day),
            title: None,
            namespace_id: None,
            journal_day: Some(day),
            format: BlockFormat::Markdown,
            file_id: None,
            original_name: None,
            journal: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.page_repo.create(&page).await?;
        Ok(page)
    }
}
```

#### 1.7 Tags + Aliases

```rust
// Tags son páginas con clase especial
pub struct Tag {
    pub page_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

// Aliases son nombres alternativos para páginas
pub struct Alias {
    pub page_id: Uuid,
    pub alias: String,
    pub created_at: DateTime<Utc>,
}
```

**Built-in Properties**:
| Property | Tipo | Descripción |
|----------|------|-------------|
| `title` | String | Título de la página |
| `alias` | [String] | Nombres alternativos |
| `tags` | [PageRef] | Tags/clasificaciones |
| `priority` | String | Prioridad (A/B/C) |
| `schedule` | Timestamp | Fecha programada |
| `deadline` | Timestamp | Fecha límite |
| `created` | Timestamp | Fecha de creación |
| `updated` | Timestamp | Fecha de actualización |

#### 1.8 File + Asset Management

```rust
pub struct File {
    pub id: Uuid,
    pub path: String,
    pub content: Option<String>,
    pub hash: Vec<u8>,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub last_modified_at: DateTime<Utc>,
}

pub struct Asset {
    pub block_id: Uuid,
    pub file_id: Uuid,
    pub asset_type: AssetType,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub align: Align,
    pub external_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    Image,
    Pdf,
    Audio,
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}
```

---

### FASE 2: Query System (Semanas 9-14)

#### 2.1 Query DSL Grammar (PEG)

Basado en el Query DSL de Logseq con pest.

```rust
// src/query/dsl/query.pest

query = { SOI ~ expr ~ EOI }

expr = { and | or | not | between | property | task | priority | page | tags | page_ref | self_ref | block_content | sample }

and = { "(" ~ "and" ~ expr+ ~ ")" }
or = { "(" ~ "or" ~ expr+ ~ ")" }
not = { "(" ~ "not" ~ expr ~ ")" }

between = { "(" ~ "between" ~ value ~ value ~ ")" }
property = { "(" ~ "property" ~ string ~ value ~ ")" }
task = { "(" ~ "task" ~ task_marker+ ~ ")" }
priority = { "(" ~ "priority" ~ priority_level+ ~ ")" }
page = { "(" ~ "page" ~ string ~ ")" }
tags = { "(" ~ "tags" ~ string ~ ")" }
page_ref = { "[[" ~ page_name ~ "]]" }
self_ref = { "self" }
block_content = { "(" ~ "full-text-search" ~ string ~ ")" }
sample = { "(" ~ "sample" ~ integer ~ ")" }

task_marker = { "now" | "later" | "todo" | "done" | "cancelled" }
priority_level = { "a" | "b" | "c" }

value = { string | integer | date | time_helper | boolean }
string = { "\"" ~ quoted_string ~ "\"" }
quoted_string = { (!"\"" ~ ANY)* }
integer = { ASCII_DIGIT+ }
date = { integer ~ "-" ~ integer ~ "-" ~ integer }
time_helper = { "-"? ~ integer ~ ("d" | "w" | "m" | "y" | "h" | "n") }
boolean = { "true" | "false" }
page_name = { (!"]]" ~ ANY)+ }
```

#### 2.2 Query Parser + AST

```rust
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "src/query/dsl/query.pest"]
pub struct QueryDslParser;

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
    Tags(String),
    PageRef(String),
    SelfRef,
    BlockContent(String),
    Sample(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyOp {
    Equals,
    NotEquals,
    Contains,
    GreaterThan,
    LessThan,
    Between,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryValue {
    String(String),
    Integer(i64),
    Date(NaiveDate),
    TimeOffset(TimeOffset),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeOffset {
    Days(i64),
    Weeks(i64),
    Months(i64),
    Years(i64),
    Hours(i64),
    Minutes(i64),
}

pub fn parse_query(input: &str) -> Result<QueryExpr, ParseError> {
    let pairs = QueryDslParser::parse(Rule::query, input)?;
    build_ast(pairs)
}

pub fn build_ast(pairs: Pairs<Rule>) -> Result<QueryExpr, ParseError> {
    // Implementación del parser
}
```

#### 2.3 Query Executor (SQLite)

```rust
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
            "SELECT b.*, p.name as page_name \
             FROM blocks b \
             JOIN pages p ON b.page_id = p.id \
             WHERE "
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
                Ok(("(".to_string() + &clauses.iter()
                    .map(|(c, _)| format!("({})", c))
                    .collect::<Vec<_>>()
                    .join(" AND ") + ")"))
            }
            QueryExpr::Or(children) => { /* similar con OR */ }
            QueryExpr::Not(inner) => {
                let (clause, _) = self.build_where(inner, depth + 1)?;
                Ok((format!("NOT ({})", clause)))
            }
            QueryExpr::Property { key, op, value } => {
                let column = format!("json_extract(properties, '$.{}')", key);
                let (cond, param) = self.property_condition(&column, op, value)?;
                Ok((cond, vec![param]))
            }
            QueryExpr::Task(markers) => {
                let marker_list: Vec<_> = markers.iter()
                    .map(|m| format!("'{}'", m.as_str()))
                    .collect();
                Ok((format!("marker IN ({})", marker_list.join(",")), vec![]))
            }
            QueryExpr::BlockContent(q) => {
                Ok((
                    "EXISTS (SELECT 1 FROM blocks_fts WHERE blocks_fts MATCH ? AND blocks_fts.rowid = b.rowid)".to_string(),
                    vec![Param::String(q.clone())]
                ))
            }
            QueryExpr::Sample(n) => {
                Ok((format!("ORDER BY RANDOM() LIMIT {}", n), vec![]))
            }
            // ... más operadores
        }
    }
}
```

#### 2.4 Time Helpers

```rust
impl TimeOffset {
    pub fn to_date(&self, base: NaiveDate) -> NaiveDate {
        match self {
            TimeOffset::Days(n) => base - ChronoDuration::days(*n),
            TimeOffset::Weeks(n) => base - ChronoDuration::weeks(*n),
            TimeOffset::Months(n) => base - ChronoDuration::days(n * 30), // aproximación
            TimeOffset::Years(n) => base - ChronoDuration::days(n * 365),
            TimeOffset::Hours(n) => base - ChronoDuration::hours(*n),
            TimeOffset::Minutes(n) => base - ChronoDuration::minutes(*n),
        }
    }
}

// Helpers predefinidos
pub const TIME_HELPERS: &[(&str, TimeOffset)] = &[
    ("today", TimeOffset::Days(0)),
    ("yesterday", TimeOffset::Days(-1)),
    ("tomorrow", TimeOffset::Days(1)),
    ("-1d", TimeOffset::Days(-1)),
    ("+1d", TimeOffset::Days(1)),
    ("-1w", TimeOffset::Weeks(-1)),
    ("+1w", TimeOffset::Weeks(1)),
    ("-1m", TimeOffset::Months(-1)),
    ("+1m", TimeOffset::Months(1)),
    ("-1y", TimeOffset::Years(-1)),
    ("+1y", TimeOffset::Years(1)),
    ("-1h", TimeOffset::Hours(-1)),
    ("-1n", TimeOffset::Minutes(-1)), // n = minutes en Logseq
];
```

#### 2.5 Property Queries

```rust
// Property types soportados
pub enum PropertyType {
    String,
    Integer,
    Boolean,
    Date,
    Ref, // Referencia a otra entidad
}

impl QueryExecutor {
    fn property_condition(
        &self,
        column: &str,
        op: &PropertyOp,
        value: &QueryValue,
    ) -> Result<(String, Param), QueryError> {
        match op {
            PropertyOp::Equals => {
                Ok((format!("{} = ?", column), Param::from_value(value)))
            }
            PropertyOp::NotEquals => {
                Ok((format!("{} != ?", column), Param::from_value(value)))
            }
            PropertyOp::Contains => {
                Ok((format!("{} LIKE '%' || ? || '%'", column), Param::from_value(value)))
            }
            PropertyOp::GreaterThan => {
                Ok((format!("{} > ?", column), Param::from_value(value)))
            }
            PropertyOp::LessThan => {
                Ok((format!("{} < ?", column), Param::from_value(value)))
            }
            PropertyOp::Between => {
                // value debe ser un array [start, end]
                todo!("Implementar between para properties")
            }
        }
    }
}
```

---

### FASE 3: Search (Semanas 15-18)

#### 3.1 FTS5 Integration

```rust
pub struct SearchService {
    db: SqlitePool,
    cache: Arc<LruCache<String, Vec<SearchResult>>>,
}

impl SearchService {
    pub async fn full_text_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let cache_key = format!("fts:{}:{}", query, limit);

        // Check cache primero
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let sql = r#"
            SELECT b.*, p.name as page_name,
                   snippet(blocks_fts, 0, '<mark>', '</mark>', '...', 32) as snippet,
                   bm25(blocks_fts) as rank
            FROM blocks_fts
            JOIN blocks b ON blocks_fts.rowid = b.rowid
            JOIN pages p ON b.page_id = p.id
            WHERE blocks_fts MATCH ?
            ORDER BY rank
            LIMIT ?
        "#;

        let results = sqlx::query_as::<_, SearchResultRow>(sql)
            .bind(query)
            .bind(limit as i64)
            .fetch_all(&self.db)
            .await?;

        let search_results: Vec<SearchResult> = results.into_iter().map(|r| r.into()).collect();

        // Cachear resultados
        self.cache.put(cache_key, search_results.clone());

        Ok(search_results)
    }

    pub async fn fuzzy_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // Normalizar query
        let normalized = self.normalize_query(query);

        // Buscar en page names y block contents
        let sql = r#"
            SELECT DISTINCT b.*, p.name as page_name,
                   b.content as snippet,
                   0.0 as rank
            FROM blocks b
            JOIN pages p ON b.page_id = p.id
            WHERE p.name LIKE ?
               OR b.content LIKE ?
            ORDER BY p.name
            LIMIT ?
        "#;

        let like_pattern = format!("%{}%", normalized);

        let results = sqlx::query_as::<_, SearchResultRow>(sql)
            .bind(&like_pattern)
            .bind(&like_pattern)
            .bind(limit as i64)
            .fetch_all(&self.db)
            .await?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }

    fn normalize_query(&self, query: &str) -> String {
        query
            .to_lowercase()
            .trim()
            .replace(' ', "%")
    }
}
```

#### 3.2 Search Index Management

```rust
pub struct SearchIndexManager {
    db: SqlitePool,
    rebuild_tx: broadcast::Sender<RebuildRequest>,
}

#[derive(Debug, Clone)]
pub enum RebuildRequest {
    Full,
    Incremental { since: DateTime<Utc> },
    Block { block_id: Uuid },
}

impl SearchIndexManager {
    pub async fn rebuild_full(&self) -> Result<()> {
        tracing::info!("Starting full search index rebuild");

        // Recrear tabla FTS
        sqlx::query("DROP TABLE IF EXISTS blocks_fts")
            .execute(&self.db)
            .await?;

        sqlx::query(r#"
            CREATE VIRTUAL TABLE blocks_fts USING fts5(
                content,
                content=blocks,
                content_rowid=rowid
            )
        "#).execute(&self.db).await?;

        // Reindexar todos los bloques
        let blocks = sqlx::query_as::<_, BlockRow>(
            "SELECT * FROM blocks"
        ).fetch_all(&self.db).await?;

        for block in blocks {
            sqlx::query(
                "INSERT INTO blocks_fts(rowid, content) VALUES (?, ?)"
            )
            .bind(block.rowid)
            .bind(&block.content)
            .execute(&self.db)
            .await?;
        }

        tracing::info!("Full search index rebuild complete: {} blocks indexed", blocks.len());
        Ok(())
    }

    pub async fn rebuild_incremental(&self, since: DateTime<Utc>) -> Result<usize> {
        let blocks = sqlx::query_as::<_, BlockRow>(
            "SELECT * FROM blocks WHERE updated_at > ?"
        )
        .bind(since.timestamp())
        .fetch_all(&self.db)
        .await?;

        for block in &blocks {
            sqlx::query(
                "INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', ?, ?)"
            )
            .bind(block.rowid)
            .bind(&block.content)
            .execute(&self.db)
            .await?;

            sqlx::query(
                "INSERT INTO blocks_fts(rowid, content) VALUES (?, ?)"
            )
            .bind(block.rowid)
            .bind(&block.content)
            .execute(&self.db)
            .await?;
        }

        Ok(blocks.len())
    }
}
```

---

### FASE 4: MCP Layer (Semanas 19-24)

#### 4.1 MCP Server Scaffold

```rust
// src/mcp/server.rs

use mcp_server::{
    McpServer, McpTool, McpResource, McpNotification,
    McpConnection, McpSubscription,
};

#[derive(Default)]
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
        let results = self.search.full_text(&query, limit as usize).await?;
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
        let day = JournalDay::from_ymd_str(&date)
            .map_err(|_| McpError::invalid_param("date"))?;
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
                deadline: deadline.map(|d| JournalDay::from_ymd_str(&d).ok()).flatten(),
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
    #[resource(uri = "logseq://graph", name = "Current Graph", mime = "application/json")]
    async fn graph(&self) -> McpResult<GraphDto> {
        let graph = self.graph.get_current().await?;
        Ok(GraphDto::from(graph))
    }

    /// All pages as a list resource
    #[resource(uri = "logseq://pages", name = "All Pages", mime = "application/json")]
    async fn pages(&self) -> McpResult<Vec<PageDto>> {
        let pages = self.pages.get_all().await?;
        Ok(pages.into_iter().map(PageDto::from).collect())
    }

    /// Specific page resource
    #[resource(uri = "logseq://pages/{name}", name = "Page by Name", mime = "application/json")]
    async fn page_by_name(&self, #[param(name = "name")] name: String) -> McpResult<PageDto> {
        let page = self.pages.get_by_name(&name).await?
            .ok_or(McpError::not_found("Page not found"))?;
        Ok(PageDto::from(page))
    }

    /// Journal page for a date
    #[resource(uri = "logseq://journal/{date}", name = "Journal Page", mime = "application/json")]
    async fn journal_page(&self, #[param(name = "date")] date: String) -> McpResult<PageDto> {
        let day = JournalDay::from_ymd_str(&date)
            .map_err(|_| McpError::invalid_param("date"))?;
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

---

### FASE 5: Sync Engine (Semanas 25-30)

#### 5.1 Loro CRDT Integration

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

    pub fn block_to_crdt_op(&mut self, block: &Block) -> LoroText {
        let text = self.doc.get_text(&block.id.to_string());
        let bytes = rkyv::to_bytes::<_, 256>(block).unwrap();
        text
    }

    pub fn apply_remote_change(&mut self, remote_bytes: &[u8]) -> Result<(), SyncError> {
        self.doc.import(remote_bytes)
            .map_err(|e| SyncError::ImportFailed(e.to_string()))?;

        if let Some(version) = self.doc.get_change_at_latest() {
            self.resolve_conflicts(version)?;
        }

        self.last_synced = Instant::now();
        Ok(())
    }

    fn resolve_conflicts(&mut self, version: LoroVersion) -> Result<(), SyncError> {
        let conflicts = self.doc.get_deep_value();
        for conflict in conflicts.iter() {
            match conflict {
                Conflict::ConcurrentEdit { id, peer_a, peer_b } => {
                    let winner = if peer_a.timestamp > peer_b.timestamp {
                        peer_a
                    } else if peer_a.timestamp == peer_b.timestamp {
                        if peer_a.peer_id > peer_b.peer_id { peer_a } else { peer_b }
                    } else {
                        peer_b
                    };
                    self.doc.apply(&winner.data)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
```

#### 5.2 Offline Queue + WAL

```rust
// src/sync/offline.rs

pub struct OfflineQueue {
    queue: WAL<ChangeRecord>,
    sync_engine: Arc<CrdtSyncEngine>,
}

impl OfflineQueue {
    pub async fn enqueue(&mut self, change: ChangeRecord) -> Result<(), SyncError> {
        self.queue.append(change.clone())?;

        if self.sync_engine.is_online().await {
            if let Err(e) = self.sync_engine.push_change(change).await {
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
                }
            }
        }
        Ok(synced)
    }
}
```

---

### FASE 6: Desktop Shell (Semanas 31-36)

#### 6.1 Tauri App Scaffold

```rust
// src-tauri/main.rs

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let db = BlockService::new(app.path().app_data_dir().unwrap());
            let search = SearchService::new(db.clone());
            let mcp = McpServer::new(db.clone());

            app.manage(db);
            app.manage(search);
            app.manage(mcp);

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

### FASE 7: UI (Semanas 37-44)

#### 7.1 Leptos/Yew Scaffold

**Vistas principales**:

1. **Daily Journal View**: Briefing matutino + editor de blocks
2. **Graph View**: Cognitive map vivo con colores cognitivos
3. **Focus Mode**: Editor con agent panel lateral
4. **Query Builder**: Constructor visual de queries DSL
5. **Agent Room**: Mesa redonda de agentes

#### 7.2 Journal View

```rust
// Vista del journal diario con briefing cognitivo

#[component]
pub fn JournalView(cx: Scope) -> impl IntoView {
    let today = use_state(cx, || Utc::now().date_naive());
    let briefing = create_resource(cx, move || today.get(), |date| async move {
        fetch_briefing(date).await
    });

    view! {
        cx,
        <div class="journal-view">
            <header>
                <h1>{today.get().format("%A, %d %B %Y")}</h1>
            </header>

            <CognitiveBriefing briefing=briefing.get() />

            <TaskList date=today.get() />

            <BlockEditor parent_id=None page_name=today.get().to_string() />
        </div>
    }
}

#[component]
pub fn CognitiveBriefing(cx: Scope, briefing: Option<Briefing>) -> impl IntoView {
    match briefing {
        Some(b) => view! {
            cx,
            <div class="briefing-panel">
                <h2>"Tu Briefing Matutino"</h2>

                <section class="cognitive-pulse">
                    <h3>"Cognitive Pulse"</h3>
                    <p>{b.cognitive_pulse}</p>
                </section>

                <section class="emergencies">
                    <h3>"Emergencias Detectadas"</h3>
                    <For each={b.emergencies}>|e| view! {
                        cx,
                        <EmergencyCard emergency=e />
                    }</For>
                </section>

                <section class="serendipity">
                    <h3>"Conexiones Serendipity"</h3>
                    <For each={b.serendipity}|conn| view! {
                        cx,
                        <SerendipityCard connection=conn />
                    }</For>
                </section>
            </div>
        },
        None => view! { cx, <Loading /> },
    }
}
```

---

### FASE 8: AI Cognitive (Semanas 45-52)

#### 8.1 Cognitive Mirror

```rust
// Análisis del grafo de conocimiento

pub struct CognitiveMirror {
    block_repo: Arc<dyn BlockRepository>,
    page_repo: Arc<dyn PageRepository>,
    ai_client: Arc<AIClient>,
}

pub struct CognitiveMap {
    pub clusters: Vec<KnowledgeCluster>,
    pub density: HashMap<Uuid, f64>,
    pub frontiers: Vec<Uuid>,
    pub gaps: Vec<KnowledgeGap>,
    pub influences: HashMap<Uuid, Vec<Uuid>>,
}

impl CognitiveMirror {
    pub async fn analyze(&self, topic: Option<&str>, depth: u8) -> Result<CognitiveMap> {
        // 1. Extraer todos los bloques
        let blocks = self.block_repo.get_all().await?;

        // 2. Detectar clusters por co-ocurrencia de referencias
        let clusters = self.detect_clusters(&blocks)?;

        // 3. Calcular densidad (número de refs por nodo)
        let density = self.calculate_density(&blocks)?;

        // 4. Identificar frontiers (nodos con muchas refs salientes pero poca profundidad)
        let frontiers = self.detect_frontiers(&blocks)?;

        // 5. Detectar gaps (temas rodeados pero no atacados)
        let gaps = self.detect_gaps(&blocks, topic)?;

        // 6. Map de influencias
        let influences = self.map_influences(&blocks)?;

        Ok(CognitiveMap { clusters, density, frontiers, gaps, influences })
    }
}
```

#### 8.2 Serendipity Engine

```rust
pub struct SerendipityEngine {
    block_repo: Arc<dyn BlockRepository>,
    ai_client: Arc<AIClient>,
}

pub struct SerendipityConnection {
    pub idea_a: Uuid,
    pub idea_b: Uuid,
    pub bridge_concept: String,
    pub confidence: f64,
    pub explanation: String,
}

impl SerendipityEngine {
    pub async fn find_connections(&self, block_id: Option<Uuid>) -> Result<Vec<SerendipityConnection>> {
        let blocks = match block_id {
            Some(id) => vec![self.block_repo.get_by_id(id).await?],
            None => self.block_repo.get_recent(100).await?,
        };

        let mut connections = Vec::new();

        for i in 0..blocks.len() {
            for j in (i+1)..blocks.len() {
                if let Some(conn) = self.check_serendipity(&blocks[i], &blocks[j]).await? {
                    connections.push(conn);
                }
            }
        }

        Ok(connections)
    }

    async fn check_serendipity(&self, a: &Block, b: &Block) -> Result<Option<SerendipityConnection>> {
        // Structural similarity
        let structural_sim = self.structural_similarity(a, b)?;

        // Temporal proximity
        let temporal = self.temporal_proximity(a, b)?;

        // Semantic bridge via AI
        let bridge = self.find_semantic_bridge(a, b).await?;

        if structural_sim > 0.7 || temporal || bridge.is_some() {
            Ok(Some(SerendipityConnection {
                idea_a: a.id,
                idea_b: b.id,
                bridge_concept: bridge.unwrap_or_default(),
                confidence: structural_sim,
                explanation: self.generate_explanation(a, b, &bridge)?,
            }))
        } else {
            Ok(None)
        }
    }
}
```

---

## 3. User Stories Clave a Satisfacer

### 3.1 Búsqueda

| ID | Descripción | Prioridad |
|----|-------------|-----------|
| US-BUS-01 | Buscar página por nombre exacto | P0 |
| US-BUS-02 | Búsqueda difusa (fuzzy) | P0 |
| US-BUS-03 | Búsqueda con acentos configurable | P1 |
| US-BUS-06 | Filtro de tarea (task) | P0 |
| US-BUS-07 | Operadores booleanos (and, or, not) | P0 |
| US-BUS-08 | Operador between (rango de fechas) | P0 |
| US-BUS-09 | Time helpers (today, -7d, +1w) | P0 |
| US-BUS-10 | Filtro de propiedad | P0 |
| US-BUS-11 | Page ref [[page-name]] | P0 |
| US-BUS-12 | Full-text search | P0 |

### 3.2 Bloques y Outliner

| ID | Descripción | Prioridad |
|----|-------------|-----------|
| US-BLK-01 | Crear bloque con contenido | P0 |
| US-BLK-02 | Indentar/dedentar (Tab/Shift+Tab) | P0 |
| US-BLK-03 | Mover bloque (drag & drop) | P0 |
| US-BLK-04 | Crear referencia a otra página [[]] | P0 |
| US-BLK-05 | Marcar tarea (TODO, DONE, etc) | P0 |
| US-BLK-06 | Asignar prioridad (A, B, C) | P1 |
| US-BLK-07 | Agregar scheduled/deadline | P1 |
| US-BLK-08 | Propiedades custom | P1 |

### 3.3 Páginas y Journals

| ID | Descripción | Prioridad |
|----|-------------|-----------|
| US-PAGE-01 | Crear página | P0 |
| US-PAGE-02 | Renombrar página | P0 |
| US-PAGE-03 | Crear journal diario | P0 |
| US-PAGE-04 | Namespace jerárquico | P1 |
| US-PAGE-05 | Tags y aliases | P1 |

### 3.4 Sync

| ID | Descripción | Prioridad |
|----|-------------|-----------|
| US-SYNC-01 | Sync local-first | P0 |
| US-SYNC-02 | Offline queue | P0 |
| US-SYNC-03 | Conflict resolution (CRDT) | P1 |
| US-SYNC-04 | E2E encryption | P2 |

---

## 4. Comparativa con Logseq Original

| Feature | Logseq (ClojureScript) | Quilt (Rust) |
|---------|-------------------------|---------------|
| Database | DataScript (in-memory) | SQLite (persistent) |
| Query | Datomic-like DSL | PEG parser + SQL |
| Sync | API REST +txid | CRDT (Loro) |
| Search | Browser native | FTS5 |
| Platform | Electron | Tauri |
| AI | Plugins (later) | Native MCP |
| Performance | Good | Excellent |
| Memory | Higher | Lower |

---

## 5. Métricas de Éxito

### Fase 1 (Foundation)
- [ ] Schema SQLite con 10 tablas creado
- [ ] Migrations system funcional
- [ ] Block CRUD pasa 20 unit tests
- [ ] Page CRUD pasa 15 unit tests

### Fase 2 (Query)
- [ ] Parser acepta 100% de queries válidas de Logseq
- [ ] Parser rechaza 100% de queries inválidas
- [ ] Query execution < 50ms para 10k bloques

### Fase 3 (Search)
- [ ] FTS5 indexing completo
- [ ] Search < 100ms para 10k bloques
- [ ] Fuzzy search tolerancia a typos

### Fase 4 (MCP)
- [ ] 10 tools funcionales
- [ ] 4 resource types implementados
- [ ] 3 notifications funcionando
- [ ] MCP conformance test suite passing

### Fase 5 (Sync)
- [ ] CRDT merge sin pérdida de datos
- [ ] Offline queue persiste 1000+ ops
- [ ] Conflict resolution automático

### Fase 6 (Desktop)
- [ ] Tauri build exitoso
- [ ] Deep links funcionan
- [ ] File watcher detecta cambios

---

## 6. Riesgos y Mitigaciones

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|--------------|---------|------------|
| Query DSL complejidad | Alta | Alta | Gramática formal, tests exhaustivos |
| CRDT merge conflicts | Media | Alta | Loro handlea la mayoría |
| FTS5 performance | Media | Media | Índices incrementales |
| MCP protocol evolution | Baja | Media | Usar spec oficial |
| Rust WASM compatibility | Baja | Alta | Leptos tiene buen track record |

---

## 7. Timeline Visual

```
2026          Q2          Q3          Q4
─────────────────────────────────────────────────
FASE 0-3  ████████
FASE 4    ░░░░████
FASE 5    ░░░░░░████
FASE 6    ░░░░░░░░████
FASE 7    ░░░░░░░░░░████████
FASE 8    ░░░░░░░░░░░░░░░░██████
FASE 9    ░░░░░░░░░░░░░░░░░░░░░████
```

**Total estimado**: 56 semanas (~14 meses)

---

## 8. Recursos Necesarios

### Dependencies Externas

```toml
# Database
sqlx = "0.8"
rkyv = "0.8"

# Async
tokio = "1"
async-trait = "0.1"

# Query
pest = "2.7"
pest_derive = "2.7"

# Sync (CRDT)
loro = "0.2"

# Serialization
serde = "1"
serde_json = "1"
uuid = "1"
chrono = "0.4"

# Error handling
thiserror = "1"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Desktop
tauri = "2"

# CLI
clap = "4"

# Cache
lru = "0.13"

# Search
tantivy = "0.22"  # Alternativa a FTS5 para casos avanzados
```

---

*Documento generado basándose en investigación de Logseq DB*
*Versión: 1.0 | Fecha: 2026-05-02*
