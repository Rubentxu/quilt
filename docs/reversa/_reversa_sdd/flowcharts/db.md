# Flowcharts — Módulo `frontend/db`

> Documentación detallada del flujo de datos en el módulo de base de datos DataScript de Logseq.

## 1. Arquitectura General del Módulo DB

```mermaid
graph TB
    subgraph "Capa de Aplicación"
        Handler[frontend.handler]
        Components[frontend.components]
    end

    subgraph "Módulo frontend/db"
        subgraph "Connection Management"
            conn[conn.cljs<br/>get-db<br/>transact!]
            conn_state[conn_state.cljs<br/>conns atom]
        end

        subgraph "Model Layer"
            model[model.cljs<br/>get-block-by-uuid<br/>get-page<br/>get-journal-page<br/>has-children?<br/>get-next<br/>get-prev]
            utils[utils.cljs<br/>entity<br/>pull<br/>pull-many<br/>q]
        end

        subgraph "Transaction Layer"
            transact[transact.cljs<br/>transact<br/>apply-outliner-ops]
            react[react.cljs<br/>q<br/>refresh!<br/>refresh-affected-queries!]
        end

        subgraph "Query Layer"
            query_dsl[query_dsl.cljs<br/>parse<br/>query<br/>custom-query]
            query_react[query_react.cljs<br/>react-query]
            query_custom[query_custom.cljs<br/>custom-query]
        end

        subgraph "Async Layer"
            async[async.cljs<br/>&lt;get-block<br/>&lt;get-blocks<br/>&lt;get-block-parents]
        end

        subgraph "Persistence Layer"
            persist[persist.cljs<br/>get-all-graphs<br/>delete-graph!]
            restore[restore.cljs<br/>restore-graph!]
        end
    end

    subgraph "DataScript Core"
        DS[DataScript Core<br/>d/q<br/>d/transact!<br/>d/entity<br/>d/pull]
    end

    subgraph "External Dependencies"
        ldb[logseq.db<br/>deps/db]
        outliner[logseq.outliner<br/>deps/outliner]
    end

    Handler -->|event tx-data| transact
    Components -->|queries| model
    Components -->|react queries| react
    Components -->|async queries| async

    conn --> conn_state
    transact --> |transact!| DS
    react --> |d/q| DS
    model --> utils
    utils --> |d/entity<br/>d/q| DS

    async --> |worker call| outliner
    transact --> |worker call| outliner

    persist --> |persist to disk| DS
    restore --> |restore from disk| DS
```

## 2. Flujo de una Transacción Típica

```mermaid
sequenceDiagram
    participant Handler as Event Handler
    participant Transact as frontend.db.transact
    participant Outliner as logseq.outliner.op
    participant Worker as DB Worker Thread
    participant DS as DataScript

    Note over Handler: User edits block

    Handler->>Transact: transact(repo, tx-data, tx-meta)

    Transact->>Transact: ensure-local-op-tx-id(tx-meta)

    Transact->>Transact: associate :local-tx? true

    Transact->>Worker: worker-call(request-f)

    Worker->>Outliner: apply-ops!(conn, ops, opts)

    Outliner->>Outliner: validate operations

    Outliner->>Outliner: calculate new orders

    Outliner->>DS: d/transact!(conn, tx-data, tx-meta)

    DS-->>Outliner: tx result

    Outliner-->>Worker: result

    Worker-->>Transact: promise resolved

    Transact->>react: refresh-affected-queries!(repo, keys)

    react->>DS: re-execute queries

    DS-->>react: new results

    react-->>Handler: component re-render
```

### Detalle de Transacción con Outliner Ops

