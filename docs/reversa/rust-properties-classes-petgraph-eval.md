# Quilt — Sistema de Propiedades Tipadas + Clases en Rust

> Análisis técnico para la reimplementación Rust de Logseq DB graph
> Fecha: 2026-05-02 | Nivel: detalhado

---

## 1. Propiedades Tipadas — Implementación Rust

### 1.1 El modelo de Logseq DB

En Logseq DB, las propiedades NO son texto libre en frontmatter. Son **atributos DataScript indexados** con tipo validado. Esto permite:

```clj
;; En Logseq DB:
;; Una propiedad NO es solo un string en :block/properties
;; Es un datom (atributo) con tipo, cardinalidad, y validación

:logseq.property/due {:type :date, :value "2026-05-02"}    ;; Tipo date
:logseq.property/priority {:type :default, :closed-values ["high" "medium" "low"]}
:logseq.property/count {:type :number, :value 42}           ;; Tipo number, no string
```

### 1.2 Implementación Rust — Property System

```rust
// src/properties/types.rs

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

/// Tipos de propiedades (equivalente a property/type.cljs:15-47)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyType {
    /// Texto libre (equivalente a :default)
    Text,
    /// Número (i64 o f64)
    Number,
    /// Fecha (YYYY-MM-DD)
    Date,
    /// Fecha y hora
    DateTime,
    /// URL
    Url,
    /// Booleano (checkbox)
    Checkbox,
    /// Referencia a otra entidad del grafo (page/block)
    Node,
}

impl PropertyType {
    pub fn validate(&self, value: &PropertyValue) -> Result<(), ValidationError> {
        match (self, value) {
            (PropertyType::Text, PropertyValue::Text(_)) => Ok(()),
            (PropertyType::Number, PropertyValue::Number(_)) => Ok(()),
            (PropertyType::Date, PropertyValue::Date(_)) => Ok(()),
            (PropertyType::DateTime, PropertyValue::DateTime(_)) => Ok(()),
            (PropertyType::Url, PropertyValue::Text(url)) => {
                if url.starts_with("http://") || url.starts_with("https://") {
                    Ok(())
                } else {
                    Err(ValidationError::InvalidUrl(url.clone()))
                }
            }
            (PropertyType::Checkbox, PropertyValue::Checkbox(_)) => Ok(()),
            (PropertyType::Node, PropertyValue::NodeRef(_)) => Ok(()),
            (expected, got) => Err(ValidationError::TypeMismatch {
                expected: format!("{:?}", expected),
                got: format!("{:?}", got),
            }),
        }
    }
}

/// Valores de propiedad tipados (equivalente a los closed values + valores abiertos)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    Text(String),
    Number(f64),
    Date(NaiveDate),
    DateTime(NaiveDateTime),
    Checkbox(bool),
    NodeRef(Uuid),  // Referencia a otra entidad
}

/// Definición completa de una propiedad en el schema
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyDefinition {
    pub id: Uuid,
    pub db_ident: String,                // :logseq.property/due
    pub title: String,                    // "Due Date"
    pub property_type: PropertyType,
    pub cardinality: Cardinality,
    
    /// Valores cerrados (solo ciertos valores permitidos)
    pub closed_values: Vec<ClosedValue>,
    
    /// Contexto de visualización
    pub view_context: ViewContext,
    
    /// ¿Los usuarios pueden usar esta propiedad?
    pub public: bool,
    
    /// ¿Es consultable?
    pub queryable: bool,
    
    /// ¿Se oculta en la UI cuando está seteada?
    pub hidden: bool,
    
    /// Atributo externo (si se guarda fuera de :block/properties)
    pub attribute: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Cardinality {
    One,
    Many,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ViewContext {
    Page,
    Block,
    Never,
}

/// Closed value (valor predefinido para dropdown/opciones)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedValue {
    pub id: Uuid,
    pub db_ident: String,  // :logseq.property.status/done
    pub value: String,     // "Done"
    pub icon: Option<String>,
    pub order: f64,        // Orden en el dropdown
}
```

### 1.3 Schema de propiedades built-in

