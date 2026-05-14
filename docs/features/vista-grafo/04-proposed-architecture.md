# Arquitectura Propuesta para Vista Grafo

## Overview

La Vista Grafo se integra en Quilt siguiendo la arquitectura existente:

```
┌─────────────────────────────────────────────────────────────────┐
│                         quilt-ui (WASM)                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                  pages/graph.rs (NUEVO)                   │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │   │
│  │  │  GraphView  │  │ GraphNode   │  │  GraphControls   │  │   │
│  │  │  Component  │  │  Component  │  │    Component    │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    bridge.rs                             │   │
│  │  get_graph_data() ──▶ window.__TAURI__.invoke()         │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                    Tauri IPC (invoke)
                              │
┌─────────────────────────────────────────────────────────────────┐
│                    quilt-mcp (Backend)                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                  server.rs                                │   │
│  │  resource_graph() ──▶ GraphDto {nodes, edges}           │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │            quilt-domain + quilt-infrastructure           │   │
│  │  PageRepository::get_all()                               │   │
│  │  BlockRepository::get_by_page()                         │   │
│  │  LightweightGraph::from_blocks()                         │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Componentes a Crear/Modificar

### 1. Extender resource_graph MCP

**Archivo**: `crates/quilt-mcp/src/server.rs`

```rust
// NUEVO: GraphDto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeDto {
    pub id: String,
    pub name: String,
    pub node_type: String,  // "page" | "journal"
    pub journal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeDto {
    pub source: String,  // Uuid como string
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDataDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
    pub last_updated: String,
}

// Modificar resource_graph()
async fn resource_graph(&self) -> Result<String, String> {
    let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
    
    // Recolectar todos los blocks con sus refs
    let mut all_blocks = Vec::new();
    for page in &pages {
        let blocks = self.block_repo.get_by_page(page.id)
            .await
            .map_err(|e| e.to_string())?;
        all_blocks.extend(blocks);
    }
    
    // Construir nodos
    let nodes: Vec<GraphNodeDto> = pages.iter().map(|p| GraphNodeDto {
        id: p.id.to_string(),
        name: p.name.clone(),
        node_type: if p.journal { "journal" } else { "page" }.to_string(),
        journal: p.journal,
    }).collect();
    
    // Construir edges desde Block.refs
    let mut edges = Vec::new();
    for block in &all_blocks {
        for &ref_id in &block.refs {
            edges.push(GraphEdgeDto {
                source: block.page_id.to_string(),
                target: ref_id.to_string(),
            });
        }
    }
    
    let graph_data = GraphDataDto {
        nodes,
        edges,
        last_updated: chrono::Utc::now().to_rfc3339(),
    };
    
    Ok(serde_json::to_string_pretty(&graph_data).unwrap())
}
```

### 2. Agregar Route en app.rs

**Archivo**: `crates/quilt-ui/src/app.rs`

```rust
// Agregar imports
use crate::pages::graph::GraphView;

// En App() -> Routes
<Route path=path!("/graph") view=GraphView />
```

### 3. Crear pages/graph.rs

**Archivo**: `crates/quilt-ui/src/pages/graph.rs`

```rust
//! Vista Grafo - Visualización del conocimiento como red

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::bridge::{self, BridgeError};

/// Nodo del grafo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeDto {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub journal: bool,
}

/// Arista del grafo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeDto {
    pub source: String,
    pub target: String,
}

/// Datos completos del grafo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDataDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
    pub last_updated: String,
}

/// Componente principal de la vista grafo
#[component]
pub fn GraphView() -> impl IntoView {
    // Async action para fetchear datos del grafo
    let fetch_graph = Action::new_local(move |_: &()| {
        async move {
            match bridge::get_graph_data().await {
                Ok(data) => Ok(data),
                Err(e) => Err(e),
            }
        }
    });

    // Iniciar fetch
    fetch_graph.dispatch(());

    // Estados reactivos
    let pending = fetch_graph.pending();
    let graph_data = fetch_graph.value();

    view! {
        <div class="graph-view">
            <div class="graph-header">
                <h2>"Vista Grafo"</h2>
                <p class="subtitle">"Navega tu conocimiento como una red"</p>
            </div>

            <Show
                when={move || !pending.get()}
                fallback={move || view! {
                    <div class="graph-loading">"Cargando grafo..."</div>
                }}
            >
                <Show
                    when={move || graph_data.get().is_some()}
                    fallback={move || view! {
                        <div class="graph-error">"Error al cargar el grafo"</div>
                    }}
                >
                    <div class="graph-container">
                        <svg class="graph-svg" viewBox="0 0 800 600">
                            // Nodos y aristas se renderizan aquí
                        </svg>
                        <GraphControls />
                    </div>
                </Show>
            </Show>
        </div>
    }
}

