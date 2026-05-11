# Quilt — MCP Agent AI Capabilities: Lateral Thinking Proposal

> Fecha: 2026-05-02
> Principio: Quilt no es un repositorio de notas con AI añadida. Es un **sistema nervioso cognitivo** donde humanos y AI agents co-evolucionan conocimiento.

---

## 0. Filosofía de Diseño

```
Un PKM tradicional:         Quilt con MCP Agents:

Humano → escribe notas      Humano ↔ AI Agent (co-creación)
         │                            │
         └── archiva                 ├── El agente lee tu grafo
                                     ├── El agente pregunta, no solo responde
                                     ├── El agente detecta patrones que tú no ves
                                     ├── El agente confronta tus sesgos
                                     └── El agente evoluciona contigo
```

**No se trata de "AI que busca en mis notas". Se trata de AI que piensa contigo.**

---

## 1. Las 7 Capacidades Fundacionales

### 1.1 🧠 COGNITIVE MIRROR — El espejo de tu pensamiento

**Qué hace:** El agente analiza tu grafo de conocimiento y te muestra tu propio mapa cognitivo.

```
MCP Tool: quilt_cognitive_mirror
Input: topic (opcional), depth (1-5)
Output: CognitiveMap { clusters, density, frontiers, gaps, influences }
```

**Funcionamiento:**
- Analiza qué áreas de conocimiento tienes más densas (muchas notas interconectadas)
- Identifica **knowledge frontiers**: áreas que mencionas frecuentemente pero con poca profundidad (superficiales)
- **Influence map**: qué ideas influyen en qué otras (no solo links, sino patrones de pensamiento)
- **Cognitive gaps**: temas que rodeas pero nunca atacas directamente

**Valor:** El humano ve su mente desde fuera. "No sabía que el 40% de mis notas giran alrededor de 'productividad' pero nunca profundizo en 'procrastinación'."

### 1.2 🔗 SERENDIPITY ENGINE — Descubrimiento de conexiones no obvias

**Qué hace:** Encuentra conexiones sorprendentes entre ideas aparentemente no relacionadas.

```
MCP Tool: quilt_serendipity
Input: none (runs periodically) or block_id (specific)
Output: SerendipityConnection { idea_a, idea_b, bridge_concept, confidence, explanation }
```

**Algoritmo:**
- **Structural similarity:** Dos nodos que comparten estructura pero no contenido
- **Temporal proximity:** Ideas que desarrollaste en la misma época pero nunca vinculaste
- **Semantic bridge:** Concepto C que conecta A y B sin que lo hayas notado
- **Contradiction detection:** Crees X en una nota e Y en otra (no son compatibles)

**Ejemplo:**
> "Tus notas sobre 'estoicismo' y 'startup failure' comparten 8 patrones estructurales idénticos. ¿Son los principios estoicos una herramienta de resiliencia para founders?"

**Valor:** El agente no espera a que preguntes. **Te notifica** conexiones que nunca habrías buscado.

### 1.3 ⚔️ ARGUMENT CARTOGRAPHER — Mapa de debates y posiciones

**Qué hace:** Estructura debates, argumentos y contraargumentos como entidades de primera clase.

```
MCP Tool: quilt_argument_map
Input: topic
Output: ArgumentGraph { positions, evidence, rebuttals, consensus_zones, open_questions }
```

**Modelo de datos en Quilt:**
```
Position "Rust es mejor que Go para sistemas"
  ├── Argument "Zero-cost abstractions" (strength: 0.9)
  │   ├── Evidence → block "Benchmark Rust vs Go 2024"
  │   └── Rebuttal → "Pero compile times..."
  ├── Argument "Memory safety sin GC" (strength: 0.95)
  │   └── Evidence → block "CVEs by language 2023"
  └── Counter-position → "Go es mejor para equipos grandes"
      └── Argument "Simplicidad de aprendizaje"
```

**Valor:** Tus notas ya contienen argumentos — el agente los estructura. Puedes visualizar debates completos, ver qué posición tiene más evidencia, y detectar falacias.

### 1.4 🌱 MENTAL MODEL GARDENER — Cultiva y evoluciona modelos mentales

**Qué hace:** Extrae modelos mentales implícitos de tus notas y los formaliza, los confronta con nueva información, y te avisa cuando necesitan actualizarse.

```
MCP Tool: quilt_mental_model
Input: domain (opcional)
Output: MentalModel { assumptions, predictions, falsifiable_tests, confidence, last_tested, contradictions_found }
```

