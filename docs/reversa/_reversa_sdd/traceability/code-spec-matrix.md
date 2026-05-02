# Code/Spec Matrix — Logseq

> **Proyecto**: Logseq
> **Generado por**: reversa-writer
> **Fecha**: 2026-05-02
> **Nivel**: detalhado
> **Cobertura**: 🟢 Completa | 🟡 Parcial | 🔴 Sin spec

---

## Archivos de `src/main/frontend/`

### Núcleo de la aplicación

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `src/main/frontend/core.cljs` | — | 🔴 | Entry point del frontend; no tiene spec SDD dedicada |
| `src/main/frontend/state.cljs` | — | 🔴 | Estado global de UI (atoms); no tiene spec dedicada |
| `src/main/frontend/config.cljs` | — | 🔴 | Configuración global de la app; no tiene spec |
| `src/main/frontend/commands.cljs` | — | 🔴 | Registro de comandos y atajos; no tiene spec |
| `src/main/frontend/routes.cljs` | — | 🔴 | Rutas de navegación interna; no tiene spec |
| `src/main/frontend/handler.cljs` | — | 🔴 | Dispatch principal de handlers; no tiene spec |
| `src/main/frontend/page.cljs` | — | 🔴 | Lógica de renderizado de página; no tiene spec |
| `src/main/frontend/date.cljs` | — | 🔴 | Utilidades de fechas; no tiene spec |
| `src/main/frontend/background_tasks.cljs` | — | 🔴 | Tareas background; no tiene spec |
| `src/main/frontend/undo_redo.cljs` | `sdd/outliner.md` | 🟡 | Undo/redo via transaction history; outliner menciona el mecanismo |
| `src/main/frontend/publishing.cljs` | — | 🔴 | Sistema de publicación; no tiene spec SDD dedicada |
| `src/main/frontend/persist_db.cljs` | — | 🔴 | Persistencia de DB a disco; no tiene spec |
| `src/main/frontend/template.cljs` | — | 🔴 | Sistema de templates; no tiene spec |
| `src/main/frontend/ui.cljs` | — | 🔴 | Utilidades de UI generales; no tiene spec |
| `src/main/frontend/mixins.cljs` | — | 🔴 | Mixins de Rum/React; no tiene spec |
| `src/main/frontend/db_mixins.cljs` | — | 🔴 | Mixins de acceso a DB para componentes; no tiene spec |
| `src/main/frontend/format.cljs` | — | 🔴 | Wrapper de formato (no confundir con frontend/format/); no tiene spec |
| `src/main/frontend/loader.cljs` | — | 🔴 | Lazy loader de módulos; no tiene spec |
| `src/main/frontend/flows.cljs` | — | 🔴 | Sistema de flows; no tiene spec |
| `src/main/frontend/debug.cljs` | — | 🔴 | Utilidades de debug; no tiene spec |
| `src/main/frontend/diff.cljs` | — | 🔴 | Diff de textos; no tiene spec |
| `src/main/frontend/image.cljs` | — | 🔴 | Manejo de imágenes; no tiene spec |
| `src/main/frontend/security.cljs` | — | 🔴 | Utilidades de seguridad; no tiene spec |
| `src/main/frontend/reaction.cljs` | — | 🔴 | Sistema reactivo (RxJS-like) para UI; no tiene spec |
| `src/main/frontend/storage.cljs` | — | 🔴 | Almacenamiento local (localStorage wrapper); no tiene spec |
| `src/main/frontend/rum.cljs` | — | 🔴 | Wrappers de Rum (React fork); no tiene spec |
| `src/main/frontend/quick_capture.cljs` | — | 🔴 | Quick capture dialog; no tiene spec |
| `src/main/frontend/common_keywords.cljs` | — | 🔴 | Keywords comunes del sistema; no tiene spec |
| `src/main/frontend/colors.cljs` | — | 🔴 | Paleta de colores; no tiene spec |
| `src/main/frontend/error.cljs` | — | 🔴 | Manejo de errores; no tiene spec |
| `src/main/frontend/spec.cljs` | — | 🔴 | Especificaciones internas (malli); no tiene spec |
| `src/main/frontend/log.cljs` | — | 🔴 | Sistema de logging; no tiene spec |
| `src/main/frontend/version.cljs` | — | 🔴 | Información de versión; no tiene spec |

---

