# Análisis del Stack Actual de Quilt

## Stack Tecnológico

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend (WASM)                       │
│  Leptos 0.7 + WASM + Tauri IPC                        │
│  crate: quilt-ui                                       │
└─────────────────────────────────────────────────────────┘
                           │
                    window.__TAURI__.invoke
                           │
┌─────────────────────────────────────────────────────────┐
│                    Backend (Rust)                        │
│  Tauri Commands + Domain + Infrastructure               │
│  crate: quilt-mcp, quilt-domain, quilt-infrastructure   │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│                    Database                              │
│  SQLite con sqlx async                                  │
└─────────────────────────────────────────────────────────┘
```

## Lo que Ya Tenemos

### 1. Estructura de Datos (Block.refs)

```rust
// crates/quilt-domain/src/entities/block.rs
pub struct Block {
    pub id: Uuid,
    pub page_id: Uuid,
    pub refs: Vec<Uuid>,        // ← Referencias a otros blocks/pages
    pub content: String,
    // ...
}
```

**Importante**: `Block.refs` contiene los IDs de bloques/páginas referenciados. Esto es todo lo que necesitamos para construir las aristas del grafo.

### 2. LightweightGraph Existente

```rust
// crates/quilt-cognitive/src/cognitive_mirror/graph.rs
#[derive(Debug, Clone)]
pub struct LightweightGraph {
    adj: HashMap<Uuid, Vec<Uuid>>,      // outgoing edges
    incoming: HashMap<Uuid, Vec<Uuid>>, // incoming edges (backlinks)
    nodes: HashSet<Uuid>,
}

impl LightweightGraph {
    pub fn from_blocks(blocks: &[Block]) -> Self {
        // Convierte blocks con refs en un grafo
    }
    
    pub fn edges(&self) -> Vec<(Uuid, Uuid)> {
        // Devuelve todas las aristas (from, to)
    }
}
```

### 3. Patrón de Bridge Tauri

```rust
// crates/quilt-ui/src/bridge.rs
pub async fn invoke<T: for<'de> Deserialize<'de>>(
    cmd: &str,
    args: &serde_json::Value,
) -> Result<T, BridgeError> {
    // window.__TAURI__.invoke(cmd, args)
}
```

### 4. Vista ArgumentMapView (Referencia)

```rust
// crates/quilt-ui/src/pages/cognitive/argument_map.rs
// Patrón para crear una vista que:
1. Usa Action para fetch async
2. Muestra estados de loading/error
3. Renderiza componentes Leptos
```

### 5. resource_graph MCP (Incompleto)

```rust
// crates/quilt-mcp/src/server.rs:2034
async fn resource_graph(&self) -> Result<String, String> {
    // Solo devuelve counts, NO la estructura real del grafo:
    // {"pages": 10, "journals": 5, "blocks": 100}
}
```

## Lo que Necesitamos Añadir

### 1. Extender resource_graph

**Actualmente**:
```rust
// Solo devuelve counts
Ok(serde_json::json!({
    "pages": page_count,
    "journals": journal_count,
    "blocks": all_blocks.len(),
}))
```

**Necesitamos**:
```rust
// Devolver estructura completa del grafo
Ok(serde_json::json!({
    "nodes": [
        {"id": "uuid", "name": "page name", "type": "page|journal"}
    ],
    "edges": [
        {"source": "uuid", "target": "uuid"}
    ]
}))
```

### 2. Nuevo Comando Tauri (Opcional)

Si queremos granularidad por página o filtrado:

```rust
// En quilt-platform/src-tauri/src/commands/
#[tauri::command]
async fn get_graph_data() -> Result<GraphData, String> {
    // Extraer lógica del MCP resource_graph
}
```

### 3. Componente GraphView en Leptos

```rust
// crates/quilt-ui/src/pages/graph.rs
#[component]
pub fn GraphView() -> impl IntoView {
    let fetch_graph = Action::new_local(...);
    
    view! {
        <div class="graph-view">
            <svg class="graph-svg">
                // Nodos y aristas
            </svg>
            <div class="graph-controls">
                // Zoom, filter, search
            </div>
        </div>
    }
}
```

## Repositorios Existentes

### PageRepository
```rust
// crates/quilt-domain/src/repositories/page_repository.rs
pub trait PageRepository: Send + Sync {
    async fn get_all(&self) -> Result<Vec<Page>, DomainError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Page>, DomainError>;
    // ...
}
```

### BlockRepository  
```rust
// crates/quilt-domain/src/repositories/block_repository.rs
pub trait BlockRepository: Send + Sync {
    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError>;
    async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, DomainError>;
    // ...
}
```

### SqliteBlockRepository (Implementación)
```rust
// crates/quilt-infrastructure/src/database/sqlite/repositories.rs:700+
impl BlockRepository for SqliteBlockRepository {
    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
        // SELECT * FROM blocks WHERE page_id = ? AND deleted_at IS NULL
    }
}
```

## Datos Disponibles

| Datos | Disponible | Fuente |
|-------|------------|--------|
| Lista de páginas | ✅ | `PageRepository::get_all()` |
| Bloques por página | ✅ | `BlockRepository::get_by_page()` |
| Referencias (refs) | ✅ | `Block.refs` |
| Backlinks | ✅ | `BlockRepository::get_backlinks()` |
| Metadatos de página | ✅ | `Page` entity |
| Grafo completo | ⚠️ | Necesita implementar |

## Conexiones a la Base de Datos

```rust
// El schema de blocks incluye refs como JSON
// crates/quilt-integration-tests/tests/e2e_tests.rs
CREATE TABLE blocks (
    refs TEXT NOT NULL DEFAULT '[]',  -- Vec<Uuid> serializado
    // ...
);
```

## Resumen de Reutilización

```
┌─────────────────────┐    ┌──────────────────┐    ┌─────────────┐
│ Block.refs          │───▶│ LightweightGraph │───▶│ UI DTOs     │
│ (edges)             │    │ from_blocks()    │    │ nodes/edges │
└─────────────────────┘    └──────────────────┘    └─────────────┘
         │                         │                       │
         ▼                         ▼                       ▼
┌─────────────────────┐    ┌──────────────────┐    ┌─────────────┐
│ PageRepository      │    │ .edges()         │    │ GraphView   │
│ get_all()           │    │ .nodes()         │    │ component   │
└─────────────────────┘    └──────────────────┘    └─────────────┘
```

**Podemos reutilizar**:
- ✅ `Block.refs` para aristas
- ✅ `LightweightGraph` para estructura del grafo
- ✅ `PageRepository::get_all()` para nodos
- ✅ Bridge pattern para Tauri IPC
- ✅ ArgumentMapView como referencia de componente

**Necesitamos implementar**:
- ❌ Extension de `resource_graph` MCP
- ❌ DTO para transferring graph data
- ❌ Componente `GraphView`
- ❌ Layout algorithm (d3-force o similar)
