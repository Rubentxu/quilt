# outliner

## Visão Geral
Core del sistema outliner de Logseq (`deps/outliner`). Implementa todas las operaciones CRUD sobre bloques jerárquicos (árboles) con validación estricta, transacciones atómicas sobre DataScript y preservación de la integridad referencial. Es el motor que permite insertar, eliminar, mover, indentar y reordenar bloques dentro de la estructura de outliner.

## Responsabilidades
- Guardar bloques (`save-block`) con validación de integridad de UUID y título de página
- Insertar bloques (`insert-blocks`) como siblings o children con cálculo de niveles y órdenes
- Eliminar bloques (`delete-blocks`) con manejo de páginas huérfanas y referencias
- Mover bloques (`move-blocks`) con validación anti-circular y reasignación de parent/order
- Mover bloques arriba/abajo (`move-blocks-up-down`) entre siblings
- Indentar/outdentar bloques (`indent-outdent-blocks`) con validación de límites
- Construir árboles desde listas planas de bloques (`blocks->vec-tree`)
- Aplanar árboles a listas (`tree-vec-flatten`)
- Validar operaciones contra reglas de negocio: bloques built-in, títulos inválidos, movimientos circulares
- Ordenar siblings con orden lexicográfico (`db-order/gen-key`)

## Interface

### Operaciones principales

```clojure
;; Guardar bloque
(save-block db block opts)
;; db:    DataScript DB  — snapshot actual de la BD
;; block: Map           — mapa del bloque a guardar
;; opts:  Map           — {:keys [update-page-timestamps? delete-blocks-fn ...]}
;; Retorna: {:tx-data [TxDatum]} — datos de transacción para aplicar

;; Insertar bloques
(insert-blocks db blocks target-block opts)
;; blocks:       [BlockMap]  — bloques a insertar (con :block/level)
;; target-block: Map?        — bloque destino (nil = página raíz)
;; opts:         Map         — {:keys [sibling? keep-uuid? ...]}
;; Retorna:      {:tx-data [TxDatum] :blocks [BlockMap]}

;; Eliminar bloques
(delete-blocks db blocks opts)
;; blocks: [BlockMap]  — bloques a eliminar
;; opts:   Map         — {:keys [children? ...]}
;; Retorna: {:tx-data [TxDatum]}

;; Mover bloques
(move-blocks conn blocks target-block opts)
;; conn:  DataScript Conn
;; Retorna: nil (efectos secundarios vía transacción)

;; Mover arriba/abajo
(move-blocks-up-down conn blocks up?)
;; Retorna: nil

;; Indentar/outdentar
(indent-outdent-blocks conn blocks indent?)
;; Retorna: nil
```

### Tipos de datos

**BlockMap:**
```clojure
{:db/id          Int       ;; ID en DataScript
 :block/uuid     UUID      ;; UUID inmutable
 :block/order    String    ;; orden lexicográfico
 :block/parent   {:db/id Int}  ;; bloque padre
 :block/page     {:db/id Int}  ;; página contenedora
 :block/level    Int?      ;; nivel jerárquico
 :block/title    String    ;; contenido visible
 :block/collapsed? Boolean?}
```

**OutlinerOp:**
```clojure
{:op   Keyword   ;; :save-block | :insert-blocks | :delete-blocks | :move-blocks | ...
 :args [Any]     ;; argumentos de la operación}
```

