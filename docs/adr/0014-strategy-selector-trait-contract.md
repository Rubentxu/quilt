# ADR: StrategySelector + StrategyScorer traits en quilt-core (WASM)


## Context

El documento de investigación propuso un selector de estrategias SUNNY-inspired para que Quilt recomiende vistas, comandos, templates y agentes según el contexto del usuario.

La sesión de auto-grill (Q002-P1 + Q009-P2, 2026-06-07) rechazó dos propuestas:
1. Q002-P1: "quilt-application use case" sin trait contract → label sin abstracción, crate placement sin verificar
2. Q009-P2: dos traits redundantes + SelectionContext sin definir + Jaccard mal aplicado + crate placement erróneo

El fork fue resuelto por decisión del arquitecto: traits en quilt-core (WASM), Phase 1 determinista, sin telemetría.

## Decision

**Dos traits en `quilt-core`: `StrategySelector` y `StrategyScorer`. Tipos de dominio en `quilt-core`. Implementaciones Phase 1 en `quilt-application`. Exposición vía WASM (frontend) y MCP (agentes).**

### Traits en quilt-core

```rust
// crates/quilt-core/src/strategy.rs

/// Features del contexto actual del usuario
#[derive(Debug, Clone)]
pub struct ContextFeatures {
    pub content_shape: ContentShape,
    pub graph_shape: GraphShape,
    pub schema_shape: SchemaShape,
    pub usage_context: UsageContext,
}

#[derive(Debug, Clone)]
pub struct ContentShape {
    pub text_length: usize,
    pub has_todo: bool,
    pub has_date: bool,
    pub has_link: bool,
    pub has_tag: bool,
    pub has_property: bool,
}

#[derive(Debug, Clone)]
pub struct GraphShape {
    pub backlinks_count: usize,
    pub outgoing_refs_count: usize,
    pub child_count: usize,
    pub child_depth: u8,
    pub page_age_days: u32,
}

#[derive(Debug, Clone)]
pub struct SchemaShape {
    pub property_keys: Vec<String>,
    pub has_template: bool,
    pub missing_required: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UsageContext {
    pub current_route: String,
    pub is_mobile: bool,
    pub hour_of_day: u8,
}

/// Acción rankeada del portfolio
#[derive(Debug, Clone)]
pub struct RankedAction {
    pub action_id: String,
    pub label: String,
    pub kind: ActionKind,
    pub score: f32,        // 0.0-1.0
    pub rationale: String,
}

#[derive(Debug, Clone)]
pub enum ActionKind {
    View,
    Command,
    Query,
    Template,
    Agent,
    Navigation,
}

/// Evalúa qué tan buena es una acción para un contexto
pub trait StrategyScorer {
    fn score(&self, action: &str, features: &ContextFeatures) -> f32;
}

/// Selecciona y rankea acciones del portfolio
pub trait StrategySelector {
    fn select(
        &self,
        features: &ContextFeatures,
        scorer: &dyn StrategyScorer,
        portfolio: &[String],
    ) -> Vec<RankedAction>;
}
```

### Crate placement

| Componente | Crate | Justificación |
|-----------|-------|---------------|
| `StrategySelector` trait | `quilt-core` | WASM-compatible, sin dependencias externas |
| `StrategyScorer` trait | `quilt-core` | Separación de concerns: scoring ≠ selection |
| `ContextFeatures`, `RankedAction`, `ActionKind` | `quilt-core` | Tipos de dominio, WASM-compatibles |
| `RuleBasedSelector` | `quilt-application` | Implementación Phase 1: reglas determinísticas |
| `FeatureExtractor` | `quilt-application` | Lee contexto desde repositorios |
| `useStrategySuggestions()` | `quilt-ui` | Hook React que llama WASM |
| `quilt_strategy_select` | `quilt-mcp` | Tool MCP para agentes |

### Phase 1: Reglas determinísticas

- Portfolio: 6-8 acciones (daily capture, page creation, task conversion, template suggestion, search, table view, kanban view, graph lens)
- Ranking: reglas ponderadas simples (e.g., "has TODO + has date → task conversion score alto")
- Output: top 3 mostradas como hints no intrusivas
- Sin telemetría, sin persistencia, sin ML
- WASM se ejecuta en browser → sub-100ms latencia

### Phase 2-5 (futuro)

| Phase | Qué | Cuándo |
|-------|-----|--------|
| 2 | Telemetría de outcomes | Cuando haya datos de uso real |
| 3 | k-NN reranking | Con suficientes InteractionCases |
| 4 | Ejecución programada | Con confidence thresholds |
| 5 | Portfolio MCP | Saved Views como tools MCP |

## Considered Options

1. **"Use case" sin trait** (rechazado por Q002-P1) — label sin contrato
2. **Dos traits en quilt-application** (rechazado por Q009-P2) — no WASM-compatible, crate placement erróneo
3. **Dos traits en quilt-core (WASM)** — aceptado: WASM para browser, separación de concerns, MCP-first

## Consequences

- `quilt-core` gana un módulo `strategy.rs` con traits y tipos puros
- `quilt-core` gana `strategy_scoring.rs` con implementación concreta `RelevanceScorer` (4 signals: type-match 0.50, property completeness 0.20, recency 0.15, semantic 0.15) y `ScoredStrategySelector`
- No requiere dependencias externas (sin ML, sin DB)
- Frontend obtiene sugerencias en tiempo real sin round-trip de red
- Agentes MCP usan el mismo selector vía `quilt_strategy_select`
- Separación Scorer/Selector permite testear calidad de ranking independientemente

## Implementation (2026-06-09)

Implementación Phase 1eterminística completa:

```rust
// crates/quilt-core/src/strategy_scoring.rs

/// Scoring signals (4 señales ponderadas, suman 1.0):
/// - type-match: 0.50 (compatibilidad de tipo)
/// - property completeness: 0.20 (completitud de schema)
/// - recency: 0.15 (half-life 24h, RFC 3339 + Unix epoch)
/// - semantic: 0.15 (reservado, neutral 0.5)

pub struct RelevanceScorer {
    pub const TYPE_MATCH_WEIGHT: f32 = 0.50;
    pub const PROPERTY_WEIGHT: f32 = 0.20;
    pub const RECENCY_WEIGHT: f32 = 0.15;
    pub const SEMANTIC_WEIGHT: f32 = 0.15;
}

impl StrategyScorer for RelevanceScorer {
    fn score(&self, action: &str, features: &ContextFeatures) -> f32 { ... }
}

pub struct ScoredStrategySelector { ... }

impl StrategySelector for ScoredStrategySelector {
    fn select(&self, features: &ContextFeatures, scorer: &dyn StrategyScorer, portfolio: &[String]) -> Vec<RankedAction> { ... }
}
```

Tests: 27 inline tests cubriendo todos los signals, weights-sum-to-one invariant, RFC 3339 + Unix epoch parsing, fallback behavior.

## References

- Q002-P1 y Q009-P2 (auto-grill 2026-06-07)
- `docs/research/ux-workflow-portfolio-analysis.md` §SUNNY For Quilt
- SUNNY: `arXiv:1311.3353`, `CP-Unibo/sunny-cp`, JAIR `sunny-as2`
- Resolución del arquitecto (2026-06-07): traits en quilt-core