```rust
// src/properties/builtin.rs

use once_cell::sync::Lazy;

/// Propiedades built-in (equivalente a property.cljs:20-120+)
pub static BUILTIN_PROPERTIES: Lazy<HashMap<String, PropertyDefinition>> = Lazy::new(|| {
    let mut props = HashMap::new();

    // Propiedades de task
    props.insert("logseq.property/status".into(), PropertyDefinition {
        id: Uuid::nil(), // será reemplazado en runtime
        db_ident: "logseq.property/status".into(),
        title: "Status".into(),
        property_type: PropertyType::Text,
        cardinality: Cardinality::One,
        closed_values: vec![
            ClosedValue {
                id: Uuid::nil(),
                db_ident: "logseq.property.status/todo".into(),
                value: "TODO".into(),
                icon: Some("check".into()),
                order: 0.0,
            },
            ClosedValue {
                id: Uuid::nil(),
                db_ident: "logseq.property.status/doing".into(),
                value: "DOING".into(),
                icon: Some("play".into()),
                order: 1.0,
            },
            ClosedValue {
                id: Uuid::nil(),
                db_ident: "logseq.property.status/done".into(),
                value: "DONE".into(),
                icon: Some("check-circle".into()),
                order: 2.0,
            },
        ],
        view_context: ViewContext::Block,
        public: true,
        queryable: true,
        hidden: false,
        attribute: None,
    });

    // Prioridad con closed values numéricos
    props.insert("logseq.property/priority".into(), PropertyDefinition {
        id: Uuid::nil(),
        db_ident: "logseq.property/priority".into(),
        title: "Priority".into(),
        property_type: PropertyType::Text,
        cardinality: Cardinality::One,
        closed_values: vec![
            ClosedValue {
                id: Uuid::nil(),
                db_ident: "logseq.property.priority/high".into(),
                value: "A".into(),
                icon: Some("arrow-up".into()),
                order: 0.0,
            },
            ClosedValue {
                id: Uuid::nil(),
                db_ident: "logseq.property.priority/medium".into(),
                value: "B".into(),
                icon: Some("minus".into()),
                order: 1.0,
            },
            ClosedValue {
                id: Uuid::nil(),
                db_ident: "logseq.property.priority/low".into(),
                value: "C".into(),
                icon: Some("arrow-down".into()),
                order: 2.0,
            },
        ],
        view_context: ViewContext::Block,
        public: true,
        queryable: true,
        hidden: false,
        attribute: None,
    });

    // Fecha (tipo date real, no string)
    props.insert("logseq.property/deadline".into(), PropertyDefinition {
        id: Uuid::nil(),
        db_ident: "logseq.property/deadline".into(),
        title: "Deadline".into(),
        property_type: PropertyType::Date,
        cardinality: Cardinality::One,
        closed_values: vec![],
        view_context: ViewContext::Block,
        public: true,
        queryable: true,
        hidden: false,
        attribute: None,
    });

    // Propiedad de fecha programada
    props.insert("logseq.property/scheduled".into(), PropertyDefinition {
        id: Uuid::nil(),
        db_ident: "logseq.property/scheduled".into(),
        title: "Scheduled".into(),
        property_type: PropertyType::Date,
        cardinality: Cardinality::One,
        closed_values: vec![],
        view_context: ViewContext::Block,
        public: true,
        queryable: true,
        hidden: false,
        attribute: None,
    });

    // URL
    props.insert("logseq.property/url".into(), PropertyDefinition {
        id: Uuid::nil(),
        db_ident: "logseq.property/url".into(),
        title: "URL".into(),
        property_type: PropertyType::Url,
        cardinality: Cardinality::One,
        closed_values: vec![],
        view_context: ViewContext::Block,
        public: true,
        queryable: true,
        hidden: false,
        attribute: None,
    });

    props
});
```

### 1.4 Persistencia de propiedades en SQLite

