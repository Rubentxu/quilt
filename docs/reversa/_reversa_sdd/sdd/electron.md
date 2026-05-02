# electron

## Visão Geral
Módulo principal de Electron (`src/electron/electron`) que gestiona la aplicación desktop de Logseq. Orquesta el ciclo de vida de la app, crea y administra ventanas BrowserWindow, registra protocolos personalizados (`lsp://`, `assets://`), configura el menú nativo, maneja IPC entre el proceso main y el renderer, gestiona deep links, e instala el CLI launcher para acceso desde terminal.

## Responsabilidades
- Inicializar la aplicación Electron (`main`) con single-instance lock
- Crear y gestionar ventanas principales (`create-main-window!`, `setup-window-listeners!`)
- Registrar esquemas privilegiados y protocolos personalizados (`lsp://`, `assets://`)
- Configurar el menú de aplicación nativo según plataforma (macOS, Windows, Linux)
- Manejar deep links vía `open-url` (macOS) y argumentos de línea de comandos (Windows)
- Configurar IPC handlers bidireccionales entre main y renderer
- Gestionar auto-updater para actualizaciones automáticas
- Instalar CLI launcher (`logseq`) en directorios del sistema PATH
- Inicializar servidor HTTP interno para servir assets y plugins
- Manejar cierre de ventana con soporte para múltiples ventanas y hide-to-tray (macOS)

## Interface

### Entry point
```clojure
(main)
;; Retorna: nil (inicia el event loop de Electron)
```

### Funciones principales

```clojure
;; Callback cuando la app está lista
(on-app-ready! app)
;; app: Electron App

;; Registrar interceptores de protocolo
(setup-interceptor! app)
;; Retorna: cleanup-fn (función para desregistrar)

;; Crear ventana principal
(create-main-window!)
;; Retorna: BrowserWindow

;; Configurar listeners de ventana
(setup-window-listeners! win)
;; Retorna: cleanup-fn

;; Instalar CLI launcher en el sistema
(install-cli-launcher!)
;; Retorna: nil

;; Configurar handlers IPC
(set-ipc-handler! win)
;; win: BrowserWindow

;; Configurar menú de aplicación
(set-app-menu!)
;; Retorna: nil

;; Configurar deep links
(setup-deeplink!)
;; Retorna: nil
```

### Tipos de datos

**WindowState:**
```clojure
{:win       BrowserWindow  ;; instancia de ventana Electron
 :quitting? Boolean}       ;; flag de cierre de aplicación
```

**Protocol URL:**
```clojure
;; lsp://plugin-id/path/to/file
;; assets:///absolute/path/to/asset
;; logseq://graph-name?page=Page%20Name
```

### IPC Channels (main ↔ renderer)

| Canal | Dirección | Propósito |
|-------|-----------|-----------|
| `toggle-max-or-min-active-win` | renderer → main | Maximizar/restaurar ventana activa |
| `call-application` | renderer → main | Invocar método en el objeto `app` de Electron |
| `call-main-win` | renderer → main | Invocar método en la ventana principal |
| `export-publish-assets` | renderer → main | Exportar assets para publicación |
| `set-quit-dirty-state` | renderer → main | Marcar estado "dirty" para evitar cierre accidental |

## Regras de Negócio
- Single instance lock: solo una instancia de la aplicación puede ejecutarse a la vez; segundos intentos activan la ventana existente 🟢
- Protocolo `lsp://`: resuelve rutas a `PLUGINS_ROOT` si la URL contiene `PLUGIN_URL`, de lo contrario a `STATIC_URL` 🟢
- Protocolo `assets://`: decodifica la ruta, verifica si es absoluta o UNC (Windows), y sirve el archivo directamente 🟢
- Cierre de ventana en macOS: si hay múltiples ventanas, se cierra normalmente; si es la última, se oculta al tray (no se cierra la app) 🟢
- Cierre de ventana en Windows/Linux: la ventana se cierra normalmente; si es la última, la app termina 🟡
- Deep links con esquema `logseq://`: si incluyen parámetro `new-window`, abren nueva ventana; si no, cambian de grafo en la ventana actual 🟢
- Menú de aplicación: se adapta por plataforma — macOS incluye menú "App" (About, Services, Hide, Quit), Windows/Linux incluye File/Edit/View 🟢
- CLI launcher: se instala en `~/.local/bin` (Unix) o `%LOCALAPPDATA%\Programs\logseq` (Windows); requiere permisos de escritura 🟡
- Actualizaciones automáticas: configuradas vía `setup-updater!` con electron-updater; se verifican al iniciar la app 🟡

