# Flowchart: frontend/handler - Event Handlers

> Flowchart detallado del sistema de eventos centralizado de Logseq.
> Generado por Reversa Archaeologist | Nivel: detalhado

## 1. Arquitectura General del Sistema de Eventos

```mermaid
flowchart TD
    subgraph ExternalSources["Fuentes Externas"]
        UA[User Actions]
        PH[Plugin Hooks]
        RTC[RTC Events]
        DBW[DB Worker]
    end

    subgraph EventChannel["Event Channel"]
        CH[(events channel)]
    end

    subgraph EventLoop["Event Loop"]
        RUN[run!]
        HANDLE[handle multimethod]
    end

    subgraph Handlers["Event Handlers"]
        subgraph GraphHandlers["Graph Handlers"]
            GS[:graph/switch]
            GR[:graph/restored]
            GY[:graph/ready]
            GSS[:graph/save-db-to-disk]
        end

        subgraph PageHandlers["Page Handlers"]
            PC[:page/create]
            PD[:page/deleted]
            PR[:page/renamed]
        end

        subgraph EditorHandlers["Editor Handlers"]
            ES[:editor/save-current-block]
            EH[:editor/set-heading]
            EU[:editor/upsert-type-block]
            EQ[:editor/quick-capture]
            ET[:editor/toggle-own-number-list]
        end

        subgraph DBHandlers["DB Handlers"]
            DS[:db/sync-changes]
            DE[:db/export-sqlite]
        end

        subgraph RTCHandlers["RTC Handlers"]
            RS[:rtc/sync-state]
            RP[:rtc/presence-update]
            RD[:rtc/download-remote-graph]
        end

        subgraph UIHandlers["UI Handlers"]
            UR[:ui/re-render-root]
            NT[:notification/show]
            SR[:search/rebuild]
        end
    end

    UA -->|pub-event!| CH
    PH -->|emit| CH
    RTC -->|sync-state| CH
    DBW -->|thread-api| CH

    CH -->|async/go-loop| RUN
    RUN --> HANDLE

    HANDLE --> GS
    HANDLE --> PC
    HANDLE --> ES
    HANDLE --> DS
    HANDLE --> RS
    HANDLE --> UR

    GS --> GY
    GS --> GR
    GR --> GSS
    PC --> PD
```

## 2. Event Loop Principal (events.cljs)

```mermaid
flowchart LR
    subgraph Init["Inicialización"]
        START[start event loop]
        CHAN[get-events-chan]
    end

    subgraph Loop["Async Go Loop"]
        RECV[recibir payload]
        MATCH[match event type]
        DISP[dispatch via multimethod]
        ERR[manejo errores]
    end

    subgraph Sync["Operaciones"]
        DB1[db/transact!]
        DB2[db-async/<get-block]
        SEARCH[search/block-search]
    end

    START --> CHAN
    CHAN --> RECV
    RECV --> MATCH
    MATCH -->|defmethod| DISP
    DISP -->|sync| DB1
    DISP -->|async| DB2
    DB2 --> SEARCH
    ERR -->|capture-error| SENTRY[Sentry]
    ERR -->|log| LOG[logseq.logging]
```

### Código del Event Loop (events.cljs:419-439)

```
run! → async/go-loop → handle (multimethod)
                           ↓
                      try/catch
                           ↓
              p/then (resolve result)
              p/catch (log error + capture-error)
```

## 3. Flujo de Evento de Edición (editor.save)

```mermaid
sequenceDiagram
    participant UI as UI/Editor Component
    participant State as frontend.state
    participant Chan as Event Channel
    participant Events as events.cljs
    participant Editor as editor.cljs
    participant DB as DB Worker
    participant Pipeline as outliner pipeline

    UI->>State: editor/save-current-block
    State->>Chan: pub-event!
    Chan->>Events: receive event
    Events->>Editor: handle [:editor/save-current-block]

    Editor->>Editor: save-current-block!

    alt has changes?
        Editor->>DB: db/transact!
        DB-->>Editor: tx result
        Editor->>Pipeline: invoke-hooks
        Pipeline->>State: update state
    end

    Events-->>Chan: result
```

## 4. Graph Switch Flow Detallado

