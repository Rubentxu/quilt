# Quilt — Arquitectura DDD con Crates por Bounded Context

> **Proyecto**: Quilt — Logseq DB Clone en Rust
> **Fecha**: 2026-05-02
> **Versión**: 2.0
> **Principios**: DDD + SOLID + Clean Architecture

---

## 1. Visión y Principios

### 1.1 Principios DDD

```
1. each crate is a BOUNDED CONTEXT
2. dependencies flow DOWNWARD (domain → application → infrastructure)
3. domain crate has ZERO external dependencies
4. each crate exposes its API via lib.rs (single public interface)
5. no cyclic dependencies between crates
```

### 1.2 Principios SOLID Aplicados

| Principio | Aplicación en Quilt |
|-----------|---------------------|
| **S**ingle Responsibility | Cada crate tiene una sola razón para cambiar |
| **O**pen/Closed | Entidades abiertas a extensión, cerradas a modificación |
| **L**iskov Substitution | Traits como abstracciones intercambiables |
| **I**nterface Segregation | Interfaces pequeñas y enfocadas por contexto |
| **D**ependency Inversion | Depender de abstracciones, no de concreciones |

### 1.3 Dependence Rule

```
┌─────────────────────────────────────────────────────────────┐
│                    INFRASTRUCTURE                             │
│   (Tauri, CLI, external services, DB implementations)        │
└─────────────────────────────┬───────────────────────────────┘
                              │ implements
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    APPLICATION                               │
│   (Use cases, commands, queries, orchestration)            │
└─────────────────────────────┬───────────────────────────────┘
                              │ uses
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      DOMAIN                                 │
│   (Entities, value objects, domain services, traits)        │
└─────────────────────────────────────────────────────────────┘

Domain ← Application ← Infrastructure
```

---

## 2. Arquitectura de Crates

### 2.1 Mapa de Bounded Contexts

```
quilt/
├── Cargo.toml                 # Workspace root - dependencies centralizadas
├── crates/
│   ├── quilt-domain/          # 🟢 Pure domain - ZERO dependencies
│   ├── quilt-application/     # 🟡 Application services - depends on domain
│   ├── quilt-infrastructure/  # 🔴 Infrastructure - implements domain traits
│   ├── quilt-query/          # Query DSL parsing + execution
│   ├── quilt-search/         # Full-text search
│   ├── quilt-sync/           # CRDT sync engine
│   ├── quilt-mcp/            # MCP protocol layer
│   └── quilt-platform/        # Tauri + CLI adapters
├── src/
│   └── main.rs               # Binary entry point
└── tests/
```

### 2.2 Dependency Graph

```
                        ┌──────────────────┐
                        │ quilt-application │
                        └────────┬─────────┘
                                 │
           ┌─────────────────────┼─────────────────────┐
           │                     │                     │
           ▼                     ▼                     ▼
    ┌─────────────┐      ┌─────────────┐      ┌─────────────┐
    │quilt-domain │      │quilt-query  │      │quilt-search │
    └─────────────┘      └─────────────┘      └─────────────┘
           ▲                     │                     │
           │                     │                     │
           │                     ▼                     │
           │              ┌─────────────┐              │
           │              │quilt-domain│◄─────────────┘
           │              └─────────────┘
           │                     ▲
           │                     │
           └─────────────────────┤
                                 │
                        ┌────────┴────────┐
                        │quilt-infrastructure│
                        └────────┬─────────┘
                                 │
                                 ▼
                        ┌──────────────────┐
                        │  quilt-platform  │
                        └──────────────────┘
```

---

## 3. Detalle de Cada Crate

### 3.1 `quilt-domain` — Core Domain

**Responsabilidad**: Entidades puras, value objects, traits de dominio, reglas de negocio.

**Principios**:
- **CERO dependencias externas**
- Solo types + traits
- Sin implementación de persistencia
- Sin async (el dominio no sabe de async)

