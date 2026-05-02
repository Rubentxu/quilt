# Quilt — Project Name Decision

**Date:** 2026-05-02
**Decision:** The Rust reimplementation of the logseq DB graph model will be called **Quilt**.

## Rationale

| Factor | Evaluation |
|--------|-----------|
| **Length** | 5 letters, easy to type |
| **Pronunciation** | Universal across languages |
| **Metaphor** | Knowledge as patches of cloth woven together by AI threads |
| **Trademark risk** | No known tech product with this name |
| **Domain** | `quilt.app`, `quilt.so`, `getquilt.com` — likely available |
| **Tagline** | "Quilt your knowledge with AI" |
| **Visual identity** | Patchwork pattern = knowledge graph visualization |

## Alternatives Considered

| Name | Rejected Because |
|------|-----------------|
| Loom | loom.com (Atlassian, $975M) + Lucasfilm game |
| Nexus | Overused, trademark conflicts |
| Node | Node.js conflict |
| Vault | HashiCorp Vault conflict |
| Warp | Warp terminal conflict |
| Mesh | Generic/overused in tech |

## Origin

Quilt inherits the **Logseq DB graph** model (typed properties, classes, closed values)
but is a complete ground-up reimplementation in Rust with MCP-first architecture.

## References in this repo

All generated Reversa documents now reference "Quilt" as the project name.
Original logseq source code paths remain unchanged (they document the reference system).