**Ciclo de vida de un modelo mental en Quilt:**
```
1. DETECT: El agente identifica un modelo implícito en tus notas
   "Detecté que operas bajo el modelo 'efecto compounding' en 23 notas"

2. FORMALIZE: El agente propone una formalización
   "Tu modelo parece ser: small_effort × time × consistency = large_result"

3. TEST: El agente busca evidencia a favor o en contra en tu propio grafo
   "Encontré 3 casos donde este modelo falló en tus notas. ¿Refinamos?"

4. EVOLVE: El modelo se actualiza con nueva información
   "Nueva nota contradice assumption #2. Sugiero revisar el modelo."

5. GARDEN: El agente monitorea la salud de tus modelos mentales
   "No has revisado tu modelo de 'product-market fit' en 14 meses. 
    ¿Sigue siendo válido con tus notas recientes?"
```

**Valor:** Tus modelos mentales se vuelven entidades vivas, no notas estáticas. El agente es un jardinero que poda, riega y cultiva tu pensamiento.

### 1.5 🔮 COUNTERFACTUAL EXPLORER — "¿Y si...?"

**Qué hace:** Genera escenarios contrafactuales basados en tu conocimiento para explorar alternativas.

```
MCP Tool: quilt_counterfactual
Input: scenario or decision_point
Output: CounterfactualTree { branches, consequences, assumptions_challenged, blindspots }
```

**Ejemplo:**
> Human: "¿Y si hubiéramos elegido PostgreSQL en vez de MongoDB en 2023?"
> 
> Agent: "Basado en tus notas de 2023:
> - Branch A (PostgreSQL): Tus notas de 'schema migrations' sugieren que habrías tenido 40% menos bugs de datos
> - Branch B (MongoDB real): Tus notas de 'flexibility' muestran que iteraste 3x más rápido en early stage
> - Assumption challenged: Tu nota 'MongoDB caused our scaling issues' — pero 4 notas posteriores sugieren que fue architectural, no DB
> - Blindspot: No consideraste 'hosted Postgres' como opción híbrida"

**Valor:** Aprendizaje de decisiones pasadas. No es arrepentimiento — es entrenamiento para mejores decisiones futuras.

### 1.6 🧬 KNOWLEDGE EVOLUTION TRACKER — Cómo cambia lo que sabes

**Qué hace:** Trackea la evolución de tu conocimiento a lo largo del tiempo.

```
MCP Tool: quilt_knowledge_evolution
Input: topic (opcional), timespan
Output: KnowledgeTimeline { 
    belief_changes, confidence_shifts, abandoned_ideas, 
    reinforced_ideas, external_influences 
}
```

**Visualización:**
```
Tema: "Remote Work"
2022: "Remote work is the future" (confidence: 0.9, 12 notas)
2023: "Remote work has collaboration costs" (confidence: 0.7, 8 notas)
2024: "Hybrid with intentional in-person" (confidence: 0.85, 15 notas)
      ↑ Influenciado por: nota "Buffer State of Remote 2024"

Abandoned ideas:
- "Fully async is always better" (abandoned 2023-Q2)
- "Daily standups are unnecessary" (reintroduced 2024-Q1)
```

**Valor:** No solo sabes lo que piensas hoy. Sabes **cómo llegaste a pensarlo** y qué te hizo cambiar de opinión. Meta-cognición cuantificada.

### 1.7 🎭 MULTI-AGENT ROUNDTABLE — Debate entre agentes usando tu conocimiento

**Qué hace:** Múltiples agentes AI con diferentes perspectivas debaten un tema usando tu grafo como evidencia.

```
MCP Tool: quilt_roundtable
Input: topic, perspectives (opcional)
Output: RoundtableResult { 
    positions, agreements, disagreements, 
    novel_synthesis, recommended_reading (de tu grafo)
}
```

**Perspectivas predefinidas:**
- 🏛️ **Skeptic** — Cuestiona todo, pide evidencia
- 🔬 **Scientist** — Busca datos, experimentos, falsabilidad
- 🎨 **Creative** — Propone analogías, conexiones laterales
- 💼 **Pragmatist** — "¿Y esto cómo se implementa?"
- 🌍 **Systems Thinker** — "¿Qué efectos de segundo orden genera esto?"
- 📜 **Historian** — "¿Qué precedentes hay en tus notas?"