/// Controles de zoom y filtros
#[component]
fn GraphControls() -> impl IntoView {
    view! {
        <div class="graph-controls">
            <button>"Zoom +"</button>
            <button>"Zoom -"</button>
            <button>"Reset"</button>
            <select>
                <option value="all">"Todas"</option>
                <option value="pages">"Solo páginas"</option>
                <option value="journals">"Solo journals"</option>
            </select>
        </div>
    }
}
```

### 4. Agregar bridge function

**Archivo**: `crates/quilt-ui/src/bridge.rs`

```rust
/// Obtener datos del grafo - wired a `resource_graph` MCP
pub async fn get_graph_data() -> Result<serde_json::Value, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({});
        match invoke::<serde_json::Value>("resource_graph", &args).await {
            Ok(data) => Ok(data),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(serde_json::json!({
            "nodes": [],
            "edges": [],
            "last_updated": ""
        }))
    }
}
```

### 5. Crear módulo pages/graph.rs

**Archivo**: `crates/quilt-ui/src/pages/mod.rs`

```rust
pub mod graph;  // AGREGAR
```

## Flujo de Datos

```
1. Usuario visita /graph
   │
   ▼
2. GraphView se monta, dispatch fetch_graph
   │
   ▼
3. bridge::get_graph_data() llama a Tauri invoke
   │
   ▼
4. Tauri command invoca MCP resource_graph
   │
   ▼
5. MCP consulta PageRepository + BlockRepository
   │
   ▼
6. LightweightGraph.from_blocks() computa estructura
   │
   ▼
7. GraphDataDto se serializa y devuelve
   │
   ▼
8. Leptos renderiza SVG con D3.js layout
```

## Interacciones

| Interacción | Comportamiento |
|-------------|----------------|
| Zoom (scroll) | Escala el SVG viewBox |
| Pan (drag fondo) | Mueve el viewport |
| Click en nodo | Resalta conexiones |
| Doble click | Navega a la página |
| Hover | Muestra tooltip con nombre |
| Filtro | Muestra solo nodos del tipo |

## Estados de Error

```rust
#[component]
fn GraphErrorState(message: String) -> impl IntoView {
    view! {
        <div class="graph-error-state">
            <h3>"Error"</h3>
            <p>{message}</p>
            <button on:click={move |_| location().reload()}>
                "Reintentar"
            </button>
        </div>
    }
}
```

## Estructura de Archivos Final

```
crates/quilt-ui/src/
├── lib.rs
├── app.rs                    # Modificado: agregar route /graph
├── bridge.rs                 # Modificado: agregar get_graph_data()
└── pages/
    ├── mod.rs                # Modificado: pub mod graph
    ├── cognitive/
    │   └── argument_map.rs   # Referencia: patrón de vista
    └── graph.rs              # NUEVO: componente principal
```

## Testing

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_graph_data_serialization() {
        let dto = GraphDataDto {
            nodes: vec![GraphNodeDto {
                id: "test".into(),
                name: "Test Page".into(),
                node_type: "page".into(),
                journal: false,
            }],
            edges: vec![],
            last_updated: "2024-01-01T00:00:00Z".into(),
        };
        
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("Test Page"));
    }
}
```

## Integración con Sidebar

Agregar link en el sidebar para acceder a la vista:

```rust
// En crate/quilt-ui/src/components/sidebar.rs
<nav>
    <a href="/graph">"Vista Grafo"</a>
    // ... existing links
</nav>
```

## Consideraciones de Performance

1. **Lazy Loading**: No cargar el grafo completo al inicio
2. **Pagination**: Si hay >500 nodos, permitir filtrar
3. **Caching**: Cachear resultado por session
4. **Debounce**: No re-fetch durante zoom/pan frecuente

## Siguiente Paso

Esta arquitectura permite un MVP rápido. Para futuras mejoras:
- Implementar D3.js force layout en WASM
- Agregar clustering para grafos grandes
- Implementar búsqueda de nodos
- Añadir animaciones de transición
