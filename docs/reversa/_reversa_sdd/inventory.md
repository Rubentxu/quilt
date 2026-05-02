# Inventario — logseq

> Generado por el Scout en 2026-05-02

## Estructura de Directorios

```
logseq/
├── .agents/                 # Agentes y skills
├── .clj-kondo/             # Configuración clj-kondo
├── .cljfmt.edn             # Formato Clojure
├── .github/workflows/       # GitHub Actions CI/CD
├── android/                # App Android
├── assets/                 # Assets estáticos
├── bb.edn                  # Babashka tasks
├── capacitor.config.ts      # Configuración Capacitor
├── cli-e2e/                # Tests E2E de CLI
├── clj-e2e/                # Tests E2E de CLJ
├── CODEBASE_OVERVIEW.md     # Documentación del codebase
├── deps.edn                # Dependencias Clojure
├── deps/                   # Librerías Clojure locales
│   ├── cli/
│   ├── common/
│   ├── db/
│   ├── db-sync/
│   ├── graph-parser/        # Parser de grafos
│   ├── outliner/            # Sistema outliner
│   ├── publish/
│   ├── publishing/
│   └── shui/
├── ios/                    # App iOS
├── package.json            # Dependencias JavaScript/Node
├── packages/               # Paquetes JS
│   └── ui/                # Sistema de componentes UI (shadcn)
├── resources/              # Recursos estáticos
├── scripts/                # Scripts de desarrollo
├── src/
│   ├── bench/              # Benchmarks
│   ├── dev-cljs/           # Utilidades de desarrollo
│   ├── electron/           # App Electron (desktop)
│   ├── main/
│   │   ├── frontend/       # Editor principal
│   │   │   ├── animations.css
│   │   │   ├── background_tasks.cljs
│   │   │   ├── commands.cljs
│   │   │   ├── components/ # Componentes UI
│   │   │   ├── config.cljs
│   │   │   ├── context/
│   │   │   ├── db/        # Acceso a DataScript
│   │   │   ├── extensions/
│   │   │   ├── format/    # Parsers de formato
│   │   │   ├── fs/        # Sistema de archivos
│   │   │   ├── handler/   # Manejadores de eventos
│   │   │   ├── modules/
│   │   │   ├── page.cljs
│   │   │   ├── publishing/
│   │   │   ├── search/
│   │   │   ├── state.cljs
│   │   │   ├── ui.css
│   │   │   ├── util/
│   │   │   └── worker/
│   │   ├── logseq/        # API para plugins
│   │   └── mobile/        # App móvil
│   └── test/              # Tests
└── shadow-cljs.edn         # Configuración Shadow CLJS
```

## Análisis por Módulo

### frontend (Editor principal)
- **Lenguaje:** ClojureScript
- **Framework:** React + Rum
- **Estado:** DataScript (documentos), Clojure atoms (UI)
- **Módulos principales:**
  - `components/` — ~50+ componentes React
  - `db/` — Modelo de datos DataScript
  - `handler/` — Event handlers
  - `fs/` — Sistema de archivos (local/cloud)
  - `format/` — Parsers (Markdown, Org-mode, etc.)
  - `search/` — Búsqueda full-text

### graph-parser (deps)
- **Función:** Parsea un grafo de Logseq y lo guarda en la base de datos
- **Lenguaje:** ClojureScript

### outliner (deps)
- **Función:** Sistema de outliner (estructura jerárquica de bloques)
- **Lenguaje:** ClojureScript

### Electron (desktop app)
- **Función:** App de escritorio
- **Lenguaje:** JavaScript + ClojureScript

### Mobile (iOS/Android)
- **Función:** Apps móviles
- **Lenguaje:** TypeScript + ClojureScript

## Tecnologías Detectadas

| Categoría | Tecnología | Versión |
|-----------|------------|---------|
| Lenguaje principal | ClojureScript | 1.12.4 |
| Build tool | Shadow CLJS | 3.4.4 |
| UI Framework | React + Rum | fork custom |
| Database | DataScript | fork custom |
| Bundler | Vite | 8.0.0 |
| Package Manager | pnpm | 10.33.0 |
| Desktop | Electron | - |
| Mobile | Capacitor | 8.2.0 |
| Schema validation | Malli | - |
| Task runner | Babashka | - |

## Puntos de Entrada

| Archivo | Tipo |
|---------|------|
| `static/electron.js` | Entry point Electron |
| `src/main/frontend/core.cljs` | Core del frontend |
| `src/main/frontend/state.cljs` | Estado global |
| `src/main/frontend/handler/events.cljs` | Event loop |

## CI/CD

- `.github/workflows/` — GitHub Actions

## Base de Datos

- **DataScript** — Base de datos in-memory (Datomic-like)
- Modelos en `src/main/frontend/db/`
- Queries en `src/main/frontend/db/query_dsl.cljs`, `query_react.cljs`

## Cobertura de Tests

- Tests en `src/test/`
- Framework: `clojure.test` + Shadow CLJS
- E2E: `cli-e2e/`, `clj-e2e/`, Playwright

## Módulos Identificados

| Módulo | Ruta | Propósito |
|--------|------|-----------|
| frontend | src/main/frontend/ | Editor principal |
| graph-parser | deps/graph-parser/ | Parser de grafos |
| outliner | deps/outliner/ | Sistema outliner |
| electron | src/electron/ | App desktop |
| db | src/main/frontend/db/ | Acceso a datos |
| components | src/main/frontend/components/ | UI components |
| handler | src/main/frontend/handler/ | Event handlers |
| fs | src/main/frontend/fs/ | File system |
| format | src/main/frontend/format/ | Parsers |
| search | src/main/frontend/search/ | Búsqueda |
| publishing | src/main/frontend/publishing/ | Publicación |
| mobile | src/main/mobile/ | App móvil |
| plugins-api | src/main/logseq/ | API de plugins |