**Ejemplo:**
```
Human: "¿Debería lanzar Quilt como open-source o SaaS primero?"

🎨 Creative: "Tus notas sobre 'network effects' sugieren que open-source 
             crearía ecosistema de plugins antes del lanzamiento pago"
             
💼 Pragmatist: "Tu nota 'Startup Runway Q2 2026' muestra 8 meses de runway. 
               SaaS-first genera revenue desde día 1. Open-source no."
               
🔬 Scientist: "3 de tus notas sobre 'OSS business models' muestran que 
              open-core es el modelo más exitoso. ¿Híbrido?"
              
📜 Historian: "En 2023 escribiste 'Obsidian ganó por plugins, no por features'.
             ¿Aplica esa lección aquí?"

🏛️ Skeptic: "Pero en 2024 escribiste 'el mercado OSS está saturado de herramientas PKM'.
           ¿Hay espacio para otra?"
           
🌍 Systems: "Si es SaaS-first, dependes de cloud. Tus notas sobre 'self-hosting'
            muestran que tu audiencia valora soberanía de datos. ¿Conflicto?"
```

**Valor:** El agente no te dice qué hacer. **Te muestra tus propias contradicciones, evidencia y patrones** desde múltiples ángulos para que decidas mejor.

---

## 2. Capacidades Pasivas (Background Agents)

Estos agentes corren en background, sin que el humano los invoque:

### 2.1 🔔 KNOWLEDGE DECAY MONITOR

```
Detecta notas que necesitan actualización:
- "Esta nota tiene 18 meses. 7 notas posteriores la contradicen parcialmente."
- "El paper que citas aquí fue refutado en 2025."
- "Esta definición ya no coincide con tu uso actual del término."
```

### 2.2 🔍 CURIOSITY INJECTOR

```
El agente detecta qué NO sabes y te lo señala:
- "Mencionas 'CRDT' en 12 notas pero nunca has escrito una nota explicando qué es."
- "Tu conocimiento de Rust cubre async pero nunca has explorado unsafe Rust."
- "Hay un gap entre lo que sabes de SQL y lo que necesitas para la feature que planeas."
```

### 2.3 ⚡ EMERGENCE DETECTOR

```
Detecta patrones emergentes ANTES de que sean obvios:
- "En las últimas 2 semanas, 8 notas tuyas mencionan 'agotamiento'. ¿Patrón?"
- "Tus notas sobre 'MCP' y 'WebAssembly' están convergiendo. ¿Nueva área de interés?"
- "Has escrito 15 notas sobre 'testing' este mes vs 2 el mes pasado. ¿Shift de foco?"
```

---

## 3. MCP Tools Catalog — Definición Técnica

### Tools activas (invocadas por el agente)

| Tool | Trigger | Input | Output |
|------|---------|-------|--------|
| `quilt_cognitive_mirror` | On-demand | topic, depth | CognitiveMap |
| `quilt_serendipity` | On-demand / Scheduled | block_id (opt) | Connection[] |
| `quilt_argument_map` | On-demand | topic | ArgumentGraph |
| `quilt_mental_model` | On-demand | domain (opt) | MentalModel[] |
| `quilt_counterfactual` | On-demand | scenario | CounterfactualTree |
| `quilt_knowledge_evolution` | On-demand | topic, timespan | KnowledgeTimeline |
| `quilt_roundtable` | On-demand | topic, perspectives[] | RoundtableResult |
| `quilt_query` | On-demand | dsl, limit | QueryResult |
| `quilt_search` | On-demand | query, limit | SearchResult[] |
| `quilt_create_block` | On-demand | page, content, parent | Block |
| `quilt_get_block_tree` | On-demand | block_id | BlockTree |
| `quilt_list_pages` | On-demand | — | Page[] |

### Notifications (push del agente al humano)

| Notification | Trigger | Payload |
|-------------|---------|---------|
| `quilt_serendipity_found` | Nueva conexión no obvia detectada | Connection |
| `quilt_model_contradicted` | Evidencia contradice modelo mental | ModelAlert |
| `quilt_knowledge_decay` | Nota necesita actualización | DecayAlert |
| `quilt_curiosity_gap` | Gap de conocimiento detectado | CuriosityAlert |
| `quilt_emergence_detected` | Patrón emergente detectado | EmergenceAlert |
| `quilt_block_changed` | Bloque modificado (por humano u otro agente) | BlockChangedEvent |

