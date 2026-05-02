# User Stories — Búsqueda

> **Proyecto**: Logseq
> **Generado por**: reversa-writer
> **Fecha**: 2026-05-02
> **Nivel**: detalhado
> **Escala**: 🟢 CONFIRMADO | 🟡 INFERIDO | 🔴 LACUNA

---

## Flujo 1: Búsqueda de Páginas

### US-BUS-01: Buscar página por nombre exacto 🟢 CONFIRMADO

**Contexto**: El usuario quiere encontrar una página específica escribiendo su nombre en la barra de búsqueda.

**Rastreabilidad**: `src/main/frontend/search.cljs` → `block-search`, `src/main/frontend/search/browser.cljs` → delegación a `thread-api/search-blocks`

```gherkin
Dado que el grafo contiene las páginas "Reuniones", "Reuniones 2025" y "Notas Reunión"
Cuando el usuario escribe "Reuniones" en la barra de búsqueda
Então se retornan resultados que coinciden con "Reuniones"
  Y "Reuniones" aparece como primer resultado (match exacto)
  Y los resultados incluyen score de relevancia (fuzzy search)
  Y la búsqueda es case-insensitive (reuniones = Reuniones)
  Y los resultados se muestran en tiempo real mientras el usuario escribe
```

---

### US-BUS-02: Buscar página con búsqueda difusa (fuzzy search) 🟢 CONFIRMADO

**Contexto**: El usuario escribe un término aproximado y el sistema encuentra coincidencias cercanas.

**Rastreabilidad**: `src/main/frontend/search.cljs` → `block-search` con `fuzzy/search-normalize`, `frontend/common/search-fuzzy`

```gherkin
Dado que existen las páginas "Documentación API", "Documentos Legales", "Documenting Process"
Cuando el usuario escribe "documntacion" (con error tipográfico)
Então el sistema normaliza la query usando `fuzzy/search-normalize`
  Y se retornan resultados relevantes como "Documentación API" y "Documentos Legales"
  Y cada resultado tiene un score de similitud fuzzy
  Y la búsqueda tolera omisiones, transposiciones y errores de tipeo
```

---

### US-BUS-03: Buscar página con acentos (configuración remove-accents) 🟢 CONFIRMADO

**Contexto**: El sistema puede ignorar o respetar acentos según la configuración del usuario.

**Rastreabilidad**: `src/main/frontend/search.cljs` → `block-search` con `enable-search-remove-accents?`, configuración global del grafo

```gherkin
Dado que existe la página "Gestión de Proyectos"
  Y la configuración `enable-search-remove-accents?` está activada
Cuando el usuario busca "gestion"
Então la query se normaliza eliminando acentos
  Y "Gestión de Proyectos" aparece en los resultados
  Y "gestión" también matchea porque la entrada también se normaliza

Dado que `enable-search-remove-accents?` está desactivada
Cuando el usuario busca "gestion" (sin tilde)
Então "Gestión de Proyectos" NO aparece (no hay match exacto)
  Y solo "gestion" sin tilde matchea resultados sin tilde
```

---

### US-BUS-04: Buscar página con motor de búsqueda vía plugin 🟡 INFERIDO

**Contexto**: El sistema soporta motores de búsqueda externos registrados vía Plugin API (patrón Agency).

**Rastreabilidad**: `src/main/frontend/search/agency.cljs` → `query` distribuye a todos los motores, `src/main/frontend/search/plugin.cljs`

```gherkin
Dado que el usuario tiene instalado un plugin de búsqueda (ej: búsqueda semántica)
  Y el plugin registra un motor de búsqueda en la Agency
Cuando el usuario busca "machine learning"
Então la query se envía al Browser engine (nativo) primero
  Y la query también se envía al Plugin engine
  Y los resultados de AMBOS motores se combinan
  Y los resultados del plugin se muestran junto con los del motor nativo
  Y no hay deduplicación entre motores (cada motor es responsable de sus resultados)
```

---

### US-BUS-05: Buscar página cuando el índice está desactualizado 🟢 CONFIRMADO

**Contexto**: El sistema reconstruye el índice de búsqueda cuando es necesario.

**Rastreabilidad**: `src/main/frontend/search.cljs` → `rebuild-indices!`, `frontend/handler/events.cljs` → `schedule-search-index-build!`