## Regras de Negócio
- UUID de bloque no puede cambiar una vez creado (validado en `-save`) 🟢
- Bloques built-in (built-in pages como "Contents", "logseq/custom.css") no pueden ser modificados ni eliminados 🟢
- No se puede mover un bloque a sus propios hijos (detección de movimiento circular) 🟢
- No se puede mover un bloque a sí mismo como target 🟢
- Orden entre siblings es lexicográfico generado vía `db-order/gen-key` (permite inserción entre dos órdenes existentes) 🟢
- Al eliminar bloques, se filtran solo los top-level (los hijos se eliminan implícitamente vía `retractEntity`) 🟢
- Bloques non-consecutivos en delete/move se ordenan y procesan como grupo 🟢
- Páginas que quedan huérfanas tras delete se mandan a recycle (no se eliminan permanentemente) 🟡
- Referencias huérfanas (bloques referenciados por bloques eliminados) se limpian automáticamente 🟡
- Título de página no puede contener: `/`, `#`, `?`, `:`, `|`, `<`, `>`, `*`, `"`, `\` 🟢
- Título de página no puede ser vacío ni solo números 🟢
- Propiedades de tipo closed-value (task markers, priority) tienen valores restringidos predefinidos 🟢

## Fluxo Principal

### save-block
1. Recibir `db`, `block` y `opts`
2. Validar que el bloque no sea built-in
3. Validar que el UUID no esté siendo modificado (comparar con entidad existente en DB)
4. Si el bloque es página (`block/page` = nil y `block/name` presente), validar título:
   - No vacío, no solo números
   - Sin caracteres prohibidos: `/`, `#`, `?`, `:`, `|`, `<`, `>`, `*`, `"`, `\`
5. Remover campos temporales (`dissoc-temp-fields`)
6. Actualizar `:block/updated-at` al timestamp actual
7. Corregir tag IDs (`fix-tag-ids`)
8. Remover clases inline no permitidas (`remove-disallowed-inline-classes`)
9. Construir `:tx-data` con `:db/add` para cada atributo modificado
10. Opcionalmente actualizar timestamps de página (`update-page-timestamps`)
11. Retornar `{:tx-data [...]}`

### insert-blocks
1. Recibir `blocks` (con niveles), `target-block` y `opts`
2. Determinar si insertar como sibling o child del target:
   - Si `sibling?` = true y `target-block` no es página raíz → nuevo parent = parent del target
   - Si `sibling?` = false → nuevo parent = target
3. Para cada bloque a insertar, calcular `:block/level` relativo
4. Generar órdenes lexicográficos vía `gen-n-keys` posicionados entre siblings existentes
5. Construir `build-insert-blocks-tx`: asociar `:block/parent`, `:block/page`, `:block/order`
6. Asignar IDs temporales (`assign-temp-id`) para referencias entre bloques nuevos
7. Si hay templates, expandir referencias de template
8. Retornar `{:tx-data [...] :blocks [...]}` con los bloques ya enriquecidos

### delete-blocks
1. Recibir `blocks` y `opts`
2. Filtrar top-level blocks (`filter-top-level-blocks`) — hijos se eliminan en cascada
3. Si los bloques no son consecutivos, ordenarlos (`sort-by-order`)
4. Validar que ningún bloque sea built-in
5. Si algún bloque tiene `default-value-property`, reemplazar con placeholder vacío
6. Para el resto, ejecutar `retractEntity` en batch
7. Si la página queda sin bloques (huérfana), enviar a recycle
8. Limpiar referencias huérfanas (`clean-orphaned-refs`)
9. Retornar `{:tx-data [...]}`

### move-blocks
1. Recibir `blocks`, `target-block` y `opts`
2. Filtrar top-level blocks
3. Obtener target block y determinar si es sibling o child
4. **Validar**: ningún bloque a mover es ancestro del target (detección circular)
5. **Validar**: el target no está entre los bloques a mover
6. Para cada bloque:
   a. Si cambia de página, mover recursivamente todos sus hijos
   b. Recalcular `:block/parent` y `:block/page`
   c. Ajustar `:block/order` para posición entre siblings destino
   d. Si el bloque tiene propiedad `created-from`, preservarla
7. Ejecutar transacción batch con todos los cambios

## Fluxos Alternativos
- **[Intento de modificar bloque built-in]:** Se lanza `ex-info` con mensaje "Cannot modify built-in entity" y la operación se aborta 🟢
- **[Intento de mover bloque a sus propios hijos]:** Se lanza `ex-info` con mensaje "Cannot move block to its own children" y la operación se aborta 🟢
- **[Inserción en página raíz (sin target)]:** Si `target-block` es nil, los bloques se insertan al final de la página como top-level blocks 🟢
- **[Indentación de bloque sin left-sibling]:** Si se intenta indentar el primer bloque de una página, se lanza error "Cannot indent — no left sibling" 🟢
- **[Outdentación de bloque top-level]:** Si se intenta outdentar un bloque que ya está en nivel 1, se lanza error "Cannot outdent — already at top level" 🟡
- **[Delete de bloque con children]:** Los hijos se eliminan implícitamente por `retractEntity` de DataScript; el batch incluye solo los top-level 🟢
- **[Move entre páginas distintas]:** Si el target pertenece a otra página, todos los bloques movidos y sus hijos cambian `:block/page` a la nueva página 🟡

