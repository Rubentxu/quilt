# ADR-006: DataScript as Primary Database

> Architecture Decision Record - Logseq
> Generado por: reversa-detective
> Fecha: 2026-05-02

---

## Status

🟢 CONFIRMADO - Implementado y en producción

---

## Context

Quilt necesita almacenar datos estructurados jerárquicos (bloques, páginas, referencias) con capacidad de queries complejas.

**Alternativas consideradas**:
- SQLite: Más común pero menos flexible para grafos
- PostgreSQL: Robusto pero overkill para local-first
- Datascript: Datomic-like, ideal para grafos

---

## Decision

Usar **DataScript** como base de datos local:

1. **Schema flexible**: Entidades con atributos arbitrary
2. **Datalog queries**: Queries recursivas para grafos
3. **In-memory**: Rápido para aplicaciones cliente
4. **Datomic-inspired**: Schema, transactions, history

**Schema principal**:
```clojure
{:block/uuid           {:db/unique :unique}
 :block/name           {:db/unique :unique}
 :file/path            {:db/unique :unique}

 :block/page           {:db/cardinality :one
                        :db/valueType :ref}
 :block/parent         {:db/cardinality :one
                        :db/valueType :ref}
 :block/_parent        {:db/cardinality :many
                        :db/valueType :ref}
 :block/refs           {:db/cardinality :many
                        :db/valueType :ref}}
```

---

## Evidence (From code-analysis.md)

```
frontend/db
├── conn.cljs         ; Conexión
├── model.cljs        ; Funciones de consulta
├── transact.cljs      ; Transacciones async
├── query_dsl.cljs    ; DSL queries
├── query_custom.cljs ; Custom queries
├── persist.cljs      ; Persistencia
└── restore.cljs      ; Restauración
```

---

## Query DSL

```clojure
;; Operadores booleanos
(and) (or) (not)

;; Filtros
(between start end)
(property key value)
(task marker*)
(priority level*)
(page "Page Name")
[[page-ref]]
(full-text-search "text")
```

---

## Consequences

**Positive**:
- ✅ Queries poderosas para grafos
- ✅ Schema flexible
- ✅ Transacciones atómicas
- ✅ History/Undo built-in

**Negative**:
- ❌ Datascript es in-memory (requiere persist separately)
- ❌ Menos ecosistema que SQLite/Postgres
- ❌ Learning curve para Datalog

---

## Persistence Strategy

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│   DataScript (In-Memory)                                            │
│          │                                                           │
│          │ persist to disk                                           │
│          ▼                                                           │
│   ┌─────────────┐    ┌─────────────┐                               │
│   │   SQLite   │ or │  Transit    │                               │
│   │  (desktop) │    │   (files)   │                               │
│   └─────────────┘    └─────────────┘                               │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

*Documento generado automáticamente por Reversa Detective*
