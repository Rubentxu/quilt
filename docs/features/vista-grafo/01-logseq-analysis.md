# Análisis de Vista Grafo de Logseq

## Resumen

Logseq implementa su vista grafo en **ClojureScript** usando **D3.js** para el layout force-directed. El código fuente está en `src/main/frontend/components/graph.cljs` (~987 líneas).

## Estructura de Datos

### Modelo de Datos
- **Nodos** = Páginas (pages)
- **Aristas** = Referencias (backlinks/references)

```clojure
;; Estructura simplificada del estado del grafo
(defstate graph-db
  {:nodes [...]   ; todas las páginas
   :edges [...]}) ; todas las referencias
```

### Obtención de Datos
Logseq usa el recurso `logseq://graph` que devuelve:
```json
{
  "pages": [...],      // Lista de páginas  
  "journals": [...],   // Páginas journal
  "blocks": [...],     // Bloques con refs
  "graphs-txids": {}   // Transacciones
}
```

## Algoritmo de Layout

Logseq usa **D3.js force simulation** con las siguientes fuerzas:

```javascript
// Fuerzas típicas en d3-force
d3.forceSimulation(nodes)
  .force("link", d3.forceLink(edges).distance(100))
  .force("charge", d3.forceManyBody().strength(-300))
  .force("center", d3.forceCenter(width/2, height/2))
  .force("collision", d3.forceCollide().radius(30))
```

### Parámetros Comunes
| Fuerza | Valor Típico | Propósito |
|--------|---------------|-----------|
| link.distance | 100px | Longitud de aristas |
| charge.strength | -300 | Repulsión entre nodos |
| collision.radius | 30px | Evita superposición |
| center | centro del SVG | Agrupa el grafo |

## Componentes UI

### Controles de Visualización
1. **Zoom/Pan**: `d3.zoom()` con escala 0.1x - 4x
2. **Filtros**: Mostrar/ocultar por tipo (journals, páginas regulares)
3. **Búsqueda**: Filtrar nodos por nombre
4. **Hover**: Mostrar preview del contenido
5. **Click**: Navegar a la página

### Estados de Nodos
```clojure
{:type :page          ; tipo de nodo
 :page/name "..."
 :graph/node-x 100    ; posición calculada
 :graph/node-y 200
 :selected? false}
```

### Interacciones
- **Drag**: Mover nodos individualmente
- **Zoom**: Scroll del mouse
- **Pan**: Arrastrar el fondo
- **Click**: Seleccionar nodo
- **Doble click**: Navegar a la página
- **Hover**: Mostrar tooltip

## Performance

Logseq maneja grafos grandes con:
1. **Niveles de zoom**: Más detalles al hacer zoom in
2. **Virtualización**: Solo renderizar nodos visibles
3. **Lazy loading**: Cargar datos del grafo bajo demanda
4. **Throttling**: Limitar actualizaciones durante drag

### Límites Típicos
- 100-200 nodos: Rendering fluido
- 500+ nodos: Requiere optimización
- 1000+ nodos: Considerar filtros o clustering

## Código Relevante (graph.cljs)

```clojure
;;获取所有页面和引用构建grafo
(defn build-graph [db]
  (let [pages (get-all-pages db)
        blocks (get-all-blocks db)
        edges (collect-refs blocks)]
    {:nodes pages
     :edges edges}))

;; Force simulation setup
(defmethod render-graph :default [graph-db]
  (let [simulation (d3-force/simulation)]
    (.nodes simulation (:nodes graph-db))
    (.force simulation "link" (d3-force/link (:edges graph-db)))
    (.force simulation "charge" (d3-force/many-body))
    (.restart simulation)))
```

## Comparación con Quilt

| Aspecto | Logseq | Quilt |
|---------|--------|-------|
| Stack | ClojureScript | Rust + Leptos WASM |
| Layout | D3.js (JS) | Por definir |
| Datos | Pages + Blocks | Page + Block + refs |
| Recursos | logseq://graph | resource_graph (incompleto) |
| Render | SVG | SVG o Canvas |

## Referencias

- Código fuente: `src/main/frontend/components/graph.cljs`
- D3.js force: <https://d3js.org/d3-force>
- Documentación de Logseq: <https://docs.logseq.com>
