# Flowchart: electron - Desktop Application

> Flowchart del módulo Electron para la aplicación desktop.

## Inicialización Principal

```mermaid
sequenceDiagram
    participant App as Electron App
    participant Main as main process
    participant Window as Window Manager
    participant IPC as IPC Handlers
    participant Server as HTTP Server

    App->>Main: will-finish-launching
    Main->>Main: requestSingleInstanceLock
    
    alt lock acquired
        Main->>Main: registerSchemesAsPrivileged
        Main->>Main: registerDefaultProtocolClient
        Main->>Main: set-app-menu!
        Main->>Main: setup-deeplink!
        Main->>Main: on-app-ready!
        
        Main->>Window: create-main-window!
        Window-->>Main: win
        
        Main->>Main: setup-interceptor!
        Main->>Main: setup-updater!
        Main->>Main: setup-app-manager!
        Main->>Main: set-ipc-handler!
        Main->>Main: server/setup!
        
        Main->>Window: window "close" listener
    else lock not acquired
        Main->>App: quit
    end
```

## Window Management

```mermaid
flowchart TD
    A[create-main-window!] --> B[BrowserWindow options]
    B --> C[loadURL logseq]
    C --> D[setup-window-listeners!]
    
    D --> E{window events}
    E -->|"close"| F[close-handler]
    E -->|"maximize"| G[toggle-max-or-min]
    E -->|"focus"| H[bring-to-front]
    
    F --> I{multiple windows?}
    F -->|yes| J[close normally]
    F -->|no| K{mac?}
    K -->|yes| L[hide to tray]
    K -->|no| M[close normally]
```

## Protocol Handlers

```mermaid
flowchart LR
    subgraph lsp:// protocol
        A[lsp:// url] --> B[parse pathname]
        B --> C{PLUGIN_URL?}
        C -->|yes| D[resolve to PLUGINS_ROOT]
        C -->|no| E[resolve to STATIC_URL]
        D --> F[serve file]
        E --> F
    end

    subgraph assets:// protocol
        G[assets:// url] --> H[decode path]
        H --> I{absolute path?}
        I -->|yes| J[serve directly]
        I -->|no| K[Windows UNC?]
        K -->|yes| L[serve as UNC]
        K -->|no| M[warn + serve]
    end
```

## IPC Handler Setup

```mermaid
flowchart TD
    subgraph IPC Channels
        A["toggle-max-or-min-active-win"]
        B["call-application"]
        C["call-main-win"]
        D["export-publish-assets"]
        E["set-quit-dirty-state"]
    end

    A -->|toggle-min?| F[minimize/restore]
    B -->|invoke| G[app invoke]
    C -->|invoke| H[window invoke]
    D -->|publish-export| I[create-export]
    E -->|dirty?| J[set-quit-dirty?]
```

## Deep Link Handling

```mermaid
sequenceDiagram
    participant OS as Operating System
    participant App as Electron App
    participant Handler as URL Handler

    OS->>App: open-url event (macOS)
    OS->>App: command-line args (Windows)
    
    App->>Handler: open-url-handler(win, url)
    
    Handler->>Handler: js/URL.parse
    Handler->>Handler: check scheme = "logseq:"
    
    Handler->>Handler: logseq-url-handler
    
    alt new-window
        Handler->>App: open-new-window-or-tab!
    else switch-graph
        Handler->>App: graph-switch
    end
```

## App Menu Setup

```mermaid
flowchart TD
    A[set-app-menu!] --> B{detect platform}
    
    B -->|macOS| C[app menu template]
    B -->|Windows/Linux| D[file/edit/view menu]
    
    C --> E[about, services, hide, quit]
    D --> E
    
    E --> F[+ file menu]
    F --> F1[new-window]
    F --> F2[close/quit]
    
    E --> G[+ edit menu]
    E --> H[+ view menu]
    E --> I[+ window menu]
    E --> J[+ help menu]
    
    G --> K[undo, redo, cut, copy, paste]
```

## CLI Launcher Installation

```mermaid
flowchart TD
    A[install-cli-launcher!] --> B{platform}
    
    B -->|Unix| C[preferred-unix-cli-dir]
    B -->|Windows| D[preferred-win-cli-dir]
    
    C --> E{find writable dir?}
    D --> E
    
    E -->|found| F[render launcher script]
    E -->|not found| G[skip install]
    
    F --> H{exists?}
    H -->|no| I[write + chmod 755]
    H -->|yes + different| J[overwrite]
    H -->|yes + same| K[skip]
```

---

*Flowchart generado por Reversa Archaeologist*
