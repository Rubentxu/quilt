# ADR-007: Plugin System with Hooks API

> Architecture Decision Record - Logseq
> Generado por: reversa-detective
> Fecha: 2026-05-02

---

## Status

🟢 CONFIRMADO - Implementado y en producción

---

## Context

Quilt necesita extensibilidad sin comprometer la estabilidad del core. Un sistema de plugins permite a la comunidad agregar funcionalidad.

---

## Decision

Implementar **Plugin System con Hooks API controlada**:

**Available Hooks**:

| Hook | Payload | Description |
|------|---------|-------------|
| `hook:db-tx` | `{:blocks :tx-data}` | Intercept DB transactions |
| `hook:block-changes` | `{:blocks tx-data}` | Detect block changes |
| `search:rebuildPagesIndice` | `{}` | Rebuild pages index |
| `search:rebuildBlocksIndice` | `{}` | Rebuild blocks index |

**Plugin manifest** (`package.json`):
```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "api-version": "1.0.0",
  "description": "My plugin"
}
```

---

## Evidence (Git Log)

```
enhance(plugins): custom block renderer
fix: add mldoc dependency for publish tests
fix(publish): acquire DO stubs in sync let to avoid RpcPromise clone
```

**From code**:
```clojure
;; src/main/logseq/api/plugin.cljs
;; Plugin API es primariamente para el desktop app
;; Hooks: hook:db-tx, hook:block-changes
```

---

## Plugin Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│   Logseq Core                                                        │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │  Hook System                                                 │   │
│   │  ┌───────────┐  ┌───────────┐  ┌───────────┐                │   │
│   │  │ hook:db-tx│  │hook:block │  │search:    │  ...           │   │
│   │  │           │  │:changes   │  │rebuild*   │                │   │
│   │  └───────────┘  └───────────┘  └───────────┘                │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                              │                                      │
│                              │ Plugin calls                         │
│                              ▼                                      │
│                    ┌─────────────────┐                             │
│                    │   Plugins       │                             │
│                    │  ┌───────────┐  │                             │
│                    │  │ Plugin A  │  │                             │
│                    │  │ Plugin B  │  │                             │
│                    │  │ Plugin C  │  │                             │
│                    │  └───────────┘  │                             │
│                    └─────────────────┘                             │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Capabilities vs Isolation

| Capability | Status | Notes |
|------------|--------|-------|
| DB Transaction hooks | 🟢 Available | Read/Intercept |
| Block change detection | 🟢 Available | Read only |
| Custom block renderers | 🟢 Available | UI extension |
| Search index rebuild | 🟢 Available | Admin |
| Direct filesystem access | 🔴 Blocked | Security |
| Network access | 🟡 Limited | Via API only |

---

## Consequences

**Positive**:
- ✅ Extensibilidad controlada
- ✅ Security por diseño (hooks only)
- ✅ Comunidad puede contribuir

**Negative**:
- ❌ API limitada a hooks predefinidos
- ❌ No full plugin isolation (same process)

---

## Related Decisions

| Decision | Status |
|----------|--------|
| Custom block renderers | 🟢 CONFIRMADO |
| Search index plugin API | 🟢 CONFIRMADO |
| Plugin marketplace | 🟡 INFERIDO |

---

*Documento generado automáticamente por Reversa Detective*