**Estructura**:
```
quilt-domain/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── entities/
    │   ├── mod.rs
    │   ├── block.rs
    │   ├── page.rs
    │   ├── journal.rs
    │   ├── tag.rs
    │   ├── file.rs
    │   └── asset.rs
    ├── value_objects/
    │   ├── mod.rs
    │   ├── journal_day.rs
    │   ├── block_format.rs
    │   ├── task_marker.rs
    │   ├── priority.rs
    │   ├── property_value.rs
    │   └── uuid.rs
    ├── repositories/
    │   ├── mod.rs
    │   ├── block_repository.rs
    │   ├── page_repository.rs
    │   ├── tag_repository.rs
    │   └── file_repository.rs
    ├── services/
    │   ├── mod.rs
    │   ├── block_service.rs
    │   ├── page_service.rs
    │   └── outliner_service.rs
    ├── events/
    │   ├── mod.rs
    │   └── domain_events.rs
    └── errors/
        ├── mod.rs
        └── domain_error.rs
```

**Entidades Principales**:

```rust
// quilt-domain/src/entities/block.rs

use crate::value_objects::{BlockFormat, TaskMarker, Priority, JournalDay};
use crate::repositories::BlockRepository;

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub id: Uuid,
    pub page_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub order: f64,                    // Lexicographic order
    pub level: u8,
    pub format: BlockFormat,
    pub marker: Option<TaskMarker>,
    pub priority: Option<Priority>,
    pub content: String,
    pub properties: HashMap<String, PropertyValue>,
    pub refs: Vec<Uuid>,
    pub tags: Vec<String>,
    pub scheduled: Option<DateTime<Utc>>,
    pub deadline: Option<DateTime<Utc>>,
    pub collapsed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Block {
    /// Regla: UUID no puede cambiar una vez creado
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    /// Regla: No se puede mover a sus propios descendientes
    pub fn can_move_to(&self, new_parent: Option<Uuid>, all_blocks: &[Block]) -> bool {
        if self.id == new_parent {
            return false;
        }

        if let Some(parent_id) = new_parent {
            // Check circular reference
            self.is_descendant_of(parent_id, all_blocks)
        } else {
            true
        }
    }

    fn is_descendant_of(&self, target_id: Uuid, blocks: &[Block]) -> bool {
        blocks.iter()
            .filter(|b| b.parent_id == Some(self.id))
            .any(|b| b.id == target_id || b.is_descendant_of(target_id, blocks))
    }
}
```

**Traits de Repositorio (Abstracciones)**:

```rust
// quilt-domain/src/repositories/block_repository.rs

use crate::entities::{Block, Page};
use crate::value_objects::JournalDay;

/// Trait para repositorio de bloques - ABSTRACCIÓN, no implementación
pub trait BlockRepository: Send + Sync {
    fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, DomainError>;
    fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError>;
    fn get_children(&self, parent_id: Uuid) -> Result<Vec<Block>, DomainError>;
    fn get_with_refs(&self, id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError>;
    fn insert(&self, block: &Block) -> Result<(), DomainError>;
    fn update(&self, block: &Block) -> Result<(), DomainError>;
    fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    fn move_block(&self, id: Uuid, new_parent: Option<Uuid>, new_order: f64) -> Result<(), DomainError>;
}

pub trait PageRepository: Send + Sync {
    fn get_by_id(&self, id: Uuid) -> Result<Option<Page>, DomainError>;
    fn get_by_name(&self, name: &str) -> Result<Option<Page>, DomainError>;
    fn get_journal(&self, day: JournalDay) -> Result<Option<Page>, DomainError>;
    fn get_all(&self) -> Result<Vec<Page>, DomainError>;
    fn get_namespace_pages(&self, namespace_id: Uuid) -> Result<Vec<Page>, DomainError>;
    fn create(&self, page: &Page) -> Result<(), DomainError>;
    fn rename(&self, id: Uuid, new_name: &str) -> Result<(), DomainError>;
    fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}

pub trait TagRepository: Send + Sync {
    fn get_by_page(&self, page_id: Uuid) -> Result<Vec<String>, DomainError>;
    fn get_pages_with_tag(&self, tag: &str) -> Result<Vec<Uuid>, DomainError>;
    fn add_tag(&self, page_id: Uuid, tag: &str) -> Result<(), DomainError>;
    fn remove_tag(&self, page_id: Uuid, tag: &str) -> Result<(), DomainError>;
}
```

**Domain Services**:

```rust
// quilt-domain/src/services/outliner_service.rs

use crate::entities::Block;
use crate::errors::DomainError;

/// Servicio de dominio para lógica de outliner
/// Sin dependencias - solo opera sobre entidades
pub struct OutlinerService;

impl OutlinerService {
    /// Calcula el nuevo orden lexicográfico para un bloque entre siblings
    pub fn calculate_order(sibling_orders: &[f64], position: usize) -> f64 {
        if sibling_orders.is_empty() {
            return 1.0;
        }

        if position == 0 {
            sibling_orders[0] / 2.0
        } else if position >= sibling_orders.len() {
            sibling_orders[sibling_orders.len() - 1] + 1.0
        } else {
            (sibling_orders[position - 1] + sibling_orders[position]) / 2.0
        }
    }

    /// Reordena todos los bloques hijos después de un movimiento
    pub fn rebalance_children(children: &mut [Block]) {
        for (i, child) in children.iter_mut().enumerate() {
            child.order = (i as f64 + 1) * 100.0;
        }
    }

    /// Valida que el movimiento no viola reglas de negocio
    pub fn validate_move(
        block: &Block,
        new_parent: Option<Uuid>,
        all_blocks: &[Block],
    ) -> Result<(), DomainError> {
        if !block.can_move_to(new_parent, all_blocks) {
            return Err(DomainError::CircularReference(block.id));
        }
        Ok(())
    }
}
```

**Value Objects**:

```rust
// quilt-domain/src/value_objects/journal_day.rs

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JournalDay(i32); // YYYYMMDD

impl JournalDay {
    pub fn from_ymd(year: u16, month: u8, day: u8) -> Option<Self> {
        if month >= 1 && month <= 12 && day >= 1 && day <= 31 {
            Some(JournalDay(
                (year as i32) * 10000 + (month as i32) * 100 + (day as i32)
            ))
        } else {
            None
        }
    }

    pub fn from_i32(value: i32) -> Option<Self> {
        let year = value / 10000;
        let month = (value % 10000) / 100;
        let day = value % 100;
        Self::from_ymd(year as u16, month as u8, day as u8)
    }

    pub fn as_int(&self) -> i32 { self.0 }
    pub fn year(&self) -> i32 { self.0 / 10000 }
    pub fn month(&self) -> i32 { (self.0 % 10000) / 100 }
    pub fn day(&self) -> i32 { self.0 % 100 }
}

impl fmt::Display for JournalDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year(), self.month(), self.day())
    }
}

impl FromStr for JournalDay {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse YYYY-MM-DD
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err(DomainError::InvalidJournalDay(s.to_string()));
        }
        let year: u16 = parts[0].parse().map_err(|_| DomainError::InvalidJournalDay(s.to_string()))?;
        let month: u8 = parts[1].parse().map_err(|_| DomainError::InvalidJournalDay(s.to_string()))?;
        let day: u8 = parts[2].parse().map_err(|_| DomainError::InvalidJournalDay(s.to_string()))?;
        Self::from_ymd(year, month, day).ok_or_else(|| DomainError::InvalidJournalDay(s.to_string()))
    }
}
```

---

### 3.2 `quilt-application` — Application Layer

**Responsabilidad**: Casos de uso, commands, queries, orquestación de servicios de dominio.

**Depende de**: `quilt-domain` (solo traits, no implementación)

**Principios**:
- Orchestrates domain logic
- No business rules here (those are in domain)
- Use Cases pattern
- Commands and Queries separated (CQRS)

**Estructura**:
```
quilt-application/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── commands/
    │   ├── mod.rs
    │   ├── block_commands.rs
    │   ├── page_commands.rs
    │   └── sync_commands.rs
    ├── queries/
    │   ├── mod.rs
    │   ├── block_queries.rs
    │   └── page_queries.rs
    ├── handlers/
    │   ├── mod.rs
    │   ├── command_handler.rs
    │   └── query_handler.rs
    └── services/
        ├── mod.rs
        ├── block_service.rs
        └── page_service.rs
```

**Commands (CQRS - Write)**:

```rust
// quilt-application/src/commands/block_commands.rs

use quilt_domain::entities::{Block, BlockCreate, BlockUpdate};
use quilt_domain::repositories::BlockRepository;
use crate::errors::ApplicationError;

#[derive(Debug)]
pub struct CreateBlockCommand {
    pub page_id: Uuid,
    pub content: String,
    pub parent_id: Option<Uuid>,
    pub order: Option<f64>,
    pub marker: Option<TaskMarker>,
}

pub struct BlockCommandHandler<R: BlockRepository> {
    repository: Arc<R>,
}

impl<R: BlockRepository> BlockCommandHandler<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn handle_create(&self, cmd: CreateBlockCommand) -> Result<Block, ApplicationError> {
        let block = Block::create(BlockCreate {
            page_id: cmd.page_id,
            content: cmd.content,
            parent_id: cmd.parent_id,
            order: cmd.order.unwrap_or_else(|| self.calculate_next_order(&cmd)),
            marker: cmd.marker,
            format: BlockFormat::Markdown,
        })?;

        self.repository.insert(&block)?;
        Ok(block)
    }

    pub async fn handle_move(&self, block_id: Uuid, new_parent: Option<Uuid>, position: usize) -> Result<(), ApplicationError> {
        let block = self.repository.get_by_id(block_id)?
            .ok_or(ApplicationError::NotFound("Block", block_id))?;

        let siblings = match new_parent {
            Some(pid) => self.repository.get_children(pid)?,
            None => self.repository.get_by_page(block.page_id)?,
        };

        let new_order = OutlinerService::calculate_order(
            &siblings.iter().map(|b| b.order).collect::<Vec<_>>(),
            position,
        );

        self.repository.move_block(block_id, new_parent, new_order)?;
        Ok(())
    }

    fn calculate_next_order(&self, cmd: &CreateBlockCommand) -> f64 {
        let siblings = self.repository.get_by_page(cmd.page_id).unwrap_or_default();
        siblings.len() as f64 * 100.0 + 1.0
    }
}
```

**Queries (CQRS - Read)**:

```rust
// quilt-application/src/queries/block_queries.rs

use quilt_domain::entities::{Block, BlockWithPage};

#[derive(Debug)]
pub struct GetBlockQuery {
    pub block_id: Uuid,
    pub include_children: bool,
    pub include_backlinks: bool,
}

#[derive(Debug)]
pub struct GetPageBlocksQuery {
    pub page_name: String,
    pub format: Option<BlockFormat>,
}

pub struct BlockQueryHandler<R: BlockRepository> {
    repository: Arc<R>,
}

impl<R: BlockRepository> BlockQueryHandler<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn handle_get(&self, query: GetBlockQuery) -> Result<Option<BlockWithPage>, ApplicationError> {
        let (block, refs) = self.repository.get_with_refs(query.block_id)?;

        let children = if query.include_children {
            self.repository.get_children(query.block_id)?
        } else {
            vec![]
        };

        let backlinks = if query.include_backlinks {
            self.repository.get_backlinks(query.block_id)?
        } else {
            vec![]
        };

        Ok(Some(BlockWithPage {
            block,
            refs,
            children,
            backlinks,
        }))
    }

    pub async fn handle_get_page_blocks(&self, query: GetPageBlocksQuery) -> Result<Vec<Block>, ApplicationError> {
        let page = self.repository.find_page_by_name(&query.page_name)?
            .ok_or_else(|| ApplicationError::NotFound("Page", query.page_name.clone()))?;

        let mut blocks = self.repository.get_by_page(page.id)?;

        // Filtrar por formato si se especifica
        if let Some(format) = query.format {
            blocks.retain(|b| b.format == format);
        }

        // Ordenar por orden lexicográfico
        blocks.sort_by(|a, b| a.order.partial_cmp(&b.order).unwrap());

        Ok(blocks)
    }
}
```

---

### 3.3 `quilt-infrastructure` — Infrastructure Layer

**Responsabilidad**: Implementación concreta de los traits de dominio, persistencia SQLite, adapters externos.

**Depende de**: `quilt-domain` (implementa sus traits)

**Principios**:
- Implements domain traits
- Contains all external dependencies (SQLx, etc.)
- No business logic here

**Estructura**:
```
quilt-infrastructure/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── database/
    │   ├── mod.rs
    │   ├── sqlite/
    │   │   ├── mod.rs
    │   │   ├── connection.rs
    │   │   ├── block_repository.rs
    │   │   ├── page_repository.rs
    │   │   └── migrations.rs
    │   └── repositories/
    │       └── sqlite_repositories.rs
    ├── serialization/
    │   ├── mod.rs
    │   └── rkyv_adapter.rs
    └── logging/
        ├── mod.rs
        └── tracing_adapter.rs
```

**Implementación de Repository**:

```rust
// quilt-infrastructure/src/database/sqlite/block_repository.rs

use quilt_domain::entities::Block;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::errors::DomainError;
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct SqliteBlockRepository {
    pool: SqlitePool,
}

impl SqliteBlockRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BlockRepository for SqliteBlockRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, DomainError> {
        let row = sqlx::query_as::<_, BlockRow>(
            "SELECT * FROM blocks WHERE id = ?"
        )
        .bind(id.as_bytes())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query_as::<_, BlockRow>(
            "SELECT * FROM blocks WHERE page_id = ? ORDER BY order"
        )
        .bind(page_id.as_bytes())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn insert(&self, block: &Block) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO blocks (id, page_id, parent_id, "order", level, format,
                marker, priority, content, properties, scheduled, deadline,
                collapsed, created_at, updated_at, refs, tags)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(block.id.as_bytes())
        .bind(block.page_id.as_bytes())
        .bind(block.parent_id.map(|id| id.as_bytes()))
        .bind(block.order)
        .bind(block.level)
        .bind(block.format.as_str())
        .bind(block.marker.map(|m| m.as_str()))
        .bind(block.priority.map(|p| p.as_str()))
        .bind(&block.content)
        .bind(serde_json::to_vec(&block.properties).unwrap())
        .bind(block.scheduled.map(|dt| dt.timestamp()))
        .bind(block.deadline.map(|dt| dt.timestamp()))
        .bind(block.collapsed)
        .bind(block.created_at.timestamp())
        .bind(block.updated_at.timestamp())
        .bind(serde_json::to_vec(&block.refs).unwrap())
        .bind(serde_json::to_vec(&block.tags).unwrap())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update(&self, block: &Block) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE blocks SET
                parent_id = ?, "order" = ?, level = ?, format = ?,
                marker = ?, priority = ?, content = ?, properties = ?,
                scheduled = ?, deadline = ?, collapsed = ?, updated_at = ?,
                refs = ?, tags = ?
            WHERE id = ?
            "#,
        )
        .bind(block.parent_id.map(|id| id.as_bytes()))
        .bind(block.order)
        .bind(block.level)
        .bind(block.format.as_str())
        .bind(block.marker.map(|m| m.as_str()))
        .bind(block.priority.map(|p| p.as_str()))
        .bind(&block.content)
        .bind(serde_json::to_vec(&block.properties).unwrap())
        .bind(block.scheduled.map(|dt| dt.timestamp()))
        .bind(block.deadline.map(|dt| dt.timestamp()))
        .bind(block.collapsed)
        .bind(Utc::now().timestamp())
        .bind(serde_json::to_vec(&block.refs).unwrap())
        .bind(serde_json::to_vec(&block.tags).unwrap())
        .bind(block.id.as_bytes())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM blocks WHERE id = ?")
            .bind(id.as_bytes())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(e.to_string()))?;
        Ok(())
    }

    async fn move_block(&self, id: Uuid, new_parent: Option<Uuid>, new_order: f64) -> Result<(), DomainError> {
        sqlx::query("UPDATE blocks SET parent_id = ?, \"order\" = ?, updated_at = ? WHERE id = ?")
            .bind(new_parent.map(|id| id.as_bytes()))
            .bind(new_order)
            .bind(Utc::now().timestamp())
            .bind(id.as_bytes())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(e.to_string()))?;
        Ok(())
    }
}
```

---

### 3.4 `quilt-query` — Query DSL

**Responsabilidad**: Parser del Query DSL, AST, executor contra repositorios.

**Depende de**: `quilt-domain` (usa entidades y traits)

**Estructura**:
```
quilt-query/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── grammar/
    │   ├── mod.rs
    │   └── query.pest
    ├── parser/
    │   ├── mod.rs
    │   ├── ast.rs
    │   └── parser.rs
    ├── executor/
    │   ├── mod.rs
    │   ├── context.rs
    │   └── executor.rs
    └── time_helpers.rs
```

**Parser con Pest**:

```rust
// quilt-query/src/grammar/query.pest

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

```rust
// quilt-query/src/parser/parser.rs

use pest::Parser;
use quilt_query::grammar::QueryParser;
use quilt_query::parser::ast::{QueryExpr, QueryValue};

pub struct QueryParser;

impl QueryParser {
    pub fn parse(&self, input: &str) -> Result<QueryExpr, ParseError> {
        let pairs = QueryParser::parse(QueryRule::query, input)
            .map_err(|e| ParseError::Syntax(e.to_string()))?;
        self.build_ast(pairs)
    }