---

## 4. El Meta-Nivel: Quilt como Sistema Evolutivo

### 4.1 Agent Memory (lo que el agente aprende de ti)

```rust
struct AgentMemory {
    /// Patrones de pensamiento del humano
    thinking_patterns: Vec<ThinkingPattern>,
    
    /// Sesgos cognitivos detectados
    cognitive_biases: Vec<CognitiveBias>,
    
    /// Preferencias de interacción
    interaction_preferences: InteractionProfile,
    
    /// Nivel de conocimiento por dominio
    knowledge_levels: HashMap<Domain, KnowledgeLevel>,
    
    /// Preguntas que el humano ignora sistemáticamente
    avoided_topics: Vec<String>,
    
    /// Cuándo el humano es más receptivo a sugerencias
    receptivity_window: Option<TimeWindow>,
}
```

**El agente aprende de ti. No es estático. Evoluciona.**

### 4.2 Collective Intelligence (multi-humano)

```
Si tú y un colega usan Quilt:
- El agente detecta áreas de conocimiento complementarias
- "María sabe de databases, tú sabes de frontend. ¿Por qué no co-escriben 
   una nota sobre 'full-stack architecture'?"
- El agente facilita colaboración sin forzarla
```

---

## 5. MCP Resource Hierarchy

```
logseq://graph                        → Full graph
  ├── logseq://pages                  → All pages
  │   └── logseq://pages/{name}       → Specific page
  ├── logseq://journals               → Journal pages
  │   └── logseq://journal/{date}     → Specific date
  ├── logseq://cognitive/mirror       → Cognitive map (live)
  ├── logseq://cognitive/models       → Mental models (live)
  ├── logseq://cognitive/evolution    → Knowledge evolution (live)
  ├── logseq://arguments/{topic}      → Argument graph (live)
  ├── logseq://serendipity            → Serendipity connections (live)
  └── logseq://agent/memory           → Agent memory (lo que aprendió de ti)
```

---

## 6. Qué hace esto DIFERENTE de cualquier PKM actual

| Sistema | Modelo | Agente |
|---------|--------|--------|
| Notion AI | "Escribe esto por mí" | Secretario |
| Obsidian Copilot | "Búscame notas sobre X" | Bibliotecario |
| Mem.ai | "Recuérdame esto" | Asistente |
| **Quilt** | **"Piensa conmigo"** | **Compañero cognitivo** |

**Quilt no es un PKM con AI. Es un entorno cognitivo donde AI y humano co-evolucionan.**

---

## 7. Sistema de Plugins — Extensibilidad del MCP Server

### 7.1 Overview del Sistema de Plugins

El servidor MCP de Quilt soporta un sistema de plugins que permite extender
las capacidades del agente sin modificar el código core. Los plugins pueden:

- Añadir nuevas tools para que el agente las use
- Proporcionar resources (fuentes de datos)
- Enviar notificaciones asíncronas al cliente

```
┌─────────────────────────────────────────────────────┐
│                   McpServer                          │
│                                                      │
│  ┌─────────────┐    ┌─────────────────────────┐   │
│  │ Built-in    │    │    PluginRegistry        │   │
│  │ Tools       │    │                          │   │
│  │             │    │  ┌────────────────────┐  │   │
│  │ - query     │    │  │ GitPlugin          │  │   │
│  │ - search    │    │  │ name: "git"        │  │   │
│  │ - create    │    │  │ tools: [status,    │  │   │
│  │ - ...       │    │  │        log, diff]   │  │   │
│  └─────────────┘    │  └────────────────────┘  │   │
│         │            │           │               │   │
│         │            │  ┌────────────────────┐  │   │
│         │            │  │ Future: Zotero     │  │   │
│         └──────────┬──│  │ Future: PDF        │  │   │
│                    │  │  └────────────────────┘  │   │
│                    │  └─────────────────────────┘   │
│                    │                                │
│         ┌──────────┴────────────────────────────┐   │
│         │  Tool Dispatch (execute_tool)          │   │
│         │                                        │   │
│         │  1. Check built-in tools               │   │
│         │  2. If not found → PluginRegistry      │   │
│         │  3. Lookup tool in index               │   │
│         │  4. Dispatch to plugin.execute_tool() │   │
│         └────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

### 7.2 Plugin Trait Contract

Para crear un plugin, implementa el trait `Plugin`:

```rust
use quilt_mcp::plugin::{Plugin, PluginContext, PluginError};

