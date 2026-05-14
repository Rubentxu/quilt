# Algoritmos de Layout y Librerías

## Opciones de Layout

### 1. Force-Directed (Recomendado para MVP)

```
┌─────────────────────────────────────────────────────────┐
│                    Force-Directed Layout                  │
│                                                          │
│    ○──────○              Características:                │
│   /        \            - Simula física (fuerzas)        │
│  ○          ○──○        - Agrupa nodos relacionados     │
│   \        /            - No requiere posiciones pre     │
│    ○──○───○             - Computación iterativa         │
│                                                          │
│   Pros: Visualización intuitiva, automático              │
│   Cons: Puede ser lento con 1000+ nodos                  │
└─────────────────────────────────────────────────────────┘
```

**Algoritmo**:
1. Cada nodo tiene carga (repulsión)
2. Cada arista tiene resorte (atracción)
3. Iteraciones hasta estabilización o límite

### 2. Dagre (Directed Acyclic Graph)

```
┌─────────────────────────────────────────────────────────┐
│                       Dagre Layout                       │
│                                                          │
│    A ─────▶ B ─────▶ C                                  │
│    │                                                │
│    └────────▶ D ─────▶ E                               │
│                                                          │
│   Pros: Rápido, predecible                              │
│   Cons: Solo para grafos acíclicos                      │
└─────────────────────────────────────────────────────────┘
```

**No recomendado** para vista grafo general porque Quilt tiene referencias cíclicas.

### 3. Hierarchical / Tree

```
┌─────────────────────────────────────────────────────────┐
│                    Hierarchical                          │
│                                                          │
│                        ○                                 │
│                       /|\                                │
│                      ○ ○ ○                               │
│                     /|   |\                              │
│                    ○ ○   ○ ○                             │
│                                                          │
│   Pros: Fácil de navegar                                │
│   Cons: No muestra conexiones laterales                   │
└─────────────────────────────────────────────────────────┘
```

## Librerías JavaScript/WASM

### D3.js Force (⭐ Recomendado)

**Pros**:
- maduro y bien documentado
-广泛使用 en la industria
- Compatible con WASM via web-sys

**Cons**:
- Solo layout, no render
- Bundle size ~60KB

```javascript
// Ejemplo de uso
const simulation = d3.forceSimulation(nodes)
  .force("link", d3.forceLink(edges).id(d => d.id))
  .force("charge", d3.forceManyBody().strength(-300))
  .force("center", d3.forceCenter(width/2, height/2));

simulation.on("tick", () => {
  // Actualizar posiciones
});
```

### Cytoscape.js

**Pros**:
- Todo-en-uno (layout + render + interactions)
- buen rendimiento con grafos grandes
- WASM bindings disponibles

**Cons**:
- Licencia GPL (no es problema para Quilt)
- Curva de aprendizaje

### Sigma.js

**Pros**:
- Optimizado para grafos grandes (canvas)
- Renderizado WebGL disponible
- Rápido con 1000+ nodos

**Cons**:
- Layout algorithms limitados
- Menos intuitivo que D3

### Vis.js Network

**Pros**:
- Fácil de usar
- Buenos defaults
- Renderizado canvas

**Cons**:
- Menos flexible
- No muy activo actualmente

## Librerías Rust/WASM Puras

### petgraph

```rust
// crates/quilt-cognitive/src/cognitive_mirror/graph.rs ya usa conceptos similares
use petgraph::graph::{DiGraph, NodeIndex};

let mut graph = DiGraph::new();
let a = graph.add_node("page_a");
let b = graph.add_node("page_b");
graph.add_edge(a, b, "ref");
```

**Pros**: Integración Rust nativa
**Cons**: No tiene layout algorithms, solo estructura de datos

### force-graph (WASM)

Port de force-graph (JavaScript) a WASM:
- <https://github.com/vasturiano/force-graph>

## Recomendación para Quilt

### Opción A: D3.js + SVG (MVP)

```rust
// En Leptos component
use web_sys::{SvgElement, Element};
use js_sys::Math;

// D3 layout en JS, llamado desde Rust
#[wasm_bindgen]
extern "C" {
    fn runForceLayout(nodes: JsValue, edges: JsValue) -> JsValue;
}
```

**Esfuerzo**: ~3-5 días
**Resultado**: MVP funcional

### Opción B: Sigma.js (Producción)

```rust
// Sigma.js tiene mejor rendimiento con grafos grandes
// Requiere más setup inicial
```

**Esfuerzo**: ~5-7 días
**Resultado**: Mejor rendimiento

### Opción C: Hybrid (⭐ Recomendado)

1. Backend: Usar `LightweightGraph` existente para computar
2. Frontend: D3.js solo para layout calculation (no render)
3. Render: SVG/Canvas propio en Leptos

**Esfuerzo**: ~4-6 días
**Resultado**: Control total, mejor integración Rust

## Parámetros de Layout Sugeridos

```rust
// Parámetros para force-directed (basado en Logseq)
const LAYOUT_CONFIG = {
    linkDistance: 100,      // Longitud de arista
    chargeStrength: -300,   // Repulsión (más negativo = más spread)
    collisionRadius: 30,    // Radio de colisión
    centerStrength: 0.1,   // Fuerza hacia el centro
    alphaDecay: 0.02,       // Decaimiento de simulación
    velocityDecay: 0.4,    // Amortiguación de velocidad
};
```

## Performance con Diferentes Tamaños

| Nodos | Aristas | Librería Recomendada | FPS Estimado |
|-------|---------|---------------------|--------------|
| < 100 | < 500 | D3.js + SVG | 60fps |
| 100-500 | 500-2500 | D3.js + Canvas | 30-60fps |
| 500-1000 | 2500-10000 | Sigma.js | 30-60fps |
| > 1000 | > 10000 | Sigma.js + clustering | 15-30fps |

## Estrategia de Optimización

1. **Niveles de detalle**:
   - Zoom out: Mostrar solo clusters
   - Zoom in: Mostrar nodos individuales

2. **Virtualización**:
   - Solo renderizar nodos en viewport
   - Ocultar aristas de nodos fuera de vista

3. **Throttling**:
   - Limit updates durante drag a 30fps
   - Actualizar posiciones cada 16ms

4. **Caching**:
   - Cache layout calculation
   - Recalcular solo cuando datos cambian

## Implementación Paso a Paso

```
Semana 1:
  □ Día 1-2: Extender resource_graph para devolver {nodes, edges}
  □ Día 3-4: Crear GraphDto y testear con curl
  □ Día 5: Integrar con bridge.rs

Semana 2:
  □ Día 1-2: Crear componente GraphView básico
  □ Día 3-4: Integrar D3.js force layout (WASM o JS interop)
  □ Día 5: Zoom y pan básicos

Semana 3:
  □ Día 1-2: Click to navigate
  □ Día 3-4: Filtros y búsqueda
  □ Día 5: Testing y polish
```