    fn build_ast(&self, pairs: Pairs<Rule>) -> Result<QueryExpr, ParseError> {
        let mut pairs = pairs;
        let pair = pairs.next().unwrap();
        self.expr_to_ast(pair)
    }

    fn expr_to_ast(&self, pair: Pair<Rule>) -> Result<QueryExpr, ParseError> {
        match pair.as_rule() {
            Rule::and => {
                let mut children = Vec::new();
                for p in pair.into_inner() {
                    if p.as_rule() != Rule::and {
                        children.push(self.expr_to_ast(p)?);
                    }
                }
                Ok(QueryExpr::And(children))
            }
            Rule::or => { /* similar */ }
            Rule::task => {
                let markers: Vec<_> = pair.into_inner()
                    .map(|p| p.as_str().into())
                    .collect();
                Ok(QueryExpr::Task(markers))
            }
            Rule::page_ref => {
                let page_name = pair.into_inner().as_str();
                Ok(QueryExpr::PageRef(page_name.to_string()))
            }
            Rule::block_content => {
                let content = pair.into_inner().as_str();
                Ok(QueryExpr::BlockContent(content.to_string()))
            }
            // ... más casos
        }
    }
}
```

---

### 3.5 `quilt-search` — Full-Text Search

**Responsabilidad**: Indexación FTS5, búsqueda difusa, ranking.

**Depende de**: `quilt-domain`

**Estructura**:
```
quilt-search/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── indexing/
    │   ├── mod.rs
    │   ├── fts5_index.rs
    │   └── index_manager.rs
    ├── search/
    │   ├── mod.rs
    │   ├── fuzzy_search.rs
    │   └── result.rs
    └── cache/
        ├── mod.rs
        └── search_cache.rs
```

---

### 3.6 `quilt-sync` — CRDT Sync Engine

**Responsabilidad**: Loro CRDT integration, offline queue, conflict resolution.

**Depende de**: `quilt-domain`

**Estructura**:
```
quilt-sync/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── crdt/
    │   ├── mod.rs
    │   ├── loro_integration.rs
    │   └── conflict_resolver.rs
    ├── offline/
    │   ├── mod.rs
    │   ├── wal.rs
    │   └── queue.rs
    └── state/
        ├── mod.rs
        └── sync_state.rs
```

---

### 3.7 `quilt-mcp` — MCP Protocol Layer

**Responsabilidad**: MCP server, tools, resources, notifications.

**Depende de**: `quilt-domain`, `quilt-application`

**Estructura**:
```
quilt-mcp/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── server/
    │   ├── mod.rs
    │   ├── server.rs
    │   └── connection.rs
    ├── tools/
    │   ├── mod.rs
    │   ├── query_tool.rs
    │   ├── block_tool.rs
    │   └── search_tool.rs
    ├── resources/
    │   ├── mod.rs
    │   ├── graph_resource.rs
    │   └── page_resource.rs
    └── notifications/
        ├── mod.rs
        └── notification.rs
```

---

### 3.8 `quilt-platform` — Platform Adapters

**Responsabilidad**: Tauri commands, CLI, deep links, system tray.

**Depende de**: `quilt-domain`, `quilt-application`, `quilt-mcp`

**Estructura**:
```
quilt-platform/
├── Cargo.toml
├── tauri/
│   ├── mod.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── block_commands.rs
│   │   └── query_commands.rs
│   └── main.rs
└── cli/
    ├── mod.rs
    └── main.rs
```

---

## 4. Cargo Workspace Root

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
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
authors = ["Quilt Team"]
license = "MIT OR Apache-2.0"
rust-version = "1.75"

[workspace.dependencies]
# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate", "uuid", "chrono"] }
rkyv = { version = "0.8", features = ["validation"] }

# Async
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Query DSL
pest = "2.7"
pest_derive = "2.7"

# Sync (CRDT)
loro = "0.2"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }

# Date/Time
chrono = { version = "0.4", features = ["serde"] }

# Error handling
thiserror = "1"
anyhow = "1"

# Logging & Tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Metrics
metrics = "0.23"
metrics-exporter-prometheus = "0.15"

# Cache
lru = "0.13"

# CLI
clap = { version = "4", features = ["derive"] }

# Platform
tauri = { version = "2", features = ["build"] }

# Utils
once_cell = "1"
parking_lot = "0.12"

[workspace.lints.rust]
unsafe_code = "forbid"

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
```