```rust
// src/db/property_store.rs

use sqlx::SqlitePool;

pub struct PropertyStore {
    pool: SqlitePool,
}

impl PropertyStore {
    /// Guardar una propiedad como atributo directo del bloque
    pub async fn set_property(
        &self,
        block_id: Uuid,
        property: &PropertyDefinition,
        value: &PropertyValue,
    ) -> Result<(), DbError> {
        // En DB mode, las propiedades se guardan DOS veces:
        // 1. Como columna directa en block_properties (indexada, consultable)
        // 2. En el JSON :block/properties (para compatibilidad)

        let value_json = serde_json::to_value(value)?;

        sqlx::query(
            "INSERT INTO block_properties (block_id, property_id, value_type, value_json)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(block_id, property_id) DO UPDATE SET
               value_json = excluded.value_json,
               updated_at = strftime('%s','now')"
        )
        .bind(block_id.as_bytes())
        .bind(property.id.as_bytes())
        .bind(format!("{:?}", property.property_type))
        .bind(value_json.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Obtener todas las propiedades de un bloque (indexadas)
    pub async fn get_properties(
        &self,
        block_id: Uuid,
    ) -> Result<HashMap<String, PropertyValue>, DbError> {
        let rows = sqlx::query(
            "SELECT p.db_ident, bp.value_type, bp.value_json
             FROM block_properties bp
             JOIN property_definitions p ON bp.property_id = p.id
             WHERE bp.block_id = ?"
        )
        .bind(block_id.as_bytes())
        .fetch_all(&self.pool)
        .await?;

        let mut props = HashMap::new();
        for row in rows {
            // Deserializar según el tipo
            let value: PropertyValue = serde_json::from_str(&row.value_json)?;
            props.insert(row.db_ident, value);
        }

        Ok(props)
    }

    /// Query por propiedad (equivalente a property(query))
    pub async fn query_by_property(
        &self,
        property_db_ident: &str,
        value: &PropertyValue,
        limit: u32,
    ) -> Result<Vec<Uuid>, DbError> {
        let value_json = serde_json::to_string(value)?;

        let rows = sqlx::query(
            "SELECT bp.block_id
             FROM block_properties bp
             JOIN property_definitions p ON bp.property_id = p.id
             WHERE p.db_ident = ? AND bp.value_json = ?
             LIMIT ?"
        )
        .bind(property_db_ident)
        .bind(value_json)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| Uuid::from_slice(&r.block_id).unwrap()).collect())
    }
}
```

### 1.5 Tabla de migración para propiedades

```sql
-- Las propiedades son entidades de primera clase

CREATE TABLE property_definitions (
    id BLOB PRIMARY KEY NOT NULL,
    db_ident TEXT NOT NULL UNIQUE,        -- :logseq.property/due
    title TEXT NOT NULL,
    property_type TEXT NOT NULL,           -- Text, Number, Date, DateTime, Url, Checkbox, Node
    cardinality TEXT NOT NULL DEFAULT 'one', -- one, many
    view_context TEXT NOT NULL DEFAULT 'block', -- page, block, never
    public INTEGER NOT NULL DEFAULT 1,
    queryable INTEGER NOT NULL DEFAULT 1,
    hidden INTEGER NOT NULL DEFAULT 0,
    attribute TEXT,                        -- Atributo externo opcional
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_property_defs_ident ON property_definitions(db_ident);

-- Valores cerrados (opciones predefinidas)
CREATE TABLE closed_values (
    id BLOB PRIMARY KEY NOT NULL,
    property_id BLOB NOT NULL,
    db_ident TEXT NOT NULL,               -- :logseq.property.status/done
    value TEXT NOT NULL,
    icon TEXT,
    "order" REAL NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
);

CREATE INDEX idx_closed_values_property ON closed_values(property_id);

-- Asignación de propiedad a bloque (indexada)
CREATE TABLE block_properties (
    block_id BLOB NOT NULL,
    property_id BLOB NOT NULL,
    value_type TEXT NOT NULL,
    value_json TEXT NOT NULL,            -- JSON serializado según el tipo
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (block_id, property_id),
    FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE,
    FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
);

CREATE INDEX idx_block_props_property_value 
    ON block_properties(property_id, value_json);
```

---

## 2. Sistema de Clases — Implementación Rust

### 2.1 Modelo de clases

