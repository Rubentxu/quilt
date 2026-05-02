# Flowchart: frontend/fs - File System

> Flowchart del sistema de archivos abstracto.

## Protocolo Fs - Métodos Principales

```mermaid
flowchart TD
    subgraph File Operations
        A[mkdir!]
        B[mkdir-recur!]
        C[readdir]
        D[read-file]
        E[read-file-raw]
        F[write-file!]
        G[rename!]
        H[copy!]
        I[unlink!]
        J[rmdir!]
    end

    subgraph Query Operations
        K[stat]
        L[open-dir]
        M[get-files]
    end

    subgraph Watch Operations
        N[watch-dir!]
        O[unwatch-dir!]
    end
```

## Implementaciones del Protocolo

```mermaid
flowchart LR
    subgraph Fs Protocol
        A[Fs protocol]
    end

    subgraph Implementations
        B[Node.js fs]
        C[Memory FS]
        D[Cloud FS]
    end

    A -->|Electron| B
    A -->|Browser/Memory| C
    A -->|Future| D
```

## Write File Flow

```mermaid
sequenceDiagram
    participant Caller as Handler
    participant Fs as Fs Implementation
    participant FS as Node fs

    Caller->>Fs: write-file!(repo, dir, path, content, opts)
    Fs->>Fs: normalize path
    Fs->>FS: ensure dir exists
    FS-->>Fs: done
    Fs->>FS: write file
    FS-->>Fs: done
    Fs-->>Caller: result
```

---

# Flowchart: frontend/format - Parsers

> Flowchart del sistema de parsing Markdown y Org-mode.

## Formato Protocol

```mermaid
flowchart LR
    subgraph Format Protocol
        A[toEdn]
        B[toHtml]
        C[exportMarkdown]
        D[exportOPML]
    end
```

## Mldoc Pipeline

```mermaid
sequenceDiagram
    participant Content as Raw Content
    participant Mldoc as Mldoc Library
    participant AST as AST
    participant Format as Format Module
    participant DB as Database

    Content->>Mldoc: parse(content, config)
    Mldoc-->>AST: s-expression AST
    
    AST->>Format: toEdn/AST
    Format->>DB: extract-pages-and-blocks
    
    Content->>Mldoc: export(config, refs)
    Mldoc-->>Format: html/markdown/opml
```

## AST Node Types

```mermaid
flowchart TD
    A[AST Node] --> B{Type}
    
    B -->|Heading| C["[Heading, {size, anchor}, content]"]
    B -->|Paragraph| D["[Paragraph, content]"]
    B -->|Code| E["[Code, {language}, code]"]
    B -->|List| F["[List, items]"]
    B -->|Quote| G["[Quote, content]"]
    B -->|Table| H["[Table, rows]"]
    B -->|Drawer| I["[Drawer, name, props]"]
    B -->|Timestamp| J["[Timestamp, {date, time}]"]
    B -->|Property| K["[Property, key, value]"]
```

---

# Flowchart: frontend/search - Búsqueda

> Flowchart del sistema de búsqueda.

## Agency Pattern

```mermaid
flowchart LR
    subgraph Agency
        A[query]
    end

    subgraph Engines
        B[Browser Engine]
        C[Plugin Engine 1]
        D[Plugin Engine N]
    end

    A -->|broadcast| B
    A -->|broadcast| C
    A -->|broadcast| D
    
    B -->|result| A
    C -->|result| A
    D -->|result| A
```

## Search Flow

```mermaid
sequenceDiagram
    participant UI as Search UI
    participant Agency as Agency
    participant Browser as Browser Engine
    participant Plugin as Plugin Engine
    participant Index as Search Index

    UI->>Agency: query(q, opts)
    Agency->>Browser: query(q)
    Browser->>Index: search
    Index-->>Browser: results
    Browser-->>Agency: browser-results
    
    Agency->>Plugin: query(q)
    Plugin-->>Agency: plugin-results
    
    Agency-->>UI: merged results
```

## Index Operations

```mermaid
flowchart TD
    subgraph Build Index
        A[rebuild-blocks-indice!]
        B[rebuild-pages-indice!]
    end

    subgraph Update Index
        C[transact-blocks!]
        D[truncate-blocks!]
    end

    subgraph Cleanup
        E[remove-db!]
    end
```

---

# Flowchart: frontend/components - UI Components

> Flowchart del sistema de componentes UI.

## Editor Component Architecture

```mermaid
flowchart LR
    subgraph Editor
        A[block-editor]
        B[content-editable]
        C[slash-commands]
        D[auto-complete]
    end

    A -->|edit| B
    A -->|commands| C
    C -->|suggest| D
    D -->|select| A
```

## Block Rendering Pipeline

```mermaid
sequenceDiagram
    participant DB as Database
    participant Model as db.model
    participant React as db/react
    participant Component as Block Component

    DB->>Model: query blocks
    Model->>DB: entities
    
    Model->>React: wrap with react
    React->>Component: reactive update
    
    Component->>Component: render with highlighting
```

## Command Palette Flow

```mermaid
flowchart TD
    A[User types /] --> B[show commands]
    B --> C{filter commands}
    C --> D[match results]
    D --> E[user selects]
    E --> F[execute command]
    F --> G[insert result]
```

---

*Flowcharts generados por Reversa Archaeologist*