---

## 5. Crate Cargo.tomls Individuales

### quilt-domain/Cargo.toml (CERO deps externos)

```toml
[package]
name = "quilt-domain"
version.workspace = true
edition.workspace = true

[dependencies]
# ZERO external dependencies - domain is pure Rust
```

### quilt-application/Cargo.toml

```toml
[package]
name = "quilt-application"
version.workspace = true
edition.workspace = true

[dependencies]
quilt-domain = { path = "../quilt-domain" }
anyhow = { workspace = true }
```

### quilt-infrastructure/Cargo.toml

```toml
[package]
name = "quilt-infrastructure"
version.workspace = true
edition.workspace = true

[dependencies]
quilt-domain = { path = "../quilt-domain" }
sqlx = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
```

### quilt-query/Cargo.toml

```toml
[package]
name = "quilt-query"
version.workspace = true
edition.workspace = true

[dependencies]
quilt-domain = { path = "../quilt-domain" }
pest = { workspace = true }
pest_derive = { workspace = true }
chrono = { workspace = true }
anyhow = { workspace = true }
```

---

## 6. Estructura Completa del Proyecto

```
quilt/
├── Cargo.toml                      # Workspace root
├── src/
│   └── main.rs                    # Binary entry
├── crates/
│   ├── quilt-domain/              # Bounded Context: Domain Core
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── entities/
│   │       ├── value_objects/
│   │       ├── repositories/
│   │       ├── services/
│   │       ├── events/
│   │       └── errors/
│   │
│   ├── quilt-application/         # Bounded Context: Application
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── commands/
│   │       ├── queries/
│   │       ├── handlers/
│   │       └── services/
│   │
│   ├── quilt-infrastructure/      # Bounded Context: Infrastructure
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── database/
│   │       ├── serialization/
│   │       └── logging/
│   │
│   ├── quilt-query/               # Bounded Context: Query DSL
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── grammar/
│   │       ├── parser/
│   │       ├── executor/
│   │       └── time_helpers.rs
│   │
│   ├── quilt-search/              # Bounded Context: Search
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── indexing/
│   │       ├── search/
│   │       └── cache/
│   │
│   ├── quilt-sync/                # Bounded Context: Sync
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── crdt/
│   │       ├── offline/
│   │       └── state/
│   │
│   ├── quilt-mcp/                 # Bounded Context: MCP Protocol
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server/
│   │       ├── tools/
│   │       ├── resources/
│   │       └── notifications/
│   │
│   └── quilt-platform/            # Bounded Context: Platform
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── tauri/
│           └── cli/
│
└── tests/
    ├── domain_tests/
    ├── application_tests/
    └── integration_tests/
```

---

## 7. Principios SOLID Aplicados por Crate

### Single Responsibility por Bounded Context

| Crate | Responsabilidad | Razón para cambiar |
|-------|----------------|-------------------|
| `quilt-domain` | Entidades + reglas de negocio | Cambian las reglas del dominio |
| `quilt-application` | Orquestación de use cases | Cambian los flujos de aplicación |
| `quilt-infrastructure` | Persistencia + external services | Cambian las tecnologías |
| `quilt-query` | Parseo y ejecución de queries | Cambia el lenguaje de queries |
| `quilt-search` | Indexación y búsqueda | Cambian los algoritmos de búsqueda |
| `quilt-sync` | Sync y CRDT | Cambian los protocolos de sync |
| `quilt-mcp` | Protocolo MCP | Cambia el spec del protocolo |
| `quilt-platform` | Adaptadores de plataforma | Cambian los requisitos de plataforma |

### Dependency Inversion

```rust
// quilt-application/src/services/block_service.rs

use quilt_domain::repositories::BlockRepository; // Abstracción
use quilt_domain::entities::Block;

pub struct BlockService<R: BlockRepository> {
    repository: Arc<R>, // Depende de abstracción, no de concreción
}

impl<R: BlockRepository> BlockService<R> {
    pub async fn create_block(&self, cmd: CreateBlockCommand) -> Result<Block, AppError> {
        // Usa el trait, no la implementación concreta
        let block = Block::new(/* ... */);
        self.repository.insert(&block).await?;
        Ok(block)
    }
}
```