### DB Layer (`src/main/frontend/db/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `src/main/frontend/db/model.cljs` | `sdd/outliner.md` | 🟡 | Funciones de acceso a datos (get-block-by-uuid, get-page, etc.); outliner las consume pero el spec no las cubre en detalle |
| `src/main/frontend/db/transact.cljs` | `sdd/outliner.md` | 🟡 | Transacciones async; outliner spec cubre transacciones pero no el wrapper async |
| `src/main/frontend/db/conn.cljs` | `sdd/outliner.md` | 🟡 | Gestión de conexiones DataScript; outliner usa conexiones |
| `src/main/frontend/db/conn_state.cljs` | — | 🔴 | Estado de conexiones; no tiene spec |
| `src/main/frontend/db/query_dsl.cljs` | — | 🔴 | Parser y ejecutor de queries DSL (and, or, not, between, task, etc.); sistema crítico sin spec dedicada |
| `src/main/frontend/db/query_custom.cljs` | — | 🔴 | Queries personalizadas avanzadas; no tiene spec |
| `src/main/frontend/db/query_react.cljs` | — | 🔴 | React queries (reactivas) para componentes; no tiene spec |
| `src/main/frontend/db/react.cljs` | — | 🔴 | Motor de queries reactivas; no tiene spec |
| `src/main/frontend/db/async.cljs` | — | 🔴 | Versiones async de funciones DB; no tiene spec |
| `src/main/frontend/db/restore.cljs` | — | 🔴 | Restauración de grafos; no tiene spec |
| `src/main/frontend/db/persist.cljs` | — | 🔴 | Persistencia de datos; no tiene spec |
| `src/main/frontend/db/utils.cljs` | — | 🔴 | Utilidades de DB; no tiene spec |
| `src/main/frontend/db/debug.cljs` | — | 🔴 | Debug de DB; no tiene spec |
| `src/main/frontend/db/rtc/debug_ui.cljs` | — | 🔴 | UI de debug para RTC sync; no tiene spec |
| `src/main/frontend/db/async/util.cljs` | — | 🔴 | Utilidades async de DB; no tiene spec |

---

### Handler Layer (`src/main/frontend/handler/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `src/main/frontend/handler/events.cljs` | — | 🔴 | Event loop principal y definición de eventos; núcleo del sistema sin spec |
| `src/main/frontend/handler/editor.cljs` | `sdd/outliner.md` | 🟡 | Handlers del editor (edit-block!, insert-new-block!, delete-block!); outliner spec cubre las operaciones core pero no los handlers UI |
| `src/main/frontend/handler/block.cljs` | `sdd/outliner.md` | 🟡 | Handlers de bloque; outliner cubre operaciones core |
| `src/main/frontend/handler/page.cljs` | — | 🔴 | Handlers de página (create, rename, delete); no tiene spec dedicada |
| `src/main/frontend/handler/journal.cljs` | — | 🔴 | Handlers de journal; no tiene spec |
| `src/main/frontend/handler/search.cljs` | `sdd/frontend-search.md` | 🟡 | Handlers de búsqueda que invocan al motor; el spec cubre el motor pero no los handlers UI |
| `src/main/frontend/handler/route.cljs` | — | 🔴 | Manejo de rutas/navegación; no tiene spec |
| `src/main/frontend/handler/repo.cljs` | — | 🔴 | Gestión de repositorios/grafos; no tiene spec |
| `src/main/frontend/handler/graph.cljs` | — | 🔴 | Handlers de grafo (switch, restore); no tiene spec |
| `src/main/frontend/handler/plugin.cljs` | — | 🔴 | Gestión de plugins; no tiene spec |
| `src/main/frontend/handler/publish.cljs` | — | 🔴 | Handlers de publicación; no tiene spec |
| `src/main/frontend/handler/property.cljs` | — | 🔴 | Handlers de propiedades; no tiene spec |
| `src/main/frontend/handler/ui.cljs` | — | 🔴 | Handlers de UI (sidebar, modales, etc.); no tiene spec |
| `src/main/frontend/handler/export.cljs` | — | 🔴 | Exportación de contenido; no tiene spec |
| `src/main/frontend/handler/paste.cljs` | — | 🔴 | Manejo de pegado (paste); no tiene spec |
| `src/main/frontend/handler/dnd.cljs` | — | 🔴 | Drag & drop handlers; no tiene spec |
| `src/main/frontend/handler/config.cljs` | — | 🔴 | Configuración de handlers; no tiene spec |
| `src/main/frontend/handler/common.cljs` | — | 🔴 | Handlers comunes compartidos; no tiene spec |
| `src/main/frontend/handler/window.cljs` | — | 🔴 | Manejo de ventanas; no tiene spec |
| `src/main/frontend/handler/user.cljs` | — | 🔴 | Handlers de usuario; no tiene spec |
| `src/main/frontend/handler/shell.cljs` | — | 🔴 | Shell/terminal handlers; no tiene spec |
| `src/main/frontend/handler/worker.cljs` | — | 🔴 | Comunicación con db-worker; no tiene spec |
| `src/main/frontend/handler/profiler.cljs` | — | 🔴 | Profiling; no tiene spec |
| `src/main/frontend/handler/code.cljs` | — | 🔴 | Bloques de código; no tiene spec |
| `src/main/frontend/handler/e2ee.cljs` | — | 🔴 | Encryptación end-to-end; no tiene spec |
| `src/main/frontend/handler/jump.cljs` | — | 🔴 | Navegación por salto; no tiene spec |
| `src/main/frontend/handler/history.cljs` | — | 🔴 | Historial de navegación; no tiene spec |
| `src/main/frontend/handler/notification.cljs` | — | 🔴 | Notificaciones; no tiene spec |
| `src/main/frontend/handler/recent.cljs` | — | 🔴 | Páginas/bloques recientes; no tiene spec |
| `src/main/frontend/handler/reaction.cljs` | — | 🔴 | Reacciones (emoji); no tiene spec |
| `src/main/frontend/handler/assets.cljs` | `sdd/frontend-fs.md` | 🟡 | Manejo de assets (archivos); fs spec cubre el protocolo pero no los handlers específicos |
| `src/main/frontend/handler/command_palette.cljs` | — | 🔴 | Paleta de comandos; no tiene spec |
| `src/main/frontend/handler/global_config.cljs` | — | 🔴 | Configuración global; no tiene spec |
| `src/main/frontend/handler/plugin_config.cljs` | — | 🔴 | Configuración de plugins; no tiene spec |
| `src/main/frontend/handler/repo_config.cljs` | — | 🔴 | Configuración de repositorio; no tiene spec |
| `src/main/frontend/handler/query/builder.cljs` | — | 🔴 | Query builder UI; no tiene spec |
| `src/main/frontend/handler/property/util.cljs` | — | 🔴 | Utilidades de propiedades; no tiene spec |