```rust
// src/classes/mod.rs

use std::collections::HashMap;
use uuid::Uuid;

/// Una clase es un tag tipado (equivalente a class.cljs:17-82)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Class {
    pub id: Uuid,
    pub db_ident: String,                   // :logseq.class/Task
    pub title: String,                      // "Task"
    
    /// Clases de las que hereda (extends)
    pub extends: Vec<Uuid>,
    
    /// Propiedades requeridas por esta clase
    pub required_properties: Vec<Uuid>,
    
    /// Propiedades por defecto (con valores)
    pub default_properties: HashMap<Uuid, PropertyValue>,
    
    /// Icono
    pub icon: Option<String>,
    
    /// ¿Es una clase built-in?
    pub builtin: bool,
    
    /// ¿Es una clase de usuario?
    pub user_defined: bool,
}

/// Built-in classes (class.cljs:17-82)
pub struct BuiltinClasses;

impl BuiltinClasses {
    pub fn root() -> Class {
        Class {
            id: Uuid::nil(),
            db_ident: "logseq.class/Root".into(),
            title: "Root Tag".into(),
            extends: vec![],
            required_properties: vec![],
            default_properties: HashMap::new(),
            icon: None,
            builtin: true,
            user_defined: false,
        }
    }

    pub fn tag() -> Class {
        Class {
            id: Uuid::nil(),
            db_ident: "logseq.class/Tag".into(),
            title: "Tag".into(),
            extends: vec![],  // extends Root implícitamente
            required_properties: vec![],
            default_properties: HashMap::new(),
            icon: None,
            builtin: true,
            user_defined: false,
        }
    }

    pub fn page() -> Class {
        Class {
            id: Uuid::nil(),
            db_ident: "logseq.class/Page".into(),
            title: "Page".into(),
            extends: vec![],
            required_properties: vec![],
            default_properties: HashMap::new(),
            icon: None,
            builtin: true,
            user_defined: false,
        }
    }

    pub fn journal() -> Class {
        let mut defaults = HashMap::new();
        defaults.insert(
            /* logseq.property.journal/title-format */ Uuid::nil(),
            PropertyValue::Text("MMM do, yyyy".into()),
        );

        Class {
            id: Uuid::nil(),
            db_ident: "logseq.class/Journal".into(),
            title: "Journal".into(),
            extends: vec![/* logseq.class/Page */],
            required_properties: vec![],
            default_properties: defaults,
            icon: Some("calendar".into()),
            builtin: true,
            user_defined: false,
        }
    }

    pub fn task() -> Class {
        Class {
            id: Uuid::nil(),
            db_ident: "logseq.class/Task".into(),
            title: "Task".into(),
            extends: vec![],  // extends Page
            required_properties: vec![
                /* logseq.property/status */ Uuid::nil(),
                /* logseq.property/priority */ Uuid::nil(),
            ],
            default_properties: HashMap::new(),
            icon: Some("checkbox".into()),
            builtin: true,
            user_defined: false,
        }
    }

    pub fn query() -> Class {
        Class {
            id: Uuid::nil(),
            db_ident: "logseq.class/Query".into(),
            title: "Query".into(),
            extends: vec![],
            required_properties: vec![],
            default_properties: HashMap::new(),
            icon: Some("search".into()),
            builtin: true,
            user_defined: false,
        }
    }

    pub fn property_class() -> Class {
        Class {
            id: Uuid::nil(),
            db_ident: "logseq.class/Property".into(),
            title: "Property".into(),
            extends: vec![],
            required_properties: vec![],
            default_properties: HashMap::new(),
            icon: None,
            builtin: true,
            user_defined: false,
        }
    }
}
```

### 2.2 Validación de clases en transacciones