## Dependências
- `datascript.core` — transacciones (`d/transact!`, `d/entity`, `d/datoms`)
- `logseq.db` — `ldb/get-block`, `ldb/page-exists?`, `ldb/transact!`
- `logseq.db.common.order` — `gen-key`, `gen-n-keys` para orden lexicográfico
- `malli.core` — validación de schemas de operaciones

## Requisitos Não Funcionais

| Tipo | Requisito inferido | Evidência no código | Confiança |
|------|--------------------|---------------------|-----------|
| Integridade | UUID de bloque inmutable validado en cada save | `deps/outliner/src/logseq/outliner/core.cljs:316-321` | 🟢 |
| Segurança | Built-in entities protegidas contra modificaciones | `deps/outliner/src/logseq/outliner/core.cljs:464-468` | 🟢 |
| Consistência | Detección de movimientos circulares entre bloques | `deps/outliner/src/logseq/outliner/core.cljs:962-968` | 🟢 |
| Atomicidade | Todas las operaciones usan transacciones batch de DataScript | `deps/outliner/src/logseq/outliner/transaction.cljc` | 🟢 |

> Inferido a partir del código. Validar con equipo de operaciones.

## Critérios de Aceitação

### Cenário: Guardar bloque con modificación de título
```gherkin
Dado un bloque existente con UUID "550e8400-..." y título "Old title"
  Y el bloque no es built-in
Cuando se llama a `save-block` con `{:block/uuid "550e8400-..." :block/title "New title"}`
Então el `:tx-data` retornado contiene `[:db/add block-id :block/title "New title"]`
  Y contiene `[:db/add block-id :block/updated-at <current-timestamp>]`
  Y el UUID no aparece en la transacción (no se modifica)
```

### Cenário: Insertar bloques como children de un target
```gherkin
Dado un bloque target con `:db/id` = 42 en la página "My Page"
  Y bloques a insertar: `[{:block/title "Child 1" :block/level 2} {:block/title "Child 2" :block/level 2}]`
Cuando se llama a `(insert-blocks db blocks target {:sibling? false})`
Então los bloques insertados tienen `:block/parent` = `{:db/id 42}`
  Y `:block/page` = página de target
  Y `:block/order` es un orden lexicográfico entre los siblings existentes de target
  Y se retornan los bloques con sus IDs temporales asignados
```

### Cenário: Eliminar bloque con validación anti-built-in
```gherkin
Dado un bloque que pertenece a la página built-in "Contents"
Cuando se llama a `delete-blocks` con ese bloque
Então se lanza excepción `ex-info` con tipo `:built-in-entity`
  Y el mensaje contiene "Cannot modify built-in entity"
  Y no se genera ninguna transacción
```

### Cenário: Prevenir movimiento circular
```gherkin
Dado un bloque A con hijo B, y B con hijo C
Cuando se intenta `(move-blocks conn [A] C {:sibling? false})` (mover A bajo C)
Então se lanza excepción `ex-info`
  Y el mensaje contiene "circular" o "cannot move to child"
  Y ningún bloque cambia de posición
```

### Cenário: Indentar bloque correctamente
```gherkin
Dado dos bloques siblings: Bloque X (nivel 1) y Bloque Y (nivel 1, inmediatamente después de X)
Cuando se llama a `(indent-outdent-blocks conn [Y] true)` (indentar Y)
Então Y pasa a ser hijo de X (`:block/parent` = X, `:block/level` = 2)
  Y `:block/order` de Y se recalcula entre los hijos existentes de X
  Y X se marca como expandido si estaba collapsed
```

## Prioridade