**Subdirectorios del handler:**

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `handler/common/editor.cljs` | `sdd/outliner.md` | 🟡 | Editor común entre file-based y db-based |
| `handler/common/page.cljs` | — | 🔴 | Páginas comunes; no tiene spec |
| `handler/common/developer.cljs` | — | 🔴 | Herramientas de desarrollo; no tiene spec |
| `handler/common/plugin.cljs` | — | 🔴 | Plugins comunes; no tiene spec |
| `handler/common/config_edn.cljs` | — | 🔴 | Config EDN; no tiene spec |
| `handler/db_based/editor.cljs` | `sdd/outliner.md` | 🟡 | Editor para modo DB-based |
| `handler/db_based/page.cljs` | — | 🔴 | Páginas DB-based; no tiene spec |
| `handler/db_based/property.cljs` | — | 🔴 | Propiedades DB-based; no tiene spec |
| `handler/db_based/import.cljs` | `sdd/graph-parser.md` | 🟡 | Importación que usa graph-parser |
| `handler/db_based/export.cljs` | — | 🔴 | Exportación DB-based; no tiene spec |
| `handler/db_based/recent.cljs` | — | 🔴 | Recientes DB-based; no tiene spec |
| `handler/db_based/sync.cljs` | — | 🔴 | Sync DB-based; no tiene spec |
| `handler/db_based/rtc_flows.cljs` | — | 🔴 | RTC flows; no tiene spec |
| `handler/db_based/rtc_background_tasks.cljs` | — | 🔴 | RTC background tasks; no tiene spec |
| `handler/editor/lifecycle.cljs` | `sdd/outliner.md` | 🟡 | Ciclo de vida del editor |
| `handler/events/ui.cljs` | — | 🔴 | Eventos de UI; no tiene spec |
| `handler/events/rtc.cljs` | — | 🔴 | Eventos RTC; no tiene spec |
| `handler/events/export.cljs` | — | 🔴 | Eventos de exportación; no tiene spec |
| `handler/export/common.cljs` | — | 🔴 | Exportación común; no tiene spec |
| `handler/export/html.cljs` | — | 🔴 | Exportación HTML; no tiene spec |
| `handler/export/opml.cljs` | — | 🔴 | Exportación OPML; no tiene spec |
| `handler/export/text.cljs` | — | 🔴 | Exportación texto; no tiene spec |
| `handler/export/zip_helper.cljs` | — | 🔴 | Ayudante ZIP para export; no tiene spec |

---