```gherkin
Dado que el índice de búsqueda está corrupto o vacío tras un cambio de grafo
Cuando el sistema detecta que `input-idle?` (5 segundos sin actividad del usuario)
Então se programa un `rebuild-indices!` para el repo actual
  Y se reconstruyen tanto el índice de páginas como el de bloques
  Y la reconstrucción se ejecuta en un worker thread para no bloquear la UI
  Y búsquedas posteriores retornan resultados correctos y completos
```

---

## Flujo 2: Búsqueda con Operadores DSL

### US-BUS-06: Buscar bloques con filtro de tarea (task) 🟢 CONFIRMADO

**Contexto**: El usuario busca bloques filtrados por su estado de tarea usando el operador `task`.

**Rastreabilidad**: `src/main/frontend/db/query_dsl.cljs` → operador `task`, `src/main/frontend/db/query_custom.cljs` → `custom-query`

```gherkin
Dado que el grafo contiene bloques con diferentes marcadores de tarea:
  - "Comprar materiales" con marker TODO
  - "Escribir informe" con marker NOW
  - "Revisar código" con marker DONE
Cuando el usuario ejecuta la query `(task TODO NOW)`
Então se retornan solo los bloques con marker TODO o NOW
  Y "Comprar materiales" y "Escribir informe" aparecen en resultados
  Y "Revisar código" (DONE) NO aparece
```

---

### US-BUS-07: Buscar con operadores booleanos compuestos (and, or, not) 🟢 CONFIRMADO

**Contexto**: El usuario combina múltiples condiciones usando operadores booleanos de la DSL.

**Rastreabilidad**: `src/main/frontend/db/query_dsl.cljs` → operadores `and`, `or`, `not`, `parse-query`

```gherkin
Dado que el grafo contiene bloques con propiedades:
  - Bloque A: tags=importante, priority=A
  - Bloque B: tags=importante, priority=B
  - Bloque C: tags=normal, priority=A
Cuando el usuario ejecuta:
  (and (tags "importante") (priority A))
Então se retorna solo el Bloque A (cumple ambas condiciones)
  Y el Bloque B no aparece (priority B, no A)
  Y el Bloque C no aparece (tags normal, no importante)
```

---

### US-BUS-08: Buscar con operador `between` (rango de fechas) 🟢 CONFIRMADO

**Contexto**: El usuario filtra bloques creados o modificados en un rango de fechas específico.

**Rastreabilidad**: `src/main/frontend/db/query_dsl.cljs` → operador `between`, time helpers

```gherkin
Dado que el grafo contiene bloques con diferentes fechas de creación:
  - Notas del 2026-05-01
  - Notas del 2026-05-10
  - Notas del 2026-05-20
Cuando el usuario ejecuta la query:
  (between 2026-05-01 2026-05-15)
Então se retornan bloques con fechas entre el 1 y el 15 de mayo
  Y "Notas del 2026-05-01" y "Notas del 2026-05-10" aparecen
  Y "Notas del 2026-05-20" NO aparece (fuera de rango)
```

---

### US-BUS-09: Buscar con time helpers relativos (today, -7d, +2w) 🟢 CONFIRMADO

**Contexto**: El usuario usa expresiones de tiempo relativas para búsquedas dinámicas.

**Rastreabilidad**: `src/main/frontend/db/query_dsl.cljs` → time helpers (`today`, `-7d`, `+1w`, `-1m`, `-1y`)

```gherkin
Dado que hoy es 2026-05-02
  Y el grafo tiene bloques con fechas:
  - Tarea del 2026-05-02 (today)
  - Tarea del 2026-04-25 (7 días atrás)
  - Tarea del 2026-04-15 (17 días atrás)
Cuando el usuario ejecuta:
  (and (task TODO) (between -7d today))
Então se retornan bloques TODO de los últimos 7 días
  Y la tarea del 2026-05-02 aparece (hoy)
  Y la tarea del 2026-04-25 aparece (dentro del rango)
  Y la tarea del 2026-04-15 NO aparece (fuera del rango)
```

---

### US-BUS-10: Buscar con filtro de propiedad (property key value) 🟢 CONFIRMADO

**Contexto**: El usuario busca bloques que tienen una propiedad específica con un valor determinado.