pub struct MyPlugin {
    name: String,
    version: String,
}

impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        &self.name  // "my_plugin"
    }

    fn version(&self) -> &str {
        &self.version  // "0.1.0"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "my_plugin::do_something".to_string(),
                description: "Does something useful".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "arg1": { "type": "string" }
                    }
                }),
            }
        ]
    }

    fn execute_tool(
        &self,
        name: &str,
        args: serde_json::Value
    ) -> Result<serde_json::Value, PluginError> {
        match name {
            "my_plugin::do_something" => {
                // Tool logic here
                Ok(serde_json::json!({ "result": "done" }))
            }
            _ => Err(PluginError::NotFound(name.to_string())),
        }
    }
}
```

### 7.3 Registro de Plugins

Los plugins se registran en el `PluginRegistry`:

```rust
use std::sync::Arc;

let mut registry = PluginRegistry::new();
registry.register(Arc::new(MyPlugin { name: "my_plugin".to_string(), version: "0.1.0".to_string() })).unwrap();
```

### 7.4 Plugin Git Incorporado

El plugin `quilt-git-extension` proporciona tools de solo lectura para git:

| Tool | Descripción | Argumentos |
|------|-------------|------------|
| `git::status` | Estado actual del repo | None |
| `git::log` | Commits recientes | `max_count`, `author` |
| `git::diff` | Cambios no commiteados | `staged`, `path` |
| `git::blame` | Información de autor por línea | `path` (requerido) |

**Ejemplo de uso:**
```json
{
  "name": "git::status",
  "arguments": {}
}
```
```json
{
  "branch": "main",
  "clean": false,
  "staged": ["src/main.rs"],
  "modified": ["Cargo.toml"],
  "untracked": ["notes.md"]
}
```

### 7.5 Convenciones para Plugins de Comunidad

Para crear un plugin comunitario:

1. **Nombre del crate**: `quilt-{plugin-name}-extension` (e.g., `quilt-zotero-extension`)
2. **Nombre del plugin**: Usar el dominio sin prefijos (e.g., "zotero", no "quilt_zotero")
3. **Tool names**: `{plugin_name}::{tool_name}` (e.g., `zotero::search`)
4. **Documentación**: Incluir README con examples de uso
5. **Tests**: Tests de integración usando temp directories

### 7.6 Puntos de Extensión Futuros

| Plugin | Descripción | Dependencias |
|--------|-------------|--------------|
| PDF | Extracción de metadata y texto de PDFs | `pdf-extract` o `lopdf` |
| LaTeX | Parsing y extracción de ecuaciones | `tex-parser` |
| Zotero | Integración con biblioteca de referencias | `zotero-api` |
| ArXiv | Búsqueda y descarga de papers | `arxiv-api` |

---

### 7.7 Sistema de Hooks — Reacción a Eventos del Grafo

El sistema de hooks permite a los plugins reaccionar a cambios en el knowledge graph (bloques creados, páginas eliminadas, transacciones committed, etc.). Mientras `Plugin::notifications()` cubre la dirección servidor→cliente, los hooks cubren la dirección core→plugin.

```
┌──────────────────────────────────────────────────────────────┐
│                     Core Domain                                │
│                                                               │
│  Block created ──┐                                           │
│  Page deleted ──┼──► HookDispatcher ──► Plugin::on_hook()    │
│  Tx committed ──┘          │                                 │
│                             │                                 │
│              ┌──────────────┴──────────────┐                 │
│              │                             │                 │
│         Priority                       Filters               │
│         Ordering                       (block_ids,           │
│                                      page_ids,              │
│                                      content_contains)       │
└──────────────────────────────────────────────────────────────┘
```

#### 7.7.1 Hooks Disponibles

| Hook | Descripción | Payload |
|------|-------------|---------|
| `BlockChanged` | Bloque creado, actualizado o eliminado | `BlockPayload { id, page_id, change_type, content }` |
| `PageChanged` | Página creada, actualizada, eliminada o renombrada | `PagePayload { id, name, change_type }` |
| `DbTransaction` | Transacción de base de datos commiteada | `TransactionPayload { tx_id, block_changes[], page_changes[], committed_at }` |
| `SearchIndexUpdated` | Índice de búsqueda actualizado | `SearchIndexPayload { update_type, blocks_affected, pages_affected }` |

#### 7.7.2 Tipos de Cambio

Cada hook incluye un `ChangeType` que indica la naturaleza del cambio:

| Tipo | Descripción |
|------|-------------|
| `Created` | Entidad fue creada |
| `Updated` | Entidad fue actualizada |
| `Deleted` | Entidad fue eliminada |

#### 7.7.3 Cómo Suscribirse a Hooks

Implementa `subscribed_hooks()` y `on_hook()` en tu plugin:

```rust
use quilt_mcp::hooks::{
    HookEvent, HookEventKind, HookSubscription, HookPayload,
    BlockPayload, Priority, ChangeType,
};
use quilt_mcp::plugin::{Plugin, PluginError, PluginContext};

