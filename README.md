# Quilt — AI-first Knowledge Graph

> Rust reimplementation of the Logseq DB graph model with MCP-first architecture.
> **Status:** Pre-implementation — reverse engineering complete, design in progress.

## Documentation

All reverse engineering and design documents live in `docs/reversa/`:

| Document | Description |
|----------|-------------|
| `rust-reimplementation-proposal.md` | Full Rust reimplementation proposal |
| `rust-mcp-ai-deep-dive.md` | Architecture deep dive with code samples |
| `rust-properties-classes-petgraph-eval.md` | Properties + classes system + petgraph evaluation |
| `quilt-mcp-agent-capabilities.md` | 7 MCP agent capabilities design |
| `quilt-ui-workflows.md` | UI views, workflows, user experience |
| `QUILT_NAME.md` | Name decision rationale |
| `_reversa_sdd/code-analysis.md` | Reverse engineering code analysis |
| `_reversa_sdd/domain.md` | Domain glossary and business rules |
| `_reversa_sdd/state-machines.md` | 7 state machines |
| `_reversa_sdd/data-dictionary.md` | Data dictionary |
| `_reversa_sdd/architecture/` | C4 diagrams + ERD + Spec Impact Matrix |
| `_reversa_sdd/adrs/` | 7 Architecture Decision Records |
| `_reversa_sdd/sdd/` | 9 SDD formal specs |
| `_reversa_sdd/flowcharts/` | Mermaid flowcharts |
| `_reversa_sdd/user-stories/` | 3 user stories (64 scenarios) |
| `_reversa_sdd/traceability/` | Code/Spec matrix |
| `context/modules.json` | Structured module data |
| `context/surface.json` | Project surface data |

## Tech Stack (planned)

| Component | Technology |
|-----------|------------|
| Language | Rust |
| Database | SQLite + rkyv |
| Search | FTS5 |
| Sync | Loro (CRDT) |
| Desktop | Tauri |
| Agent Protocol | MCP |