| Requisito | MoSCoW | Justificativa |
|-----------|--------|---------------|
| save-block | Must | Operación fundamental — cada edición de texto la invoca |
| insert-blocks | Must | Crear nuevos bloques es operación core del outliner |
| delete-blocks | Must | Eliminar bloques es parte del flujo normal de edición |
| move-blocks | Must | Drag & drop y reorganización dependen de esta operación |
| Validación anti-built-in | Must | Protege la integridad del sistema |
| Validación anti-circular | Must | Previene corrupción de la estructura de árbol |
| indent-outdent-blocks | Should | Importante para organización pero con alternativa (move) |
| move-blocks-up-down | Should | Conveniencia de usuario, se puede emular con move-blocks |
| Orden lexicográfico | Must | Requerido para mantener orden entre siblings sin reindexar |
| Cleanup de referencias huérfanas | Should | Importante para integridad pero no bloqueante |

> Prioridad inferida por frecuencia de llamada y posición en la cadena de dependencias.

## Cenários de Borda

### 1. Eliminación del último bloque de una página
**Situação:** Una página tiene un solo bloque y el usuario lo elimina.
**Comportamento esperado:**
- El bloque se elimina normalmente
- La página queda sin bloques (huérfana)
- La página se envía a recycle (papelera), no se elimina permanentemente
- Si la página era un journal, se preserva la estructura de journal-day
- La página puede ser restaurada desde recycle

### 2. Inserción masiva de bloques con niveles mixtos
**Situação:** Se insertan 50+ bloques de una vez con niveles que varían entre 1 y 6.
**Comportamento esperado:**
- `blocks-with-level` calcula el nivel correcto para cada bloque
- Si un bloque de nivel N+1 se inserta justo después de uno de nivel N, se convierte en child (indentado)
- Si un bloque de nivel N se inserta después de uno de nivel N+2, el nuevo bloque NO se convierte en child del anterior (vuelve al nivel del ancestro correcto)
- Los órdenes lexicográficos permiten hasta 50+ siblings sin colisiones

### 3. Transacción con referencias entre bloques nuevos
**Situação:** Se insertan dos bloques A y B donde A contiene `[[referencia a B]]`.
**Comportamento esperado:**
- `assign-temp-id` asigna IDs temporales a ambos bloques
- La referencia de A a B se resuelve usando el ID temporal de B
- Tras `d/transact!`, los IDs temporales se reemplazan por IDs reales de DataScript
- La referencia queda correctamente establecida en la entidad persistida

## Rastreabilidade de Código

| Arquivo | Função / Classe | Cobertura |
|---------|-----------------|-----------|
| `deps/outliner/src/logseq/outliner/core.cljs` | `save-block` | 🟢 |
| `deps/outliner/src/logseq/outliner/core.cljs` | `insert-blocks` | 🟢 |
| `deps/outliner/src/logseq/outliner/core.cljs` | `delete-blocks` | 🟢 |
| `deps/outliner/src/logseq/outliner/core.cljs` | `move-blocks` | 🟢 |
| `deps/outliner/src/logseq/outliner/core.cljs` | `move-blocks-up-down` | 🟢 |
| `deps/outliner/src/logseq/outliner/core.cljs` | `indent-outdent-blocks` | 🟢 |
| `deps/outliner/src/logseq/outliner/core.cljs` | `tree-vec-flatten` | 🟢 |
| `deps/outliner/src/logseq/outliner/core.cljs` | `blocks-with-level` | 🟢 |
| `deps/outliner/src/logseq/outliner/tree.cljs` | `blocks->vec-tree` | 🟢 |
| `deps/outliner/src/logseq/outliner/tree.cljs` | `filter-top-level-blocks` | 🟢 |
| `deps/outliner/src/logseq/outliner/tree.cljs` | `block-entity->map` | 🟢 |
| `deps/outliner/src/logseq/outliner/op.cljs` | `apply-ops!` | 🟢 |
| `deps/outliner/src/logseq/outliner/transaction.cljc` | Batch transaction logic | 🟡 |
| `deps/outliner/src/logseq/outliner/op/construct.cljc` | Operation construction | 🟡 |