```mermaid
flowchart TD
    Start([User Action]) --> TX{Transact Type?}

    TX -->|apply-outliner-ops| OutlinerOp[Outliner Operation]
    TX -->|simple transact| SimpleTx[Simple Transaction]

    OutlinerOp --> Validate{Validate Op?}
    Validate -->|Yes| Valid[Operation Valid]
    Validate -->|No| Reject[Reject Operation]

    Valid --> CalcOrder[Calculate New Orders]
    CalcOrder --> CheckParent{Parent Changed?}

    CheckParent -->|Yes| Orphan[Handle Orphan]
    CheckParent -->|No| NoOrphan[No Orphan]

    Orphan --> UpdateTree[Update Tree Structure]
    NoOrphan --> UpdateTree

    UpdateTree --> BuildTxData[Build Tx Data]
    BuildTxData --> DS[DataScript transact!]

    SimpleTx --> DS

    DS --> Notify[Notify Listeners]
    Notify --> Refresh[Refresh Queries]

    Refresh --> End([Done])
```

## 3. Flujo de Ejecución de Queries

```mermaid
sequenceDiagram
    participant UI as React Component
    participant ReactQ as frontend.db.react/q
    participant QueryState as *query-state atom
    participant DSL as frontend.db.query_dsl
    participant DS as DataScript
    participant Worker as DB Worker

    UI->>ReactQ: query(repo, key, query-opts, query, inputs)

    ReactQ->>QueryState: get-query-cached-result(k)

    alt Cache Hit
        QueryState-->>UI: cached result atom
    else Cache Miss
        ReactQ->>ReactQ: create result-atom

        ReactQ->>DSL: parse-query(query-string, db, opts)

        DSL->>DSL: pre-transform(s)

        DSL->>DSL: reader/read-string

        DSL->>DSL: build-query(form)

        DSL-->>ReactQ: {query, rules, sort-by, blocks?}

        ReactQ->>ReactQ: query-wrapper(where)

        ReactQ->>DS: d/q query inputs

        alt Simple Query
            DS-->>ReactQ: results
        else Advanced Query
            ReactQ->>Worker: <invoke-db-worker
            Worker-->>ReactQ: promise
        end

        ReactQ->>QueryState: add-q!(k, ...)

        ReactQ-->>UI: result-atom
    end

    Note over UI: Component subscribes to result-atom
```

### Query DSL Parser Flow

```mermaid
flowchart TD
    Input["Query String<br/>'(and [[page]] (task A))'"] --> Parse["parse(s, db, opts)"]

    Parse --> PreTransform["pre-transform(s)"]
    PreTransform --> ReadString["reader/read-string<br/>custom-readers"]
    ReadString --> Simplify["simplify-query(form)"]

    Simplify --> BuildQuery["build-query(form, env, level)"]

    subgraph "build-query recursion"
        BuildQuery --> FE{First Element}
        FE -->|and/or/not| BoolOp[Boolean Operator]
        FE -->|between| Between[Time Filter]
        FE -->|property| Property[Property Filter]
        FE -->|task| Task[Task Filter]
        FE -->|page| Page[Page Filter]
        FE -->|page-ref| PageRef[Page Reference]
        FE -->|datalog clause| Datalog[Datalog Clause]

        BoolOp --> Recurse["build-query for each clause"]
        Recurse --> Merge[Merge results]
        Merge --> Output
    end

    Output --> AddBindings["add-bindings!(query)"]
    AddBindings --> Wrapper["query-wrapper(where)"]
    Wrapper --> Result["{query, rules, sort-by, blocks?}"]
```

## 4. Reactive Query System