pub struct BlockTracker {
    name: String,
    version: String,
}

impl Plugin for BlockTracker {
    fn name(&self) -> &str {
        &self.name  // "block_tracker"
    }

    fn version(&self) -> &str {
        &self.version  // "0.1.0"
    }

    /// Declare which hooks this plugin wants to receive
    fn subscribed_hooks(&self) -> Vec<HookSubscription> {
        vec![
            HookSubscription {
                event: HookEventKind::BlockChanged,
                priority: Priority::NORMAL,
                filter: None,  // Receive all block changes
            },
            HookSubscription {
                event: HookEventKind::PageChanged,
                priority: Priority::HIGH,
                filter: Some(HookFilter {
                    block_ids: None,
                    page_ids: None,
                    content_contains: Some(vec!["TODO".to_string(), "FIXME".to_string()]),
                }),
            },
        ]
    }

    /// Handle hook events
    fn on_hook(&self, event: HookEvent) -> Result<(), HookError> {
        match event {
            HookEvent::BlockChanged(payload) => {
                tracing::info!(
                    block_id = %payload.id,
                    change = %payload.change_type.name(),
                    "Block changed"
                );
            }
            HookEvent::PageChanged(payload) => {
                tracing::info!(
                    page_id = %payload.id,
                    change = %payload.change_type.name(),
                    "Page changed"
                );
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value, PluginError> {
        Err(PluginError::NotFound(name.to_string()))
    }
}
```

#### 7.7.4 Filtros de Hook

Los filtros permiten suscribirse a un subconjunto de eventos:

```rust
HookSubscription {
    event: HookEventKind::BlockChanged,
    priority: Priority::NORMAL,
    filter: Some(HookFilter {
        // Solo bloques específicos
        block_ids: Some(vec!["block-123".to_string(), "block-456".to_string()]),
        // Solo páginas específicas
        page_ids: Some(vec!["page-abc".to_string()]),
        // Solo si el contenido contiene ciertas cadenas
        content_contains: Some(vec!["TODO".to_string()]),
    }),
}
```

#### 7.7.5 Prioridades de Dispatch

Los plugins se ejecutan en orden de prioridad (mayor primero). Prioridades reservadas:

| Prioridad | Rango | Uso |
|-----------|-------|-----|
| `SYSTEM` | 100 | Logging de auditoría, sistemas críticos |
| `HIGH` | 75 | Indexación de búsqueda |
| `NORMAL` | 50 | Plugins normales (default) |
| `LOW` | 25 | Plugins no-críticos |

```rust
HookSubscription {
    event: HookEventKind::BlockChanged,
    priority: Priority::HIGH,  // Se ejecuta antes que NORMAL
    filter: None,
}
```

#### 7.7.6 Aislamiento de Panics

Si un plugin hace panic en `on_hook()`, el panic es capturado y el dispatch continúa a los siguientes plugins. El panic se registra como error en el `HookResult`:

```rust
// Plugin que hace panic
fn on_hook(&self, event: HookEvent) -> Result<(), HookError> {
    panic!("intentional panic for testing");
}

// Resultado: HookResult { success: false, error: Some("Plugin panicked: intentional panic...") }
```

---

## 7b. Próximos Pasos de Implementación

1. [ ] `quilt_query` + `quilt_search` — Base (Fase 1 MVP)
2. [ ] `quilt_cognitive_mirror` — Primera capacidad diferenciadora
3. [ ] `quilt_serendipity` — Notificaciones pasivas de conexiones
4. [ ] `quilt_argument_map` — Estructuración de debates
5. [ ] `quilt_mental_model` — Modelos mentales vivos
6. [ ] `quilt_roundtable` — Multi-agente debate
7. [ ] Agent Memory — Aprendizaje del perfil cognitivo