```rust
// src/classes/validation.rs

pub struct ClassValidator {
    class_store: Arc<ClassStore>,
    property_store: Arc<PropertyStore>,
}

impl ClassValidator {
    /// Validar que un bloque cumpla con las propiedades requeridas de sus clases
    pub async fn validate_block(
        &self,
        block: &Block,
        tags: &[Uuid],  // tags (clases) asignadas al bloque
    ) -> Result<(), ValidationError> {
        // 1. Para cada tag, resolver su clase
        for tag_id in tags {
            let class = self.class_store.get_by_id(*tag_id).await?;
            
            if let Some(class) = class {
                // 2. Verificar propiedades requeridas (incluyendo heredadas)
                let all_required = self.get_all_required_properties(&class).await?;
                
                for prop_id in all_required {
                    let prop = self.property_store.get_by_id(prop_id).await?;
                    if let Some(prop) = prop {
                        let has_prop = self.property_store
                            .has_property(block.id, prop.id)
                            .await?;
                        
                        if !has_prop {
                            return Err(ValidationError::MissingRequiredProperty {
                                class: class.title.clone(),
                                property: prop.title.clone(),
                                block_id: block.id,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Resolver herencia de propiedades requeridas (recursivo)
    async fn get_all_required_properties(
        &self,
        class: &Class,
    ) -> Result<Vec<Uuid>, DbError> {
        let mut required = class.required_properties.clone();
        
        // Propiedades heredadas
        for parent_id in &class.extends {
            if let Some(parent) = self.class_store.get_by_id(*parent_id).await? {
                let parent_props = self.get_all_required_properties(&parent).await?;
                required.extend(parent_props);
            }
        }
        
        Ok(required)
    }
}
```

### 2.3 Tablas SQL para clases

```sql
CREATE TABLE class_definitions (
    id BLOB PRIMARY KEY NOT NULL,
    db_ident TEXT NOT NULL UNIQUE,     -- :logseq.class/Task
    title TEXT NOT NULL,
    icon TEXT,
    builtin INTEGER NOT NULL DEFAULT 0,
    user_defined INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_classes_ident ON class_definitions(db_ident);

-- Herencia entre clases
CREATE TABLE class_inheritance (
    class_id BLOB NOT NULL,
    parent_id BLOB NOT NULL,
    PRIMARY KEY (class_id, parent_id),
    FOREIGN KEY (class_id) REFERENCES class_definitions(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES class_definitions(id) ON DELETE CASCADE
);

-- Propiedades requeridas por clase
CREATE TABLE class_required_properties (
    class_id BLOB NOT NULL,
    property_id BLOB NOT NULL,
    PRIMARY KEY (class_id, property_id),
    FOREIGN KEY (class_id) REFERENCES class_definitions(id) ON DELETE CASCADE,
    FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
);

-- Propiedades por defecto de clase
CREATE TABLE class_default_properties (
    class_id BLOB NOT NULL,
    property_id BLOB NOT NULL,
    default_value_json TEXT NOT NULL,
    PRIMARY KEY (class_id, property_id),
    FOREIGN KEY (class_id) REFERENCES class_definitions(id) ON DELETE CASCADE,
    FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
);
```

---

## 3. Evaluación de petgraph — Pros y Contras

### 3.1 ¿Qué es petgraph?

Librería de grafos en Rust puro. Soporta grafos dirigidos y no dirigidos, algoritmos de grafos, y serialización. Es la librería estándar de facto para grafos en Rust ecosistema.

```
petgraph::Graph<N, E> donde:
  N = tipo de dato del nodo
  E = tipo de dato de la arista

Algoritmos incluidos:
- Dijkstra, Bellman-Ford (shortest path)
- Topological sort
- Strongly connected components (Tarjan)
- Minimum spanning tree
- Isomorphism checks
- Dot format export
```

### 3.2 Pros de añadir petgraph al stack

#### PRO-1: Queries de grafo reales (no simuladas)

```rust
use petgraph::graph::Graph;
use petgraph::algo::{dijkstra, has_path_connecting, connected_components};

// Sin petgraph:
let related = block.refs.iter()
    .map(|r| get_block(r))
    .collect::<Vec<_>>();  // Solo 1 nivel de profundidad

// Con petgraph:
let path = dijkstra(&graph, source_node, None, |e| *e.weight());
// "¿Cuál es el camino más corto entre dos páginas?"
// "¿Qué páginas son las más centrales en mi knowledge graph?"
// "¿Hay comunidades de conocimiento conectadas?"
```

#### PRO-2: Visualización de grafo más eficiente