### Components Layer (`src/main/frontend/components/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `components/editor.cljs` | `sdd/outliner.md` | 🟡 | Editor de bloques; outliner spec cubre operaciones core pero no la UI del editor |
| `components/block.cljs` | `sdd/outliner.md` | 🟡 | Componente de bloque individual; outliner cubre estructura |
| `components/block/macros.cljs` | — | 🔴 | Macros de bloque; no tiene spec |
| `components/page.cljs` | — | 🔴 | Componente de página; no tiene spec |
| `components/journal.cljs` | — | 🔴 | Componente de journal; no tiene spec |
| `components/query.cljs` | — | 🔴 | Componente de queries; no tiene spec |
| `components/query/view.cljs` | — | 🔴 | Vista de resultados de query; no tiene spec |
| `components/query/result.cljs` | — | 🔴 | Resultado individual de query; no tiene spec |
| `components/query/builder.cljs` | — | 🔴 | Query builder visual; no tiene spec |
| `components/left_sidebar.cljs` | — | 🔴 | Barra lateral izquierda; no tiene spec |
| `components/right_sidebar.cljs` | — | 🔴 | Barra lateral derecha; no tiene spec |
| `components/header.cljs` | — | 🔴 | Encabezado; no tiene spec |
| `components/container.cljs` | — | 🔴 | Contenedor principal de la app; no tiene spec |
| `components/reference.cljs` | — | 🔴 | Renderizado de referencias; no tiene spec |
| `components/reference_filters.cljs` | — | 🔴 | Filtros de referencias; no tiene spec |
| `components/property.cljs` | — | 🔴 | Componente de propiedades; no tiene spec |
| `components/property/value.cljs` | — | 🔴 | Valor de propiedad; no tiene spec |
| `components/property/dialog.cljs` | — | 🔴 | Diálogo de propiedad; no tiene spec |
| `components/property/config.cljs` | — | 🔴 | Configuración de propiedad; no tiene spec |
| `components/property/default_value.cljs` | — | 🔴 | Valor por defecto de propiedad; no tiene spec |
| `components/settings.cljs` | — | 🔴 | Panel de settings; no tiene spec |
| `components/selection.cljs` | — | 🔴 | Selección de bloques; no tiene spec |
| `components/file.cljs` | `sdd/frontend-fs.md` | 🟡 | Componente de archivo; fs spec cubre el protocolo |
| `components/filepicker.cljs` | `sdd/frontend-fs.md` | 🟡 | Selector de archivos; fs spec cubre el protocolo |
| `components/search.cljs` | `sdd/frontend-search.md` | 🟡 | Componente de búsqueda (no confundir con search.cljs del módulo search) |
| `components/home.cljs` | — | 🔴 | Pantalla de inicio; no tiene spec |
| `components/repo.cljs` | — | 🔴 | Gestión de repositorios UI; no tiene spec |
| `components/recycle.cljs` | — | 🔴 | Papelera de reciclaje; no tiene spec |
| `components/shortcut.cljs` | — | 🔴 | Gestión de atajos; no tiene spec |
| `components/shortcut_help.cljs` | — | 🔴 | Ayuda de atajos; no tiene spec |
| `components/plugins.cljs` | — | 🔴 | Panel de plugins; no tiene spec |
| `components/plugins_settings.cljs` | — | 🔴 | Configuración de plugins; no tiene spec |
| `components/plugin_logs.cljs` | — | 🔴 | Logs de plugins; no tiene spec |
| `components/theme.cljs` | — | 🔴 | Gestión de temas; no tiene spec |
| `components/dnd.cljs` | — | 🔴 | Drag & drop UI; no tiene spec |
| `components/content.cljs` | — | 🔴 | Contenido de página; no tiene spec |
| `components/objects.cljs` | — | 🔴 | Objetos embebidos; no tiene spec |
| `components/macro.cljs` | — | 🔴 | Macros UI; no tiene spec |
| `components/class.cljs` | — | 🔴 | Clases UI; no tiene spec |
| `components/datepicker.cljs` | — | 🔴 | Selector de fecha; no tiene spec |
| `components/all_pages.cljs` | — | 🔴 | Lista de todas las páginas; no tiene spec |
| `components/find_in_page.cljs` | — | 🔴 | Buscar en página; no tiene spec |
| `components/onboarding.cljs` | — | 🔴 | Onboarding inicial; no tiene spec |
| `components/onboarding/setups.cljs` | — | 🔴 | Setup de onboarding; no tiene spec |
| `components/library.cljs` | — | 🔴 | Librería de bloques; no tiene spec |
| `components/lazy_editor.cljs` | — | 🔴 | Editor lazy-loaded; no tiene spec |
| `components/shell.cljs` | — | 🔴 | Shell/terminal UI; no tiene spec |
| `components/server.cljs` | — | 🔴 | Servidor local UI; no tiene spec |
| `components/e2ee.cljs` | — | 🔴 | UI de encryptación; no tiene spec |
| `components/export.cljs` | — | 🔴 | UI de exportación; no tiene spec |
| `components/assets.cljs` | `sdd/frontend-fs.md` | 🟡 | Assets UI; fs spec cubre protocolo |
| `components/select.cljs` | — | 🔴 | Componente select genérico; no tiene spec |
| `components/svg.cljs` | — | 🔴 | Utilidades SVG; no tiene spec |
| `components/icon.cljs` | — | 🔴 | Iconos; no tiene spec |
| `components/imports.cljs` | — | 🔴 | Importaciones UI; no tiene spec |
| `components/page_menu.cljs` | — | 🔴 | Menú contextual de página; no tiene spec |
| `components/scheduled_deadlines.cljs` | — | 🔴 | Deadlines programados UI; no tiene spec |
| `components/window_controls.cljs` | — | 🔴 | Controles de ventana; no tiene spec |
| `components/views.cljs` | — | 🔴 | Sistema de vistas; no tiene spec |
| `components/bug_report.cljs` | — | 🔴 | Reporte de bugs; no tiene spec |
| `components/quick_add.cljs` | — | 🔴 | Quick add dialog; no tiene spec |
| `components/handbooks.cljs` | — | 🔴 | Handbooks UI; no tiene spec |
| `components/profiler.cljs` | — | 🔴 | UI de profiling; no tiene spec |
| `components/user/login.cljs` | — | 🔴 | Login de usuario; no tiene spec |
| `components/rtc/indicator.cljs` | — | 🔴 | Indicador RTC; no tiene spec |
| `components/db_based/page.cljs` | — | 🔴 | Página modo DB-based; no tiene spec |
| `components/cmdk/core.cljs` | — | 🔴 | CMD+K command palette core; no tiene spec |
| `components/cmdk/state.cljs` | — | 🔴 | Estado de CMD+K; no tiene spec |
| `components/cmdk/scroll.cljs` | — | 🔴 | Scroll de CMD+K; no tiene spec |
| `components/cmdk/list_item.cljs` | — | 🔴 | Item de lista CMD+K; no tiene spec |

