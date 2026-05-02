# Flowchart: outliner - Sistema Outliner

> Flowchart del sistema de operaciones de árbol/outliner.

## Arquitectura de Operaciones

```mermaid
flowchart TD
    subgraph API Layer
        A[save-block]
        B[insert-blocks]
        C[delete-blocks]
        D[move-blocks]
        E[indent-outdent-blocks]
    end

    subgraph Core
        F[outliner-core]
        G[outliner-tree]
        H[outliner-op]
    end

    subgraph Validation
        I[outliner-validate]
        J[malli schema]
    end

    A -->|1| F
    B -->|1| F
    C -->|1| F
    D -->|1| F
    E -->|1| F

    F -->|2| G
    F -->|3| J
    
    G -->|4| H
    
    H -->|5| I
    
    H -->|6| Transaction
```

## Protocolo INode

```mermaid
sequenceDiagram
    participant Entity as DataScript Entity
    participant INode as INode Protocol
    participant Core as outliner-core
    participant Tx as Transaction

    Entity->>INode: -save(*txs-state, db, opts)
    
    INode->>Core: dissoc-temp-fields
    Core->>Core: block-with-updated-at
    Core->>Core: fix-tag-ids
    Core->>Core: remove-disallowed-inline-classes
    
    Core->>Tx: conj tx-data to *txs-state
    Tx->>Tx: retract attributes
    Tx->>Tx: update page timestamps
    
    Entity->>INode: -del(*txs-state, db)
    INode->>Tx: retractEntity or batch delete
```

## Insert Blocks Flow

```mermaid
sequenceDiagram
    participant API as insert-blocks API
    participant Target as get-target-block
    participant Level as blocks-with-level
    participant Order as get-block-orders
    participant Build as build-insert-blocks-tx
    participant Temp as assign-temp-id
    participant DS as DataScript

    API->>Target: determine sibling?/child?
    Target-->>API: [target-block sibling?]
    
    API->>Level: calculate levels
    Level-->>API: leveled-blocks
    
    API->>Order: gen-n-keys
    Order-->>API: orders
    
    API->>Build: build tx
    Build-->>API: blocks-tx
    
    API->>Temp: assign temp ids
    Temp-->>API: full-tx
    
    API->>DS: d/transact! with tx
```

## Delete Blocks Flow

```mermaid
flowchart TD
    A[delete-blocks] --> B[filter-top-level-blocks]
    B --> C{consecutive?}
    C -->|no| D[sort-non-consecutive]
    C -->|yes| E[get-top-level-blocks]
    D --> F
    
    E --> F[validate-not-built-in]
    F --> G{built-in?}
    G -->|yes| H[throw error]
    G -->|no| I
    
    I[batch-delete] --> J{default-value-property?}
    J -->|yes| K[set-empty-placeholder]
    J -->|no| L[batch retractEntity]
    
    K --> M[return tx-data]
    L --> M
```

## Move Blocks Flow

```mermaid
sequenceDiagram
    participant Move as move-blocks
    participant Filter as filter-top-level-blocks
    participant Target as get-target-block
    participant Validate as validate-not-cyclic
    participant Tx as ldb/batch-transact

    Move->>Filter: identify top-level
    Filter-->>Move: top-level-blocks
    
    Move->>Target: determine new position
    Target-->>Move: [target sibling?]
    
    Move->>Validate: check-not-moving-to-child
    Validate-->>Move: valid?
    
    alt valid
        Move->>Tx: for each block
        Tx->>Tx: move-block
        Tx->>Tx: update children page-refs
        Tx->>Tx: handle created-from property
    else invalid
        Move->>Move: throw ex-info
    end
```

## Tree to Vec Conversion

```mermaid
flowchart LR
    A[blocks] --> B[group-by :block/parent]
    B --> C[sort-by-order for each group]
    C --> D[block-children recursive]
    D --> E{has children?}
    E -->|yes| F[recurse]
    E -->|no| G[leaf node]
    F --> H[:block/children]
    G --> I[add level]
    H --> I
    I --> J[build tree]
    J --> K[vec-tree result]
```

## Block Navigation

```mermaid
flowchart TD
    A[get-next block] --> B{collapsed?}
    B -->|yes| C[get-right-sibling]
    B -->|no| D{has-children?}
    D -->|yes| E[return-first-child]
    D -->|no| F[get-right-sibling]
    
    C --> G[result]
    E --> G
    F --> G

    A2[get-prev block] --> B2{has-prev-sibling?}
    B2 -->|yes| C2[has-children & not-collapsed?]
    C2 -->|yes| D2[get-deep-last-open]
    C2 -->|no| E2[return-prev-sibling]
    B2 -->|no| F2[return-parent]
    
    D2 --> G2[result]
    E2 --> G2
    F2 --> G2
```

## Indent/Outdent Algorithm

```mermaid
flowchart TD
    A[indent-outdent] --> B[filter-top-level]
    B --> C{indent?}
    C -->|yes| D[left-sibling exists?]
    C -->|no| E{can-outdent?}
    
    D -->|yes| F[new-parent = left-sibling]
    D -->|no| G[throw - cannot indent]
    
    E -->|yes| H[new-parent = parent-of-parent]
    E -->|no| I[throw - cannot outdent]
    
    F --> J[reparent blocks]
    H --> J
    
    J --> K[regenerate orders]
    K --> L[transact!]
```

## Operation Validation

```mermaid
flowchart TD
    A[Operation] --> B[validate-built-in-entity?]
    B --> C{is-built-in?}
    C -->|yes| D[throw: cannot modify]
    C -->|no| E[validate-schema]
    
    E --> F{valid?}
    F -->|no| G[throw: invalid schema]
    F -->|yes| H[execute-op]
    
    A2[save-block] --> I[validate-page-title]
    I --> J{valid-chars?}
    J -->|no| K[throw: invalid title]
    J -->|yes| L[OK]
```

---

*Flowchart generado por Reversa Archaeologist*
