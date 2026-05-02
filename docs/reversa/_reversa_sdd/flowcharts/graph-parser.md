# Flowchart: graph-parser

> Flowchart del módulo de parsing de archivos a estructuras de datos.

## Flujo Principal de Extracción

```mermaid
sequenceDiagram
    participant File as File Content
    participant Detect as Format Detection
    participant Mldoc as Mldoc Parser
    participant Extract as extract.cljc
    participant Block as block.cljs
    participant DB as DataScript

    File->>Detect: get-format(file-path)
    Detect-->>Extract: :markdown | :org
    
    Extract->>Mldoc: mldoc/->edn(content, config)
    Mldoc-->>Extract: AST
    
    Extract->>Extract: get-page-name(file, ast)
    Extract->>Extract: extract-properties(first-ast-block)
    
    Extract->>Block: extract-blocks(ast, content, format)
    Block-->>Extract: [blocks with refs]
    
    Extract->>Extract: build-page-map(properties)
    Extract->>Extract: build-pages-aux(namespaces)
    
    Extract-->>DB: {:pages [...], :blocks [...], :ast}
```

## Page Name Parsing

```mermaid
flowchart TD
    A[file-name] --> B{filename-format}
    
    B -->|:triple-lowbar| C[tri-lb-title-parsing]
    B -->|:legacy| D[legacy-title-parsing]
    
    C --> C1[decode-namespace-underlines]
    C1 --> C2[safe-url-decode]
    C2 --> C3[make-valid-namespaces]
    C3 --> E
    
    D --> D1[replace . with /]
    D1 --> D2[safe-decode-uri-component]
    D2 --> E[page-name]
    
    A2[file content] --> F{first block}
    F -->|has title::| G[use title property]
    F -->|heading| H[use heading content]
    F -->|none| E
```

## Block Extraction Pipeline

```mermaid
flowchart TD
    A[AST] --> B{Block Type?}
    
    B -->|Heading| C[construct-block]
    B -->|Property| D[extract-properties]
    B -->|Timestamp| E[extract-timestamps]
    B -->|Paragraph| F[check-timestamp]
    B -->|Other| G[add-to-body]
    
    C --> H{pre-block?}
    H -->|yes| I[attach-to-prev-block]
    H -->|no| J[as-separate-block]
    
    D --> K{merge-with-prev?}
    K -->|yes| L[accumulate]
    K -->|no| M[new-block]
    
    E --> N[accumulate-timestamps]
    F -->|has-ts| N
    F -->|no| G
    
    G --> O{end-of-section?}
    O -->|no| B
    O -->|yes| P[flush-body]
    
    P --> Q[apply-timestamps-to-last-heading]
    Q --> R[blocks with metadata]
```

## Properties Extraction

```mermaid
flowchart LR
    A[AST Properties Block] --> B[map first/last]
    B --> C{valid-key?}
    C -->|yes| D[keywordize]
    C -->|no| E[track invalid]
    
    D --> F{property-type}
    F -->|ref| G[parse-page-refs]
    F -->|string| H[parse-text]
    F -->|number| I[parse-number]
    
    G --> J[build-ref-list]
    H --> J
    I --> J
    
    J --> K[build-properties-map]
    E --> K2[invalid-properties-set]
    
    K --> L{:properties<br/>:properties-text-values<br/>:page-refs<br/>:block-refs<br/>:invalid-properties}
```

## Journal Detection

```mermaid
flowchart TD
    A[file-name / page-title] --> B[match journal-pattern?]
    
    B -->|yes| C[journal-title->int]
    C --> D{valid-date?}
    D -->|yes| E[journal-day = YYYYMMDD]
    D -->|no| F[treat-as-regular-page]
    
    B -->|no| F
    
    E --> G{has-custom-format?}
    G -->|yes| H[convert-to-custom-format]
    G -->|no| I[use-default-format]
    
    H --> I
    I --> J[original-name<br/>normalized-name<br/>journal-day]
```

## Reference Resolution

```mermaid
sequenceDiagram
    participant AST as AST Node
    participant RefResolver as Reference Resolver
    participant DB as DB
    participant Pages as Page Entities

    RefResolver->>AST: get-page-reference(node)
    AST-->>RefResolver: page-ref string
    
    RefResolver->>RefResolver: block-ref/get-block-ref-id
    RefResolver->>DB: page-exists?(name)
    DB-->>RefResolver: page entity?
    
    alt page found
        RefResolver->>Pages: use existing
    else page not found
        RefResolver->>Pages: create placeholder
    end
    
    RefResolver-->>Output: {:block/name<br/>:block/uuid<br/>:block/title}
```

## Title Parsing Priority

```mermaid
flowchart TD
    A[Determine Page Title] --> B{title:: property?}
    B -->|exists| C[Use property value]
    B -->|not| D{file-name format?}
    
    D -->|tri-ll| E[parse file name]
    D -->|legacy| F[parse file name]
    
    E --> G{first-heading?}
    F --> G
    
    G -->|exists| H[Use heading content]
    G -->|not| I[Use file-name/body]
    
    C --> J[Final Title]
    H --> J
    I --> J
```

---

*Flowchart generado por Reversa Archaeologist*