---

### Search Module (`src/main/frontend/search/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `src/main/frontend/search.cljs` | `sdd/frontend-search.md` | 🟢 | Funciones públicas: block-search, file-search, template-search, rebuild-indices!, fuzzy-search |
| `src/main/frontend/search/protocol.cljs` | `sdd/frontend-search.md` | 🟢 | Protocolo Engine (6 métodos) |
| `src/main/frontend/search/agency.cljs` | `sdd/frontend-search.md` | 🟢 | Agency coordinator: query, rebuild-indices, transact-blocks! |
| `src/main/frontend/search/browser.cljs` | `sdd/frontend-search.md` | 🟢 | Motor browser nativo (thread-api/search-blocks) |
| `src/main/frontend/search/plugin.cljs` | `sdd/frontend-search.md` | 🟡 | Motor vía Plugin API; spec lo menciona pero sin detalles de implementación |

---

### File System Module (`src/main/frontend/fs/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `src/main/frontend/fs.cljs` | `sdd/frontend-fs.md` | 🟢 | Wrapper y factory de FS |
| `src/main/frontend/fs/protocol.cljs` | `sdd/frontend-fs.md` | 🟢 | Protocolo Fs (14 métodos) |
| `src/main/frontend/fs/node.cljs` | `sdd/frontend-fs.md` | 🟢 | Implementación Node.js / Electron |
| `src/main/frontend/fs/memory_fs.cljs` | `sdd/frontend-fs.md` | 🟢 | Implementación en memoria (testing) |

---

### Format Module (`src/main/frontend/format/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `src/main/frontend/format/protocol.cljs` | `sdd/frontend-format.md` | 🟢 | Protocolo Format (toEdn, toHtml, exportMarkdown, exportOPML) |
| `src/main/frontend/format/mldoc.cljs` | `sdd/frontend-format.md` | 🟢 | Wrapper de Mldoc para parsing Markdown/Org |
| `src/main/frontend/format/block.cljs` | `sdd/frontend-format.md` | 🟢 | Funciones de parsing de bloques desde AST |

---