```mermaid
flowchart TD
    A[:graph/switch event] --> B[export/cancel-db-backup!]
    B --> C[state/set-state! :db/async-queries {}]
    C --> D[st/refresh!]
    D --> E[graph-switch-on-persisted]

    E --> F[repo-handler/restore-and-setup-repo!]
    F --> G[db-restore/restore-graph!]
    G --> H[repo-config-handler/restore-repo-config!]
    H --> I{global-config-enabled?}

    I -->|yes| J[global-config-handler/restore!]
    I -->|no| K[continuar]
    J --> K

    K --> L[ui-handler/add-style-if-exists!]
    L --> M[graph-switch]
    M --> N[page-handler/init-commands!]
    N --> O[repo-config-handler/restore-repo-config!]
    O --> P[route-handler/redirect-to-home!]
    P --> Q[graph-handler/settle-metadata-to-local!]

    Q --> R{rtc-download?}
    R -->|yes| S[repo-handler/refresh-repos!]
    R -->|no| T[schedule-search-index-build!]

    S --> T
    T --> U[export/backup-db-graph]
    U --> V[log info]

    style R fill:#f9f,stroke:#333
```

## 5. Editor Operations Flow

```mermaid
flowchart TD
    subgraph Input["User Input"]
        KEY[keyboard event]
        CLICK[click event]
        PASTE[paste event]
    end

    KEY --> ED[editor-handler]
    CLICK --> ED
    PASTE --> ED

    subgraph EditorOps["Editor Operations"]
        SAVE[save-current-block!]
        INSERT[insert-new-block!]
        DELETE[delete-block!]
        FORMAT[format-text!]
        CYCLE[cycle-todo!]
    end

    ED --> SAVE
    ED --> INSERT
    ED --> DELETE
    ED --> FORMAT
    ED --> CYCLE

    SAVE --> WRAP[wrap-parse-block]
    WRAP --> OUTLINER[outliner-op/save-block!]
    OUTLINER --> TX[ui-outliner-tx/transact!]

    INSERT --> SPLIT[compute-fst-snd-block-text]
    SPLIT --> NEWBLOCK[insert-new-block-aux!]
    NEWBLOCK --> OUTLINER2[outliner-op/insert-blocks!]

    DELETE --> GETSTATE[get-state]
    GETSTATE --> DELETEINNER[delete-block-inner!]
    DELETEINNER --> OUTLINER3[outliner-op/delete-blocks!]

    TX --> PIPELINE[pipeline/invoke-hooks]
    OUTLINER2 --> PIPELINE
    OUTLINER3 --> PIPELINE

    PIPELINE --> DB[db/sync-changes event]
    DB --> UPDATE[frontend.components/*]
```

## 6. Page Create Flow

```mermaid
flowchart TD
    A[:page/create event] --> B{today journal?}
    B -->|yes| C[page-handler/create-today-journal!]
    B -->|no| D[page-common-handler/<create!]

    C --> C1[date/today]
    C1 --> C2[state/set-today!]
    C2 --> C3[db/get-today-journal-page]
    C3 --> C4{exists?}
    C4 -->|no| C5[ui-outliner-tx/transact!]
    C5 --> C6[outliner-op/create-page!]
    C6 --> C7[plugin-handler/hook-plugin-app :today-journal-created]

    D --> D1[wrap-tags if # present]
    D1 --> D2[db-editor-handler/wrap-parse-block]
    D2 --> D3{has tags?}
    D3 -->|yes| D4[validate tags]
    D3 -->|no| D5[create page]
    D4 --> D6{valid?}
    D6 -->|no| D7[notification.show! error]
    D6 -->|yes| D5
    D5 --> D8[ui-outliner-tx/transact!]
    D8 --> D9[route-handler/redirect-to-page!]
```

## 7. Search System Flow

```mermaid
flowchart TD
    subgraph SearchInit["Search Initialization"]
        RESTORED[:graph/restored]
        READY[:graph/ready]
    end

    RESTORED --> SCHEDULE[schedule-search-index-build!]
    READY --> BUILD[<build-search-index!>]

    SCHEDULE --> IDLE{input-idle?}
    IDLE -->|no| SCHEDULE
    IDLE -->|yes| BUILD

    BUILD --> WORKER[db-worker thread-api/search-build-blocks-indice]

    WORKER --> SUCCESS{success?}
    SUCCESS -->|yes| LOG[log info]
    SUCCESS -->|no| ERROR[console.error]
    LOG --> REBUILD[schedule 5s retry]
```

## 8. RTC Collaboration Flow

```mermaid
flowchart LR
    subgraph RemoteEvents["Remote Events"]
        RS[:rtc/sync-state]
        RP[:rtc/presence-update]
        RD[:rtc/download-remote-graph]
    end

    RS --> UPD[state/update-state!]
    RP --> PRES[rtc-handler/<rtc-update-presence!]
    RD --> DOWN[rtc-handler/<rtc-download-graph!]

    UPD --> MERGE[merge state]
    PRES --> TX[transact presence]
    DOWN --> GRAPHS[refresh remote graphs]

    MERGE --> SYNC[rtc-flows/trigger-rtc-start]
    TX --> PIPELINE[pipeline/invoke-hooks]
```