## Fluxo Principal

### Inicialización de la aplicación
1. `main` es invocado (entry point del proceso main de Electron)
2. `app/requestSingleInstanceLock` — si ya hay instancia, se envía señal `second-instance` y la app se cierra (`app/quit`)
3. `app/registerSchemesAsPrivileged` — registra esquemas `lsp` y `assets` con privilegios de `standard`, `secure`, `supportFetchAPI`, `corsEnabled`
4. `app/setAsDefaultProtocolClient("logseq")` — registra esquema `logseq://`
5. `set-app-menu!` — construye y aplica el menú según plataforma
6. `setup-deeplink!` — registra handlers para `open-url` (macOS) y `second-instance` con argumentos (Windows)
7. `app/on("ready", on-app-ready!)` — cuando la app está lista:
   a. `setup-interceptor!` — registra handlers de protocolo `lsp://` y `assets://`
   b. `create-main-window!` — crea BrowserWindow con dimensiones y configuración
   c. `setup-window-listeners!` — registra eventos de ventana (close, maximize, focus)
   d. `setup-updater!` — inicia el sistema de auto-update
   e. `setup-app-manager!` — configura el gestor de aplicaciones
   f. `set-ipc-handler!` — registra handlers IPC en la ventana
   g. `server/setup!` — inicia servidor HTTP interno
8. La app queda corriendo en el event loop de Electron

### Ciclo de vida de ventana
1. `create-main-window!` crea `BrowserWindow` con opciones:
   - `webPreferences.nodeIntegration` = false
   - `webPreferences.contextIsolation` = true
   - `webPreferences.preload` = script de preload
2. Carga `loadURL` apuntando al entry HTML de Logseq
3. `setup-window-listeners!` registra:
   - `"close"` → `close-handler`: decide si ocultar al tray (macOS) o cerrar
   - `"maximize"` → toggle entre maximizado y tamaño normal
   - `"focus"` → traer ventana al frente
4. Al cerrar la última ventana en macOS, la app se oculta al tray; en otros SO, se cierra

### Protocol handlers
1. `setup-interceptor!` registra dos handlers:
   - `lsp://`: parsea URL, resuelve a `PLUGINS_ROOT` o `STATIC_URL`, sirve archivo con MIME type correcto
   - `assets://`: decodifica path, maneja rutas absolutas y UNC (Windows), sirve archivo
2. Cada handler retorna un callback con el contenido del archivo o error 404

### Deep link handling
1. `setup-deeplink!` registra:
   - `app/on("open-url", handler)` para macOS
   - `app/on("second-instance", handler)` para Windows/Linux (con `process.argv`)
2. El handler parsea la URL con `js/URL.parse`
3. Verifica que el esquema sea `"logseq:"`
4. Extrae parámetros: `graph-name`, `page`, `new-window`
5. Si `new-window` está presente → `open-new-window-or-tab!`
6. Si no → envía evento `:graph/switch` al renderer vía IPC