### Modules (`src/main/frontend/modules/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `modules/outliner/tree.cljs` | `sdd/outliner.md` | 🟢 | Construcción de árboles desde bloques; outliner spec cubre estas funciones |
| `modules/outliner/pipeline.cljs` | `sdd/outliner.md` | 🟡 | Pipeline de procesamiento; outliner spec lo menciona |
| `modules/outliner/op.cljs` | `sdd/outliner.md` | 🟢 | Dispatcher de operaciones del outliner |
| `modules/shortcut/core.cljs` | — | 🔴 | Sistema de atajos de teclado; no tiene spec |
| `modules/shortcut/config.cljs` | — | 🔴 | Configuración de atajos; no tiene spec |
| `modules/shortcut/before.cljs` | — | 🔴 | Pre-procesamiento de atajos; no tiene spec |
| `modules/shortcut/utils.cljs` | — | 🔴 | Utilidades de atajos; no tiene spec |
| `modules/shortcut/data_helper.cljs` | — | 🔴 | Ayudante de datos para atajos; no tiene spec |
| `modules/layout/core.cljs` | — | 🔴 | Sistema de layout; no tiene spec |
| `modules/instrumentation/core.cljs` | — | 🔴 | Instrumentación; no tiene spec |
| `modules/instrumentation/sentry.cljs` | — | 🔴 | Integración Sentry; no tiene spec |
| `modules/instrumentation/posthog.cljs` | — | 🔴 | Integración PostHog; no tiene spec |

---

### Extensions (`src/main/frontend/extensions/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `extensions/code.cljs` | — | 🔴 | Bloques de código; no tiene spec |
| `extensions/latex.cljs` | — | 🔴 | Renderizado LaTeX; no tiene spec |
| `extensions/graph.cljs` | — | 🔴 | Visualización de grafo; no tiene spec |
| `extensions/graph/pixi.cljs` | — | 🔴 | Renderizado Pixi.js para grafo; no tiene spec |
| `extensions/pdf/core.cljs` | — | 🔴 | Anotaciones PDF; no tiene spec |
| `extensions/pdf/utils.cljs` | — | 🔴 | Utilidades PDF; no tiene spec |
| `extensions/pdf/toolbar.cljs` | — | 🔴 | Toolbar de PDF; no tiene spec |
| `extensions/pdf/assets.cljs` | — | 🔴 | Assets PDF; no tiene spec |
| `extensions/pdf/windows.cljs` | — | 🔴 | Ventanas PDF; no tiene spec |
| `extensions/zotero.cljs` | — | 🔴 | Integración Zotero; no tiene spec |
| `extensions/zip.cljs` | — | 🔴 | Manejo de ZIP; no tiene spec |
| `extensions/highlight.cljs` | — | 🔴 | Highlight de sintaxis; no tiene spec |
| `extensions/html_parser.cljs` | — | 🔴 | Parser HTML; no tiene spec |
| `extensions/lightbox.cljs` | — | 🔴 | Lightbox para imágenes; no tiene spec |
| `extensions/sci.cljs` | — | 🔴 | SCI (Small Clojure Interpreter); no tiene spec |
| `extensions/fsrs.cljs` | — | 🔴 | FSRS (Spaced Repetition); no tiene spec |
| `extensions/srs/handler.cljs` | — | 🔴 | SRS handler; no tiene spec |
| `extensions/video/youtube.cljs` | — | 🔴 | Embed de YouTube; no tiene spec |
| `extensions/handbooks/core.cljs` | — | 🔴 | Handbooks; no tiene spec |
| `extensions/calc.cljc` | — | 🔴 | Calculadora inline; no tiene spec |

---