**Rastreabilidad**: `src/main/frontend/db/query_dsl.cljs` → operador `property`

```gherkin
Dado que existen bloques con propiedades:
  - Bloque A: type:: meeting
  - Bloque B: type:: task
  - Bloque C: type:: meeting, priority:: A
Cuando el usuario ejecuta:
  (property type "meeting")
Então se retornan todos los bloques con propiedad `type` = `meeting`
  Y el Bloque A y el Bloque C aparecen
  Y el Bloque B (type task) NO aparece
```

---

### US-BUS-11: Buscar con `page-ref` para encontrar referencias a una página 🟢 CONFIRMADO

**Contexto**: El usuario busca todos los bloques que referencian a una página específica.

**Rastreabilidad**: `src/main/frontend/db/query_dsl.cljs` → operador `[[page-ref]]`, `frontend/db/model.cljs` → `get-page`

```gherkin
Dado que la página "Proyecto Alpha" es referenciada en múltiples bloques:
  - "Ver [[Proyecto Alpha]] para detalles" en la página "Reuniones"
  - "Relacionado con [[Proyecto Alpha]]" en la página "Notas"
Cuando el usuario ejecuta: [[Proyecto Alpha]]
Então se retornan todos los bloques que contienen `[[Proyecto Alpha]]`
  Y los resultados incluyen los bloques de "Reuniones" y "Notas"
  Y se muestra la página de origen de cada resultado
  Y los backlinks a "Proyecto Alpha" son navegables
```

---

### US-BUS-12: Buscar con `full-text-search` para búsqueda de texto libre 🟢 CONFIRMADO

**Contexto**: El usuario busca cualquier bloque cuyo contenido contenga un texto específico.

**Rastreabilidad**: `src/main/frontend/db/query_dsl.cljs` → operador `full-text-search`

```gherkin
Dado que existen bloques con contenido:
  - "La arquitectura de microservicios requiere..."
  - "El patrón arquitectónico MVC es..."
  - "Diseño de bases de datos relacionales"
Cuando el usuario ejecuta:
  (full-text-search "arquitectura")
Então se retornan los bloques que contienen la palabra "arquitectura"
  Y "La arquitectura de microservicios requiere..." aparece
  Y "El patrón arquitectónico MVC es..." aparece (contiene "arquitectónico")
  Y "Diseño de bases de datos" NO aparece
```

---

### US-BUS-13: Buscar con `sample` para obtener muestra aleatoria 🟢 CONFIRMADO

**Contexto**: El usuario usa el operador `sample` para obtener una cantidad limitada de resultados aleatorios.

**Rastreabilidad**: `src/main/frontend/db/query_dsl.cljs` → operador `sample`

```gherkin
Dado que el grafo tiene 100+ bloques con marker TODO
Cuando el usuario ejecuta: (and (task TODO) (sample 5))
Então se retornan exactamente 5 bloques con marker TODO
  Y los 5 bloques son una selección aleatoria de los 100+
  Y la query se completa en tiempo razonable
  Y ejecutar la misma query de nuevo puede retornar diferentes 5 bloques
```

---

## Flujo 3: Búsqueda de Archivos

### US-BUS-14: Buscar archivo por nombre en el grafo 🟢 CONFIRMADO

**Contexto**: El usuario busca archivos no-Markdown dentro de su grafo (configuraciones, assets, etc.).

**Rastreabilidad**: `src/main/frontend/search.cljs` → `file-search`, filtro de extensión `.md` / `.markdown`

```gherkin
Dado que el grafo contiene los archivos:
  - "config.edn"
  - "readme.md"
  - "notes.org"
  - "diagram.png"
  - "custom.css"
Cuando el usuario ejecuta `file-search` con query "config"
Então se retorna "config.edn"
  Y NO se retorna "readme.md" ni "notes.org" (archivos markdown/org excluidos)
  Y el resultado incluye la ruta relativa del archivo
```

---

### US-BUS-15: Buscar archivo con extensión específica 🟢 CONFIRMADO

**Contexto**: El usuario filtra archivos por tipo usando el nombre de archivo.

**Rastreabilidad**: `src/main/frontend/search.cljs` → `file-search`