```rust
use petgraph::dot::{Dot, Config};

// El graph view ya NO se recalcula desde queries relacionales
// El grafo ES la estructura de datos en memoria
let dot = Dot::with_config(&graph, &[Config::EdgeNoLabel]);
// Renderizar con D3, cytoscape, o cualquier visualizador de grafos
```

#### PRO-3: PageRank y Centralidad para AI agents

```rust
// Algoritmos de importancia de nodos (no disponibles en modelo relacional)
// ¿Qué páginas son más importantes?
// ¿Qué bloques son hubs de conocimiento?

fn compute_pagerank(graph: &Graph<BlockNode, RefEdge>) -> Vec<(Uuid, f64)> {
    // petgraph no tiene PageRank built-in, pero es trivial implementarlo
    // sobre su estructura de grafo
}

// Esto permite a los AI agents priorizar lecturas
```

#### PRO-4: Soporte para MCP Resources avanzados

```rust
// MCP resource: subgraph of a topic
#[mcp_resource(uri = "logseq://topics/{topic}/subgraph")]
async fn topic_subgraph(&self, topic: String) -> SubgraphDto {
    // Extraer subgrafo conectado alrededor de un tópico
    // Usando petgraph para encontrar el connected component
}

// Esto es valioso para AI agents que exploran conocimiento
```

#### PRO-5: Serialización eficiente para sync

```rust
// El grafo se serializa/deserializa para sync entre dispositivos
// Las operaciones de grafo (add_node, add_edge) son atómicas
// CRDT de grafos es más natural que CRDT de tablas
```

#### PRO-6: Detección de ciclos y consistencia

```rust
use petgraph::algo::is_cyclic_directed;

// Validar que no haya ciclos en la jerarquía de bloques
// (un bloque no puede ser hijo de sí mismo indirectamente)
if is_cyclic_directed(&hierarchy_graph) {
    return Err(ValidationError::CircularReference);
}

// Validar que no haya herencia circular en clases
if is_cyclic_directed(&class_inheritance_graph) {
    return Err(ValidationError::CircularInheritance);
}
```

### 3.3 Contras de añadir petgraph

#### CONTRA-1: Duplicación de estado

El mayor problema: **petgraph mantiene su propia estructura en memoria, independiente de SQLite.**

```
SQLite (persistencia)          petgraph::Graph (memoria)
     │                              │
     │  Cada cambio debe            │  Hay que mantenerlos
     │  sincronizarse en            │  sincronizados.
     │  ambas direcciones           │
     │                              │
```

Esto crea un problema de **doble fuente de verdad**.

#### CONTRA-2: Complejidad de sincronización

```rust
// Cada transacción de bloque requiere:
// 1. Escribir en SQLite (persistencia)
// 2. Escribir en FTS5 (búsqueda)
// 3. Escribir en petgraph (estructura de grafo)
// 4. Notificar MCP subscribers

// Si una de estas operaciones falla, hay inconsistencia
pub async fn create_block(&self, block: Block) -> Result<()> {
    let tx = self.db.begin().await?;
    
    // 1. SQLite
    sqlx::query("INSERT INTO blocks ...").execute(&mut tx).await?;
    
    // 2. FTS5 (automático via triggers)
    
    // 3. petgraph (en memoria)
    let node_idx = self.graph.add_node(BlockNode::from(&block));
    // ¿Qué pasa si esto falla? ¿Rollback de SQLite?
    
    // 4. Referencias como aristas
    for ref_id in &block.refs {
        let target = self.node_index(*ref_id)?;
        self.graph.add_edge(node_idx, target, RefEdge::default());
    }
    
    tx.commit().await?;
    Ok(())
}
```

#### CONTRA-3: Consumo de memoria

```
SQLite:  ~50MB para un grafo de 100k bloques  (con índices)
petgraph: ~80MB para el mismo grafo            (cada nodo y arista en memoria)
SQLite + petgraph: ~130MB                      (ambos simultáneos)
```

Para un grafo grande (1M+ bloques), petgraph se vuelve prohibitivo.

#### CONTRA-4: petgraph no es distribuible

```
El modelo CRDT (>Loro) no se integra nativamente con petgraph.
Habría que:
1. Mutar Loro CRDT
2. Mutar petgraph
3. Sincronizar ambos

Esto es frágil y propenso a bugs.
```