### Interface Segregation

```rust
// Los traits son pequeños y enfocados

pub trait BlockRepository: Send + Sync {
    // Solo operaciones de bloques
    fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, Error>;
    fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, Error>;
    fn insert(&self, block: &Block) -> Result<(), Error>;
    // ... no hay métodos de Page aquí
}
```

---

## 8. Flujo de Dependencias

### Crear un Bloque (Write Path)

```
User → quilt-platform (Tauri command)
    → quilt-application (BlockCommandHandler::handle_create)
        → quilt-domain (Block::create, validates business rules)
            → Return Block
        ← quilt-infrastructure (SqliteBlockRepository::insert)
            → SQLite
```

### Buscar con Query DSL (Read Path)

```
User → quilt-platform (Tauri command)
    → quilt-application (QueryHandler::handle_query)
        → quilt-query (QueryParser::parse, QueryExecutor::execute)
            → quilt-domain (Block entity for mapping)
            → quilt-infrastructure (SqliteBlockRepository for raw SQL)
                → SQLite
```

### Sync (CRDT)

```
Remote changes → quilt-sync (CrdtSyncEngine::apply_remote)
    → quilt-infrastructure (deserialize)
        → quilt-domain (Block entity)
    ← quilt-infrastructure (SqliteBlockRepository::update)
```

---

## 9. Tests por Capa

```
tests/
├── domain_tests/              # Test entities sin dependencias
│   ├── block_tests.rs
│   ├── page_tests.rs
│   └── outliner_tests.rs
│
├── application_tests/         # Test use cases con mocks
│   ├── block_command_tests.rs
│   └── page_query_tests.rs
│
├── infrastructure_tests/     # Test implementaciones SQLite
│   └── sqlite_block_repository_tests.rs
│
├── query_tests/              # Test parser y executor
│   ├── parser_tests.rs
│   └── executor_tests.rs
│
└── integration_tests/        # Test end-to-end
    ├── full_crud_tests.rs
    └── sync_tests.rs
```

---

## 10. Errores Tipificados por Capa

```rust
// quilt-domain/src/errors/domain_error.rs

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Block not found: {0}")]
    BlockNotFound(Uuid),

    #[error("Circular reference detected for block: {0}")]
    CircularReference(Uuid),

    #[error("Invalid journal day format: {0}")]
    InvalidJournalDay(String),

    #[error("Cannot delete block with children")]
    BlockHasChildren,

    // ... más errores de dominio
}

// quilt-application/src/errors/application_error.rs

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Not found: {entity} with id {id}")]
    NotFound(&'static str, Uuid),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    // ... más errores de aplicación
}

// quilt-infrastructure/src/errors/infrastructure_error.rs

#[derive(Debug, thiserror::Error)]
pub enum InfrastructureError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Connection error: {0}")]
    Connection(String),
}
```

---

## 11. Roadmap Actualizado con Arquitectura DDD

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

FASE 2: Query DSL (Semanas 11-16)
├── 2.1 quilt-query: grammar + parser
├── 2.2 quilt-query: AST
├── 2.3 quilt-query: executor
├── 2.4 Time helpers
└── 2.5 Query integration tests

FASE 3: Search (Semanas 17-20)
├── 3.1 FTS5 setup
├── 3.2 quilt-search: indexing
├── 3.3 quilt-search: fuzzy search
└── 3.4 Search integration

FASE 4: MCP Layer (Semanas 21-26)
├── 4.1 quilt-mcp: server scaffold
├── 4.2 Tools (10+)
├── 4.3 Resources (4 types)
├── 4.4 Notifications
└── 4.5 MCP conformance tests

FASE 5: Sync Engine (Semanas 27-32)
├── 5.1 quilt-sync: Loro integration
├── 5.2 Conflict resolution
├── 5.3 Offline queue
└── 5.4 Sync state machine

FASE 6: Platform (Semanas 33-38)
├── 6.1 quilt-platform: Tauri setup
├── 6.2 CLI commands
├── 6.3 File watcher
└── 6.4 Deep links

FASE 7: Polish (Semanas 39-44)
├── 7.1 Performance optimization
├── 7.2 Error handling hardening
├── 7.3 Full test coverage
└── 7.4 Documentation
```

---

*Arquitectura DDD + SOLID - Versión 2.0*
*Fecha: 2026-05-02*