```gherkin
Dado que el grafo contiene archivos:
  - "screenshot.png"
  - "logo.png"
  - "document.pdf"
  - "notes.md" (excluido por extensión markdown)
Cuando el usuario busca ".png"
Então se retornan "screenshot.png" y "logo.png"
  Y "document.pdf" no aparece (no coincide)
  Y "notes.md" no aparece (extensión markdown excluida automáticamente)
```

---

### US-BUS-16: Buscar archivo sin resultados (vector vacío) 🟢 CONFIRMADO

**Contexto**: La búsqueda de archivos no encuentra coincidencias y retorna un vector vacío sin errores.

**Rastreabilidad**: `src/main/frontend/search.cljs` → `file-search` retorna `[]` cuando no hay resultados

```gherkin
Dado que el grafo no contiene ningún archivo que coincida con "xyz-nonexistent"
Cuando el usuario ejecuta `file-search` con query "xyz-nonexistent"
Então se retorna un vector vacío `[]`
  Y no se lanza ninguna excepción
  Y la UI muestra un mensaje de "Sin resultados" o equivalente
```

---

### US-BUS-17: Buscar templates por título 🟢 CONFIRMADO

**Contexto**: El usuario busca entre las plantillas (templates) disponibles para insertar en el grafo.

**Rastreabilidad**: `src/main/frontend/search.cljs` → `template-search`

```gherkin
Dado que el grafo tiene templates definidos:
  - "Daily Standup"
  - "Sprint Retrospective"
  - "Meeting Notes"
  - "Bug Report"
Cuando el usuario busca "meeting"
Então se retorna el template "Meeting Notes"
  Y "Daily Standup" no aparece (no coincide con "meeting")
  Y la búsqueda es case-insensitive
  Y el resultado incluye el título del template y su identificador
```

---

## Cenários de Borda

### BUS-BORDE-1: Query DSL con sintaxis inválida o mal formada 🟡 INFERIDO

**Contexto**: El usuario escribe una query DSL con paréntesis desbalanceados o operadores desconocidos.

**Comportamiento esperado**:
- El parser `parse-query` intenta parsear la query
- Si la sintaxis es inválida, se lanza una excepción o se retorna error
- La UI puede mostrar un mensaje de error "Invalid query syntax"
- La query no se ejecuta y no se retornan resultados

---

### BUS-BORDE-2: Búsqueda con la query vacía o solo espacios 🟢 CONFIRMADO

**Contexto**: El usuario ejecuta una búsqueda sin escribir ningún término.

**Comportamiento esperado**:
- `block-search` con query vacía se normaliza a string vacío
- El motor de búsqueda puede retornar vector vacío o todos los resultados (depende de implementación)
- `file-search` con query vacía matchea todos los archivos no-markdown (comportamiento de fuzzy search con string vacío)

---

### BUS-BORDE-3: Búsqueda en grafo con más de 100,000 bloques 🟡 INFERIDO

**Contexto**: Grafo masivo con años de notas acumuladas.

**Comportamiento esperado**:
- `rebuild-indices!` completo puede ser costoso en tiempo
- `transact-blocks!` incremental mitiga la necesidad de rebuilds completos
- La búsqueda se ejecuta en worker thread para no bloquear la UI
- `block-search` con fuzzy search puede degradarse con índices muy grandes
- El motor browser puede aplicar límites internos de resultados

---

### BUS-BORDE-4: Motor de búsqueda browser no disponible (worker caído) 🟡 INFERIDO

**Contexto**: El worker thread que aloja el motor browser de búsqueda falla.

**Comportamiento esperado**:
- Agency intenta delegar al Browser engine y recibe un error
- Si hay Plugin engines registrados, sus resultados aún se retornan
- Si no hay Plugin engines, la búsqueda falla completamente
- Se puede disparar un rebuild del worker

---

## Resumen de cobertura

| Flujo | Escenarios | Confianza predominante |
|-------|-----------|----------------------|
| Búsqueda de Páginas | 5 | 🟢 CONFIRMADO (4), 🟡 INFERIDO (1) |
| Búsqueda con Operadores DSL | 8 | 🟢 CONFIRMADO (8) |
| Búsqueda de Archivos | 4 | 🟢 CONFIRMADO (4) |
| Cenários de Borda | 4 | 🟢 CONFIRMADO (1), 🟡 INFERIDO (3) |

---

*Documento generado automáticamente por Reversa Writer*
