# ADR-005: Logseq CLI

> Architecture Decision Record - Logseq
> Generado por: reversa-detective
> Fecha: 2026-05-02

---

## Status

🟢 CONFIRMADO - Implementado recentemente (PR #12340)

---

## Context

Quilt historically ha sido una aplicación GUI (Electron, Web). Usuarios avanzados y developers necesitan acceso programático via línea de comandos.

**Requerimientos identificados**:
- Automatización de tareas (scripts)
- CI/CD integration
- Database queries
- Graph management sin GUI

---

## Decision

Implementar **CLI completa con subcomandos**:

```bash
logseq-cli graph list
logseq-cli graph create
logseq-cli graph delete
logseq-cli sync status
logseq-cli sync upload
logseq-cli sync download
logseq-cli query "..."
logseq-cli task status
```

**Principales subcomandos**:

| Category | Commands |
|----------|----------|
| Graph | `list`, `create`, `delete`, `open` |
| Sync | `status`, `upload`, `download` |
| Query | `query`, `search` |
| Task | `task status`, `task list` |
| Page | `page create`, `page delete` |

---

## Evidence (Git Log)

```
feat: init full-featured Logseq CLI (#12340)
fix(cli): resolve packaged db-worker runtime path
fix(cli,db-worker): not keep empty new graph when sync download failed
fix(cli): sync status fails with unactionable e2ee-password-not-found error
fix(cli): hint not showing up for sync subcommands
fix(cli): config.edn reading and writing incorrectly
fix(cli-e2e): dont delete skill.md in e2e cleanup
enhance(cli): default list order to desc
enhance(cli-e2e): add jobs-based(default=4) parallel case execution
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│   logseq-cli                                                         │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │  CLI Entry Point                                            │   │
│   │  ┌───────────┐  ┌───────────┐  ┌───────────┐               │   │
│   │  │  Graph    │  │   Sync    │  │  Query    │  ...            │   │
│   │  │  Commands │  │  Commands │  │  Commands │               │   │
│   │  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘               │   │
│   │        │              │              │                       │   │
│   │        └──────────────┼──────────────┘                       │   │
│   │                       │                                      │   │
│   │              ┌────────┴────────┐                            │   │
│   │              │  DB Worker API  │                            │   │
│   │              └────────┬────────┘                            │   │
│   │                       │                                      │   │
│   └───────────────────────┼──────────────────────────────────────┘   │
│                           │                                          │
│                           ▼                                          │
│                    ┌─────────────┐                                  │
│                    │ DataScript  │                                  │
│                    │   Graph     │                                  │
│                    └─────────────┘                                  │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## CLI Auth Model

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│   Auth Flow                                                          │
│   ┌───────────┐    ┌───────────┐    ┌───────────┐                  │
│   │  Login    │───►│  Token    │───►│  Store    │                  │
│   │  OAuth     │    │  Refresh  │    │  Secure   │                  │
│   └───────────┘    └───────────┘    └───────────┘                  │
│                                                                      │
│   - Tokens stored in ~/.logseq/auth.json                            │
│   - Support for OAuth refresh tokens                                │
│   - E2EE password separate from auth                                │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Consequences

**Positive**:
- ✅ Automatización posible
- ✅ CI/CD integration
- ✅ Developer experience mejorada
- ✅ Database queries sin GUI

**Negative**:
- ❌ Maintenance de CLI interface
- ❌ Breaking changes en CLI son user-facing

---

## Related Decisions

| Decision | Status |
|----------|--------|
| DB Worker separate process | 🟢 CONFIRMADO |
| Auth tokens secure storage | 🟢 CONFIRMADO |
| E2EE separate from auth | 🟡 INFERIDO |

---

*Documento generado automáticamente por Reversa Detective*