#### CONTRA-5: petgraph no persiste nativamente

```
No hay `petgraph.save_to_disk()` ni `petgraph.load_from_sqlite()`.
Hay que serializar/deserializar manualmente.
```

### 3.4 Matriz de decisión

| Criterio | Solo SQLite | SQLite + petgraph | Peso |
|----------|-------------|-------------------|------|
| **Persistencia** | Nativa | Duplicación manual | Alto |
| **Queries de grafo** | No (simuladas) | Sí (nativas) | Medio |
| **Memoria** | Bajo | Alto (2x) | Medio |
| **Sync (CRDT)** | Simple | Complejo | Alto |
| **Visualización** | Recalculada | Directa | Bajo |
| **AI agent value** | Medio | Alto | Alto |
| **Mantenibilidad** | Alta | Media | Alto |
| **Rendimiento queries** | SQL | O(1) aristas | Bajo |

### 3.5 Recomendación

```
NO añadir petgraph al stack inicial.
```

**Estrategia recomendada:**

#### Fase 1: SQLite puro (MVP)
- Modelo relacional con tablas `blocks` + `refs`
- El grafo se materializa bajo demanda (como Logseq actual)
- Queries via SQL + FTS5
- CRDT via Loro (sobre el modelo relacional)

#### Fase 2: Graph index opcional (si necesidad)
- **No petgraph.** Usar un **índice de grafo ligero**:
```rust
// Cache de adyacencia para queries de grafo
struct GraphIndex {
    // Solo guarda IDs, no datos completos
    outgoing: HashMap<Uuid, Vec<Uuid>>,  // source → targets
    incoming: HashMap<Uuid, Vec<Uuid>>,  // target → sources
}

impl GraphIndex {
    fn rebuild_from_db(&mut self, pool: &SqlitePool) { ... }
    fn add_edge(&mut self, from: Uuid, to: Uuid) { ... }
    fn remove_edge(&mut self, from: Uuid, to: Uuid) { ... }
    
    // Queries de grafo usando este índice
    fn shortest_path(&self, from: Uuid, to: Uuid) -> Option<Vec<Uuid>> {
        // BFS sobre el HashMap (sin petgraph)
    }
    
    fn neighbors(&self, node: Uuid) -> &[Uuid] {
        self.outgoing.get(&node).unwrap_or(&[])
    }
    
    fn centrality(&self) -> HashMap<Uuid, f64> {
        // Betweenness centrality sobre el índice
    }
}
```

#### Fase 3: petgraph solo si:
1. Los queries de grafo son >30% de las operaciones
2. La visualización de grafo necesita performance de grafo real
3. Hay budget de memoria suficiente
4. Se encuentra una solución de sync petgraph↔CRDT estable

---

## 4. Stack Final Recomendado

```toml
# Cargo.toml

[dependencies]
# Core
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = "0.4"

# Graph (Fase 2)
# petgraph = "0.6"   ← NO en Fase 1
# GraphIndex manual en Fase 2

# Sync
loro = "0.2"  # CRDT

# Search
# sonic = "1.3"  # o Tantivy para full-text

# MCP
mcp-sdk = "0.1"

# Async
tokio = { version = "1", features = ["full"] }

# Desktop
tauri = "2"

# Error
thiserror = "1"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Metrics
metrics = "0.23"

# LRU
lru = "0.13"
```

---

## 5. Conclusión

| Pregunta | Respuesta |
|----------|-----------|
| ¿Propiedades tipadas en Rust? | Sí, con `enum PropertyType` y validación |
| ¿Sistema de clases con herencia? | Sí, con SQL tables + validación en transacciones |
| ¿petgraph? | **No en Fase 1.** GraphIndex manual en Fase 2. petgraph solo si necesario en Fase 3 |
| ¿CRDT sync? | Con Loro, sobre el modelo relacional |
| ¿AI agent value? | Alto: propiedades tipadas permiten queries semánticas |

**Principio:** El grafo es una **proyección** de los datos, no la fuente de verdad. La fuente de verdad es SQLite. El grafo se materializa bajo demanda, con un índice de adyacencia ligero como cache.