### Worker (`src/main/frontend/worker/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `worker/search.cljs` | `sdd/frontend-search.md` | 🟡 | Búsqueda en worker; search spec cubre el motor browser que lo invoca |
| `worker/pipeline.cljs` | `sdd/outliner.md` | 🟡 | Pipeline de procesamiento en worker |
| `worker/sync.cljs` | — | 🔴 | Sincronización; no tiene spec |
| `worker/sync/apply_txs.cljs` | — | 🔴 | Aplicación de transacciones sync; no tiene spec |
| `worker/sync/upload.cljs` | — | 🔴 | Upload sync; no tiene spec |
| `worker/sync/download.cljs` | — | 🔴 | Download sync; no tiene spec |
| `worker/sync/crypt.cljs` | — | 🔴 | Criptografía sync (E2EE); no tiene spec |
| `worker/sync/auth.cljs` | — | 🔴 | Autenticación sync; no tiene spec |
| `worker/sync/transport.cljs` | — | 🔴 | Transporte sync; no tiene spec |
| `worker/sync/handle_message.cljs` | — | 🔴 | Manejo de mensajes sync; no tiene spec |
| `worker/sync/client_op.cljs` | — | 🔴 | Operaciones cliente sync; no tiene spec |
| `worker/sync/presence.cljs` | — | 🔴 | Presencia sync; no tiene spec |
| `worker/sync/assets.cljs` | — | 🔴 | Assets sync; no tiene spec |
| `worker/sync/asset_db_listener.cljs` | — | 🔴 | Listener de asset DB; no tiene spec |
| `worker/sync/large_title.cljs` | — | 🔴 | Títulos largos sync; no tiene spec |
| `worker/sync/log_and_state.cljs` | — | 🔴 | Log y estado sync; no tiene spec |
| `worker/sync/temp_sqlite.cljs` | — | 🔴 | SQLite temporal sync; no tiene spec |
| `worker/sync/const.cljs` | — | 🔴 | Constantes sync; no tiene spec |
| `worker/sync/util.cljs` | — | 🔴 | Utilidades sync; no tiene spec |
| `worker/db_worker.cljs` | — | 🔴 | DB worker principal; no tiene spec |
| `worker/db_worker_node.cljs` | — | 🔴 | DB worker para Node.js; no tiene spec |
| `worker/db_core.cljs` | — | 🔴 | Core de DB worker; no tiene spec |
| `worker/db_listener.cljs` | — | 🔴 | Listener de cambios en DB; no tiene spec |
| `worker/undo_redo.cljs` | `sdd/outliner.md` | 🟡 | Undo/redo en worker; outliner menciona transaction history |
| `worker/publish.cljs` | — | 🔴 | Publicación en worker; no tiene spec |
| `worker/export.cljs` | — | 🔴 | Exportación en worker; no tiene spec |
| `worker/react.cljs` | — | 🔴 | React queries en worker; no tiene spec |
| `worker/state.cljs` | — | 🔴 | Estado del worker; no tiene spec |
| `worker/commands.cljs` | — | 🔴 | Comandos del worker; no tiene spec |
| `worker/debug.cljs` | — | 🔴 | Debug del worker; no tiene spec |
| `worker/shared_service.cljs` | — | 🔴 | Servicios compartidos worker; no tiene spec |
| `worker/thread_atom.cljs` | — | 🔴 | Thread-safe atoms; no tiene spec |
| `worker/graph_dir.cljs` | — | 🔴 | Directorio de grafos en worker; no tiene spec |
| `worker/version.cljs` | — | 🔴 | Versión del worker; no tiene spec |
| `worker/ui_request.cljs` | — | 🔴 | UI requests desde worker; no tiene spec |
| `worker/platform.cljs` | — | 🔴 | Plataforma worker; no tiene spec |
| `worker/platform/node.cljs` | — | 🔴 | Platform Node.js; no tiene spec |
| `worker/platform/browser.cljs` | — | 🔴 | Platform browser; no tiene spec |
| `worker/handler/page.cljs` | — | 🔴 | Page handler en worker; no tiene spec |
| `worker/db/validate.cljs` | `sdd/outliner.md` | 🟡 | Validación de DB; outliner cubre validaciones |
| `worker/db/migrate.cljs` | — | 🔴 | Migraciones de DB; no tiene spec |
| `worker/db/fix.cljs` | — | 🔴 | Correcciones de DB; no tiene spec |
| `worker/db_worker_node_lock.cljs` | — | 🔴 | Lock de DB worker Node; no tiene spec |

---

### Utilidades (`src/main/frontend/util/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `util/text.cljs` | — | 🔴 | Utilidades de texto; no tiene spec |
| `util/page.cljs` | — | 🔴 | Utilidades de página; no tiene spec |
| `util/ref.cljs` | — | 🔴 | Utilidades de referencias; no tiene spec |
| `util/cursor.cljs` | — | 🔴 | Manejo de cursor; no tiene spec |
| `util/keycode.cljs` | — | 🔴 | Códigos de teclas; no tiene spec |
| `util/clock.cljs` | — | 🔴 | Reloj/timers; no tiene spec |
| `util/url.cljs` | — | 🔴 | Utilidades URL; no tiene spec |
| `util/thingatpt.cljs` | — | 🔴 | "Thing at point" (texto bajo cursor); no tiene spec |
| `util/datalog.cljc` | — | 🔴 | Utilidades Datalog; no tiene spec |

---

### Contexto (`src/main/frontend/context/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `context/i18n.cljs` | — | 🔴 | Internacionalización; no tiene spec |

---

### Esquemas (`src/main/frontend/schema/`)

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `schema/handler/repo_config.cljc` | — | 🔴 | Schema de config de repo; no tiene spec |
| `schema/handler/plugin_config.cljc` | — | 🔴 | Schema de config de plugin; no tiene spec |
| `schema/handler/global_config.cljc` | — | 🔴 | Schema de config global; no tiene spec |
| `schema/handler/common_config.cljc` | — | 🔴 | Schema de config común; no tiene spec |