## Fluxos Alternativos
- **[Segunda instancia detectada]:** Si `requestSingleInstanceLock` falla, se emite `second-instance` con los argumentos del nuevo intento; la instancia original procesa el deep link; la segunda instancia llama a `app/quit` 🟢
- **[Ventana cerrada en macOS]:** `close-handler` verifica `quitting?` — si true, cierra; si false y es la última ventana, `hide()` en lugar de `close()` 🟢
- **[Protocolo lsp:// con path no encontrado]:** Si el archivo no existe en `PLUGINS_ROOT` ni `STATIC_URL`, se retorna callback con error 404 🟡
- **[Protocolo assets:// con path UNC (Windows)]:** Si el path comienza con `\\`, se trata como UNC y se sirve directamente; si es letra de unidad (`C:\`), se sirve como path absoluto 🟡
- **[CLI launcher: directorio no escribible]:** Si `preferred-unix-cli-dir` o `preferred-win-cli-dir` no encuentra directorio con permisos de escritura, la instalación se omite silenciosamente 🟡
- **[Deep link sin nombre de grafo]:** Si la URL `logseq://` no especifica graph-name, se abre la app en el último grafo usado o se muestra la pantalla de selección de grafo 🔴

## Dependências
- `electron` — framework de aplicación desktop (BrowserWindow, app, protocol, ipcMain, Menu, shell)
- `fs-extra` — operaciones de sistema de archivos (lectura/escritura de assets, instalación CLI)
- `electron-log` — logging estructurado para el proceso main
- `electron-updater` — sistema de auto-actualización

## Requisitos Não Funcionais

| Tipo | Requisito inferido | Evidência no código | Confiança |
|------|--------------------|---------------------|-----------|
| Segurança | `contextIsolation: true` y `nodeIntegration: false` en BrowserWindow | `src/electron/electron/window.cljs` | 🟢 |
| Disponibilidade | Single-instance lock evita múltiples instancias conflictivas | `src/electron/electron/core.cljs:455` | 🟢 |
| Portabilidade | Menú y comportamiento de ventana adaptados por plataforma (macOS vs Win/Linux) | `src/electron/electron/core.cljs:set-app-menu!` | 🟢 |
| Manutenibilidade | IPC channels definidos con nombres explícitos y documentados | `src/electron/electron/handler.cljs` | 🟢 |

> Inferido a partir del código. Validar con equipo de operaciones.

## Critérios de Aceitação

### Cenário: Inicio normal de la aplicación
```gherkin
Dado que no hay otra instancia de Logseq corriendo
Cuando se ejecuta `main`
Então se obtiene el single instance lock
  Y se registran los esquemas privilegiados lsp:// y assets://
  Y se registra el protocol client "logseq"
  Y se configura el menú de aplicación según la plataforma
  Y se crea una BrowserWindow cargando la URL de Logseq
  Y se configuran los handlers IPC
  Y el servidor HTTP interno se inicia
```

### Cenário: Segunda instancia bloqueada y redirigida
```gherkin
Dado que ya existe una instancia de Logseq corriendo
Cuando se ejecuta `main` en un segundo proceso
Então `requestSingleInstanceLock` retorna false
  Y la instancia original recibe el evento `second-instance` con los argumentos
  Y la segunda instancia llama a `app/quit` inmediatamente
  Y la ventana de la instancia original se trae al frente (focus)
```

### Cenário: Deep link abre grafo específico
```gherkin
Dado que la aplicación Logseq está corriendo
  Y el usuario hace clic en un enlace `logseq://my-graph?page=Home`
Cuando el sistema operativo envía el evento `open-url` con esa URL
Então el handler parsea la URL y extrae `graph-name = "my-graph"` y `page = "Home"`
  Y se envía evento `:graph/switch` al renderer con el nombre del grafo
  Y la app cambia al grafo "my-graph" y navega a la página "Home"
```

### Cenário: Cierre de ventana en macOS con hide-to-tray
```gherkin
Dado que la app corre en macOS con una sola ventana abierta
  Y `quitting?` = false
Cuando el usuario cierra la ventana (click en botón rojo)
Então `close-handler` llama a `win.hide()` en lugar de `win.close()`
  Y la app sigue corriendo en background (visible en dock y tray)
  Y al hacer click en el icono del dock, la ventana se muestra de nuevo
```

### Cenário: Instalación de CLI launcher en Unix
```gherkin
Dado que la plataforma es Linux o macOS
  Y existe un directorio escribible en PATH (ej: ~/.local/bin)
Cuando se llama a `install-cli-launcher!`
Então se renderiza el script launcher con la ruta correcta al binario de Electron
  Y se escribe en `~/.local/bin/logseq`
  Y se aplica `chmod 755` al archivo
  Y al ejecutar `logseq` desde terminal, se abre la aplicación
```

## Prioridade

| Requisito | MoSCoW | Justificativa |
|-----------|--------|---------------|
| Inicialización de Electron app | Must | Sin esto la app desktop no existe |
| Creación de BrowserWindow | Must | La UI de Logseq se renderiza en esta ventana |
| Registro de protocolos lsp:// y assets:// | Must | Plugins y assets no funcionarían sin estos protocolos |
| IPC handlers | Must | Comunicación main↔renderer es esencial para todas las features |
| Single instance lock | Should | Importante para UX pero no bloquea funcionalidad core |
| Deep links (logseq://) | Should | Feature de integración con SO, no usada en flujo normal |
| Menú de aplicación nativo | Should | Mejora UX pero la app funciona sin menú personalizado |
| Auto-updater | Should | Conveniente pero no bloqueante para uso básico |
| CLI launcher | Could | Solo relevante para usuarios avanzados de terminal |
| Hide-to-tray en macOS | Could | Comportamiento específico de plataforma,不影响核心功能 |

> Prioridad inferida por frecuencia de llamada y posición en la cadena de dependencias.

## Cenários de Borda

### 1. Fallo al cargar la URL en BrowserWindow
**Situação:** La URL de Logseq no responde (servidor de desarrollo no iniciado, build corrupto, o archivo HTML no encontrado).
**Comportamento esperado:**
- `win.loadURL` falla con error capturable
- Si hay un servidor de desarrollo configurado, se intenta `win.loadURL` con URL de respaldo
- Si no, se muestra pantalla de error genérica ("Failed to load application")
- La ventana NO se cierra — el usuario puede intentar recargar con Cmd+R / Ctrl+R
- El error se registra en `electron-log`

### 2. Múltiples ventanas abiertas y cierre de aplicación
**Situação:** La app tiene 3 ventanas abiertas y el usuario selecciona "Quit" desde el menú.
**Comportamento esperado:**
- Se establece `quitting? = true` en el estado global
- Se itera sobre todas las ventanas llamando a `win.close()` en cada una
- Los handlers `"close"` detectan `quitting? = true` y permiten el cierre normal
- En macOS, NO se hace hide-to-tray porque `quitting?` es true
- Tras cerrar la última ventana, la app termina limpiamente

### 3. Protocolo lsp:// con path que contiene caracteres especiales
**Situação:** Un plugin solicita `lsp://plugin%20name/path/with%20spaces/file.js`.
**Comportamento esperado:**
- La URL se decodifica (`decodeURIComponent`) antes de resolver el path
- `plugin%20name` → `plugin name`
- `path/with%20spaces/file.js` → `path/with spaces/file.js`
- Se busca el archivo en `PLUGINS_ROOT/plugin name/path/with spaces/file.js`
- Si el plugin no existe, se retorna 404
- Si el archivo no existe, se retorna 404

## Rastreabilidade de Código

| Arquivo | Função / Classe | Cobertura |
|---------|-----------------|-----------|
| `src/electron/electron/core.cljs` | `main` | 🟢 |
| `src/electron/electron/core.cljs` | `on-app-ready!` | 🟢 |
| `src/electron/electron/core.cljs` | `setup-interceptor!` | 🟢 |
| `src/electron/electron/core.cljs` | `install-cli-launcher!` | 🟢 |
| `src/electron/electron/core.cljs` | `set-app-menu!` | 🟡 |
| `src/electron/electron/core.cljs` | `setup-deeplink!` | 🟡 |
| `src/electron/electron/window.cljs` | `create-main-window!` | 🟢 |
| `src/electron/electron/window.cljs` | `setup-window-listeners!` | 🟢 |
| `src/electron/electron/window.cljs` | `close-handler` | 🟢 |
| `src/electron/electron/handler.cljs` | `set-ipc-handler!` | 🟢 |
| `src/electron/electron/db.cljs` | Database initialization | 🟡 |
| `src/electron/electron/server.cljs` | `setup!` | 🟡 |
| `src/electron/electron/updater.cljs` | `setup-updater!` | 🟡 |
| `static/electron.js` | Electron main process entry | 🟡 |