```mermaid
flowchart TB
    subgraph "Query Registration"
        Component[React Component] --> Register[add-query-component!]
        Register --> Map1[component-&gt;query-key]
        Register --> Map2[query-key-&gt;components]
    end

    subgraph "Query Execution"
        NewTx[New Transaction] --> Refresh[refresh!]
        Refresh --> GetAffected[Get Affected Keys]
        GetAffected --> Filter[Filter by repo]
        Filter --> ForEach[For each affected key]

        ForEach --> Execute[execute-query!]
        Execute --> ReRun[Re-run query]
        ReRun --> Transform[transform-fn]
        Transform --> Compare{Changed?}
        Compare -->|Yes| Notify[set-new-result!]
        Compare -->|No| Skip[Skip]
        Notify --> Done[Done]
    end

    subgraph "Custom Query Scheduling"
        Chan[(reactive-custom-queries-chan)]
        ForEach --> Queue[Queue to chan]
        Queue --> IdleCheck{Input Idle?}

        IdleCheck -->|Yes| Run[Execute]
        IdleCheck -->|No| Wait[Wait 2s]
        Wait --> IdleCheck
        Run --> Done
    end
```

## 5. Bloqueo de Datos y Persistencia

```mermaid
flowchart LR
    subgraph "Write Path"
        Edit[User Edit] --> Transact[transact]
        Transact --> Validate[Validate & Transform]
        Validate --> Ops[Outliner Ops]
        Ops --> DS[(DataScript)]
        DS --> Notify[Notify Watches]
    end

    subgraph "Persistence"
        DS --> Persist[persist-db]
        Persist --> IDB[(IndexedDB)]
        Persist --> SQLite[(SQLite)]
        Persist --> File[Local File]
    end

    subgraph "Read Path"
        Query[Query] --> Cache[Check Cache]
        Cache -->|Miss| DS
        DS --> Result
    end
```

## 6. Estructura de Datos DataScript - Schema

```mermaid
erDiagram
    Block ||--o{ Block : "block/parent"
    Block ||--|| Page : "block/page"
    Block ||--o{ Block : "block/refs"
    Block ||--o{ Tag : "block/tags"
    Block ||--o{ File : "file/path"

    Page ||--o{ Block : "contains"
    Page ||--o{ Page : "block/alias"
    Page ||--o{ Tag : "block/tags"
    Page ||--o| Journal : "block/journal-day"

    Block {
        uuid block_uuid PK
        string block_name
        string block_title
        ref block_page FK
        ref block_parent FK
        int block_order
        boolean block_collapsed
        refs block_refs
        refs block_tags
        long block_created_at
        long block_updated_at
    }

    Page {
        uuid block_uuid PK
        string block_name UK
        string block_title
        int block_journal_day
        ref block_namespace
    }

    File {
        string file_path PK
        string file_content
        long file_created_at
        long file_last_modified_at
    }

    Tag {
        uuid block_uuid PK
        string block_title
    }
```

## 7. Batched Block Fetching

```mermaid
flowchart TD
    Start[&lt;get-block calls] --> Batch?

    Batch? -->|Enabled| Enqueue[enqueue-get-blocks-request!]
    Enqueue --> Schedule[schedule-get-blocks-batch-flush!]
    Schedule --> Accumulate[Accumulate in queue]

    Batch? -->|Disabled| Direct[Direct worker call]

    Accumulate --> Timer{Flush Timer}

    Timer -->|Tick| Flush[flush-get-blocks-batch!]
    Flush --> Group[Group by graph]
    Group --> Invoke[&lt;invoke-worker-get-blocks]

    Direct --> Invoke

    Invoke --> Transit[Serialize requests]
    Transit --> Worker[(DB Worker)]

    Worker --> Process[Process batch]
    Process --> Response[Transit response]
    Response --> Resolve[resolve-batched-get-blocks!]

    Resolve --> Return[Return promises]
```

## Convenciones de Color

- 🟢 **Confirmado**: Extraído directamente del código fuente
- 🟡 **Inferido**: Basado en patrones observados
- 🔴 **Lacuna**: No determinable desde el código

## Métricas del Módulo

| Métrica | Valor |
|---------|-------|
| Archivos principales | 14 |
| Líneas de código | ~2,500 |
| Entidades schema | 4 (Block, Page, File, Tag) |
| Tipos de queries | 2 (DSL, Datalog puro) |
| Sistema reactivo | Yes (Rum reactive) |
| Concurrencia | core.async + promesa |
