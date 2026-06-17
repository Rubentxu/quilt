# ADR-0030: Graph Space, almacenamiento canónico y lifecycle journal-first

Status: accepted

## Contexto

Quilt mezcla hoy dos modelos incompatibles:

1. un modelo heredado de `graph file` suelto (`quilt.db`)
2. un modelo emergente de **Graph Space** como unidad aislada de trabajo

Además, el producto necesita una entrada estable y útil. Restaurar rutas internas arbitrarias, autoreparar graphs inválidos o abrir siempre un selector debilita el modelo mental del usuario.

La referencia principal es **Logseq DB** en tres ideas concretas:

- unidad local-first por graph
- SQLite canónica dentro del graph
- configuración específica viviendo con el graph

Sin embargo, Quilt ya diverge en puntos que conviene preservar:

- properties tipadas como fuente semántica principal
- arquitectura MCP-first
- panel derecho contextual adaptativo
- proyecciones y semántica visual persistida

## Decision

### 1. Graph como unidad canónica

Quilt usa **Graph** como la unidad completa que el usuario crea, abre, cierra y cambia.

Un Graph está anclado a un directorio del usuario.

### 2. Persistencia canónica

Cada Graph guarda su persistencia principal en:

```text
<graph-root>/.quilt/quilt.db
```

`quilt.db` es la **fuente de verdad canónica** del Graph.

### 3. Graph en carpetas reales del usuario

Un Graph puede vivir en:

- una carpeta vacía
- una carpeta existente con contenido previo

Quilt no exige una carpeta dedicada vacía. Solo reserva su espacio interno bajo `.quilt/`.

### 4. Recursos externos compatibles

Si la carpeta del Graph contiene recursos compatibles:

- Quilt puede detectarlos
- Quilt puede importarlos o reindexarlos
- la operación es **manual y explícita**
- no hay autoingesta al abrir el Graph
- no hay watch automático en v1

Una vez ingeridos, la verdad canónica sigue siendo `quilt.db`.

### 5. Estado global vs estado del Graph

#### Estado específico del Graph

Vive dentro del propio Graph e incluye:

- páginas
- bloques
- links
- properties
- vistas
- metadata del Graph Space

#### Estado global de aplicación

Vive fuera del Graph e incluye:

- `last_opened_graph`
- lista de graphs recientes
- preferencias globales de shell/layout
- logs y caches globales

### 6. Graph inválido

Si el `last_opened_graph` es inválido porque el path no existe, fue movido o `quilt.db` es inusable, Quilt debe:

- fallar explícitamente
- no recrear `.quilt/quilt.db` en silencio
- no autoreparar sin consentimiento
- ofrecer elegir o crear otro Graph

### 7. Instancia única, un Graph activo

Quilt funciona con:

- una sola ventana / instancia activa
- un solo Graph activo a la vez

Sí se permite cambiar de Graph dentro de la misma instancia. Cambiar de Graph requiere desmontar completamente el contexto ligado al Graph anterior.

### 8. Arranque

Al arrancar:

- si `last_opened_graph` es válido, Quilt lo abre directamente
- si no, Quilt muestra el selector de Graph

El selector es una superficie de recuperación/selección, no la home principal.

### 9. Creación de Graph

Crear un Graph significa:

- elegir un directorio
- permitir a Quilt crear `.quilt/quilt.db`

No se elige manualmente un `.db` suelto.

### 10. Superficie de entrada: Journal de hoy

Abrir un Graph siempre navega al **Journal de hoy**. Quilt no restaura la última ruta interna visitada.

### 11. Creación de journals

Los journals se crean bajo demanda:

- el Journal de hoy se crea automáticamente si no existe
- los días no abiertos no existen todavía
- un día concreto puede crearse al navegar desde calendario o desplazamiento temporal

### 12. Navegación temporal

La entrada Journal debe soportar:

- día anterior
- día siguiente
- calendario para saltar a una fecha concreta

### 13. Briefings y ayudas cognitivas

Superficies como `Morning Briefing` deben ser:

- colapsables
- visibles por defecto solo cuando el Journal de hoy:
  - acaba de crearse, o
  - está vacío
- limitadas al día actual

No deben ocupar permanentemente la cabecera principal del Journal.

### 14. Panel derecho contextual

En desktop, el panel derecho:

- está visible por defecto
- es colapsable/ocultable
- recuerda su visibilidad como preferencia global

Su prioridad contextual es:

1. selección activa
2. contexto de página/journal
3. contexto general del Graph

Es la superficie principal para:

- edición rica de properties
- metadata
- acciones contextuales
- sugerencias
- contexto semántico

Puede mostrar como máximo **0 o 1 acción principal**, solo cuando la confianza contextual sea alta.

### 15. Graph v1

El primer grafo útil de Quilt es:

- local
- contextual
- 2D
- centrado en la página o bloque actual
- con profundidad 1 / 2 / 3
- sin filtros en v1
- bidireccional con la superficie principal

El grafo global masivo no es la experiencia principal de v1.

### 16. Semántica visual

Elementos como:

- `icon::`
- `emoji::`
- futura metadata visual

viven como properties persistidas del Graph.

Resolución visual:

1. valor explícito del usuario
2. default por rol semántico
3. valor derivado por contexto
4. fallback del sistema

Para `icon::`, v1 usa una **librería curada del sistema**.

### 17. Graph Space vs Graph Content

Se distingue entre:

- **Graph Space**: identidad del espacio completo
- **Graph Content**: contenido interno del graph

La metadata del Graph Space incluye:

- nombre
- icono
- descripción
- color/tema
- path
- fecha de creación

Esa metadata vive dentro del propio Graph y se edita desde la configuración del Graph, no desde el selector.

## Consequences

- se elimina progresivamente el modelo mental de `quilt.db` suelto
- server, CLI, MCP, docs y UI deben alinearse al mismo modelo de Graph
- el Journal pasa a ser la puerta de entrada real del producto
- el panel derecho deja de ser accesorio y se vuelve superficie operativa contextual
- el local graph v1 prioriza utilidad sobre espectáculo

## Rejected alternatives

### Restaurar la última ruta interna

Rechazado. El producto debe entrar por una superficie estable y útil: el Journal de hoy.

### Autoreparar graphs inválidos

Rechazado. Puede esconder corrupción o pérdida real de datos.

### Watch automático de recursos externos

Rechazado en v1. Introduce acoplamiento y complejidad de lifecycle demasiado pronto.

### Grafo global como experiencia principal

Rechazado. Suele ser más decorativo que útil.

### SVG arbitrario libre en v1

Rechazado en favor de una librería curada con semántica consistente.