## 9. Plugin Hook System

```mermaid
flowchart TD
    TX[db/transact!] --> HOOK[:plugin/hook-db-tx event]
    HOOK --> PAYLOAD[merge tx-data]

    PAYLOAD --> PLUGIN1[plugin-handler/hook-plugin-db]
    PAYLOAD --> PLUGIN2[plugin-handler/hook-plugin-block-changes]

    PLUGIN1 --> ONDBTX[plugin.onDbTx hook]
    PLUGIN2 --> ONBLOCK[plugin.onBlockChanged hook]

    ONDBTX --> PLUGINCODE[Plugin User Code]
    ONBLOCK --> PLUGINCODE
```

## 10. Block Handler (block.cljs) - Operaciones Principales

```mermaid
flowchart TD
    subgraph BlockOps["Block Operations"]
        EDIT[edit-block!]
        SELECT[select-block!]
        TOUCH[touch events]
    end

    EDIT --> BLOCK[db/entity]
    BLOCK --> RECYCLED{recycled?}
    RECYCLED -->|yes| WARN[notification.show! readonly]
    RECYCLED -->|no| EDITING[state/set-editing!]
    EDITING --> MARK[mark-last-input-time!]

    SELECT --> GETBLOCKS[util/get-blocks-by-id]
    GETBLOCKS --> SETSELECT[state/exit-editing-and-set-selected-blocks!]

    TOUCH --> START[on-touch-start]
    START --> MOVE[on-touch-move]
    MOVE --> END[on-touch-end]
```

## 11. Repo Handler (repo.cljs) - Gestión de Grafos

```mermaid
flowchart TD
    subgraph RepoOps["Repository Operations"]
        NEW[new-db!]
        REMOVE[remove-repo!]
        RESTORE[restore-and-setup-repo!]
        REFRESH[refresh-repos!]
    end

    NEW --> EXISTS{graph-already-exists?}
    EXISTS -->|yes| ERROR[notification.show! error]
    EXISTS -->|no| CREATE[create-db]
    CREATE --> PERSIST[persist-db/<new]
    PERSIST --> START[db/start-db-conn!]
    START --> ADD[state/add-repo!]
    ADD --> SETUP[restore-and-setup-repo!]

    REMOVE --> DELCONN[db/remove-conn!]
    DELCONN --> DELPERSIST[db-persist/delete-graph!]
    DELPERSIST --> DELSEARCH[search/remove-db!]
    DELSEARCH --> DELSTATE[state/delete-repo!]

    RESTORE --> RESTDB[db-restore/restore-graph!]
    RESTDB --> RESTCONFIG[repo-config-handler/restore-repo-config!]
```

## 12. Estructura de Archivos del Módulo Handler

```
handler/
├── events.cljs              # Event loop principal (multimethod handle)
├── events/
│   ├── ui.cljs              # UI events (dialogs, modals, etc)
│   ├── rtc.cljs             # Real-time collaboration events
│   └── export.cljs          # Export events
├── editor.cljs              # ~3600 líneas - Editor principal
├── editor/
│   └── lifecycle.cljs        # Editor lifecycle
├── page.cljs                # Page utilities
├── block.cljs               # Block utilities
├── repo.cljs                # Repository/graph management
├── search.cljs             # Search handling
├── ui.cljs                 # UI state management
├── common/
│   ├── editor.cljs         # Shared editor utilities
│   ├── page.cljs           # Shared page utilities
│   └── ...
├── db_based/
│   ├── editor.cljs         # DB-based editor ops
│   ├── page.cljs           # DB-based page ops
│   ├── property.cljs       # Property handling
│   ├── rtc_flows.cljs     # RTC flows
│   └── sync.cljs           # DB sync
└── ... (50+ archivos más)
```

---

## Key Findings

| Aspecto | Detalle |
|---------|---------|
| **Event Loop** | core.async channel + multimethod dispatch |
| **Error Handling** | try/catch + Sentry capture + logging |
| **Transactions** | promesa.core promises + p/do! pattern |
| **State Updates** | pipeline/invoke-hooks → component re-render |
| **Plugin Integration** | hook-db-tx y onBlockChanged hooks |
| **RTC** | WebSocket state sync + presence updates |

**Escala de Confianza:**
- 🟢 CONFIRMADO: Event loop, multimethod dispatch, editor operations
- 🟢 CONFIRMADO: Page create/delete/rename flows
- 🟢 CONFIRMADO: Graph switch y repo management
- 🟡 INFERIDO: RTC internals (requiere código de electron)
- 🟡 INFERIDO: Plugin hook implementation details

---

*Flowchart generado por Reversa Archaeologist*