---

### Otros archivos en `src/main/frontend/`

| Archivo | Spec correspondiente | Cobertura | Notas |
|---------|---------------------|-----------|-------|
| `spec/storage.cljc` | — | 🔴 | Especificación de storage; no tiene spec |
| `namespaces.cljc` | — | 🔴 | Definición de namespaces; no tiene spec |
| `dicts.cljc` | — | 🔴 | Diccionarios de traducción; no tiene spec |
| `util.cljc` | — | 🔴 | Utilidades compartidas; no tiene spec |
| `common/async_util.cljc` | — | 🔴 | Utilidades async comunes; no tiene spec |
| `common/thread_api.cljc` | — | 🔴 | API de threads común; no tiene spec |
| `worker_common/util.cljc` | — | 🔴 | Utilidades comunes de worker; no tiene spec |
| `modules/outliner/ui.cljc` | — | 🔴 | UI del outliner compartida; no tiene spec |

---

## Resumen de cobertura

### Estadísticas globales

| Métrica | Valor |
|---------|-------|
| **Total archivos analizados** | ~270 |
| **Archivos con spec 🟢 Completa** | 17 |
| **Archivos con spec 🟡 Parcial** | 25 |
| **Archivos sin spec 🔴** | ~228 |
| **Porcentaje de cobertura completa** | 6.3% |
| **Porcentaje de cobertura parcial o total** | 15.6% |

### Por módulo

| Módulo / Directorio | Total | 🟢 | 🟡 | 🔴 | % Cubierto |
|---------------------|-------|-----|-----|-----|-----------|
| `frontend/search/` | 5 | 4 | 1 | 0 | 100% |
| `frontend/fs/` | 4 | 4 | 0 | 0 | 100% |
| `frontend/format/` | 3 | 3 | 0 | 0 | 100% |
| `modules/outliner/` | 3 | 2 | 1 | 0 | 100% |
| `frontend/db/` | 15 | 0 | 3 | 12 | 20% |
| `frontend/handler/` | 60+ | 0 | 9 | 51+ | 15% |
| `frontend/components/` | 69 | 0 | 6 | 63 | 9% |
| `frontend/worker/` | 42 | 0 | 4 | 38 | 10% |
| `frontend/extensions/` | 20 | 0 | 0 | 20 | 0% |
| `frontend/modules/` | 12 | 2 | 1 | 9 | 25% |
| Raíz `frontend/` | 37 | 0 | 1 | 36 | 3% |
| `frontend/util/` | 9 | 0 | 0 | 9 | 0% |
| `frontend/schema/` | 4 | 0 | 0 | 4 | 0% |
| Otros (context, spec, common) | 7 | 0 | 0 | 7 | 0% |

### Specs existentes y su cobertura

| Spec SDD | Archivos cubiertos (🟢+🟡) | Notas |
|----------|--------------------------|-------|
| `sdd/frontend-search.md` | 5 | Cobertura completa del módulo search; handlers y componentes de search son 🟡 |
| `sdd/frontend-fs.md` | 4 | Cobertura completa del módulo fs; componentes de archivo son 🟡 |
| `sdd/frontend-format.md` | 3 | Cobertura completa del módulo format |
| `sdd/outliner.md` | ~15 | Cubre deps/outliner/ + algunos archivos en modules/ y handler/editor |
| `sdd/graph-parser.md` | ~2 | Cubre deps/graph-parser/; tiene alcance limitado a frontend/ |
| `sdd/electron.md` | 0 | Cubre src/electron/ (fuera de frontend/) |

### Módulos críticos SIN spec dedicada 🔴

| Módulo | Impacto | Prioridad sugerida |
|--------|---------|-------------------|
| `frontend/db/` (query_dsl, query_react, model) | 🔴 HIGH — 15 features dependen de DB | **Crítica** — Especificar Query DSL y modelo de datos |
| `frontend/handler/events.cljs` | 🔴 HIGH — Event loop principal | **Crítica** — Especificar sistema de eventos |
| `frontend/components/` (editor, page, block) | 🔴 HIGH — UI principal | **Alta** — Especificar componentes core |
| `frontend/handler/editor.cljs` | 🟡 Parcial via outliner | **Media** — Completar spec de editor handlers |
| `frontend/worker/sync/` | 🔴 HIGH — Sync system | **Alta** — Especificar sistema de sincronización |
| `frontend/handler/plugin.cljs` | 🟡 Medio — Plugin system | **Media** — Especificar API de plugins |
| `frontend/publishing.cljs` | 🔴 Medio — Publishing | **Media** — Especificar sistema de publicación |

---

*Documento generado automáticamente por Reversa Writer*
