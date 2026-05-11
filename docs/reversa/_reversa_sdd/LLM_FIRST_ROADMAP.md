# LLM-First Gap Analysis & Implementation Roadmap

**Project:** Quilt — AI-First Knowledge Graph
**Date:** 2026-05-06
**Analyst:** SDD Explorer Agent
**Mode:** Strategic Gap Analysis for LLM-First Prioritization

---

## Executive Summary

Quilt's MCP server is functional but **incomplete for AI agents**. The critical issue is that all 6 cognitive engines (CognitiveMirror, SerendipityEngine, AgentMemory, ArgumentCartographer, MentalModelGardener, CounterfactualExplorer) exist as fully-implemented Rust crates but are **never wired into the MCP server** — they sit in `quilt_cognitive::` as optional dependencies with `None` values. This is the #1 blocker preventing AI agents from delivering the "thinking companion" experience that differentiates Quilt.

Secondary gaps include a **sync strategy mismatch** (spec says Loro CRDT, impl uses custom LWW), **missing resilience** in search, and **missing MCP Resources** for the `logseq://cognitive/*` hierarchy.

---

## TIER 0 — CRITICAL (Do Now)

These features are prerequisites for AI agents to function at all. Without them, Quilt is just a searchable notes app.

### Feature: Wire Cognitive Engines to MCP Server

- **LLM Rationale**: This is the entire value proposition. The cognitive engines provide:
  - `CognitiveMirror`: Analyzes your thinking patterns, reveals knowledge gaps
  - `SerendipityEngine`: Finds unexpected connections you missed
  - `ArgumentCartographer`: Structures debates from raw notes
  - `MentalModelGardener`: Evolves your mental models over time
  - `CounterfactualExplorer`: Explores "what if" scenarios
  - `KnowledgeEvolutionTracker`: Shows how your beliefs changed
  Without wiring, AI agents only get raw CRUD operations, not "thinking with you".

- **Effort**: **Low-Medium** (mostly configuration, not implementation)
  - Engines already exist in `quilt-cognitive/src/`
  - Server already has `with_cognitive()` method
  - Just need to call it in the server initialization

- **Security**: No concerns — engines read from local DB, no network exposure

- **Performance**: 
  - CognitiveMirror: O(n) graph traversal, cacheable
  - SerendipityEngine: Background task, runs periodically
  - Others: On-demand, async

- **Dependencies**: 
  - Requires: `quilt-cognitive` crate compiled
  - Blocks: All cognitive tools and resources listed below

- **Implementation Notes**:
  1. Find where `McpServer::new()` is called
  2. Add `.with_cognitive(Some(cognitive_mirror), Some(serendipity_engine), ...)`
  3. Remove `#[allow(dead_code)]` attributes once wired
  4. Add integration tests verifying cognitive tools appear in `tools/list`

---

### Feature: Implement `logseq://cognitive/*` MCP Resources

- **LLM Rationale**: AI agents use MCP Resources to understand the knowledge graph state. `logseq://cognitive/*` resources allow agents to:
  - Subscribe to cognitive map updates
  - Read serendipity discoveries as a resource
  - Access argument graphs without calling a tool
  - These are defined in spec but not implemented

- **Effort**: **Low**
  - Resources already conditionally added to `resources()` list (lines 654-689)
  - Handler methods (`resource_cognitive_map()`, etc.) already exist
  - Just need cognitive engines wired first

- **Security**: Read-only resources, no data modification

- **Performance**: Same as cognitive engines — cached, async

- **Dependencies**: **Requires cognitive engine wiring first**

- **Implementation Notes**:
  1. After wiring cognitive engines, verify resources appear in `resources/list`
  2. Add subscription support if not present (for real-time updates)

---

### Feature: Implement `logseq://` Deep Link Handler

- **LLM Rationale**: Users and AI agents need to share links to specific knowledge. Deep links enable:
  - "Open page X" from external apps
  - AI agents sharing links to specific blocks or pages
  - Integration with other tools via URL schemes

- **Effort**: **Low-Medium**
  - URL parsing already exists in `JournalDay::from_str()`
  - Need platform-specific handling (Tauri commands for `logseq://`)

- **Security**: Validate URL params, sanitize graph names, verify page exists before navigation

- **Performance**: No impact — single page lookup

- **Dependencies**: None blocking

- **Implementation Notes**:
  1. Add Tauri command handler for `logseq://` URL scheme
  2. Parse URI: `logseq://pages/{name}`, `logseq://journal/{date}`, `logseq://blocks/{id}`
  3. Navigate to appropriate view or return page data

---

## TIER 1 — HIGH (Next Sprint)

Major utility gains for AI agents and system reliability.

### Feature: File System Watcher (External Change Detection)

- **LLM Rationale**: AI agents need to know when external tools modify the graph. If user edits in another app, agent should:
  - Detect the change
  - Notify via MCP `notifications/block_changed`
  - Re-index search accordingly

- **Effort**: **Medium**
  - `notify` crate is already in Cargo.toml (line 1305)
  - Need to integrate with existing event system

- **Security**: No concerns — watches local graph directory only

- **Performance**: 
  - Uses OS-level file watching (inotify on Linux)
  - Debounce rapid changes to avoid event storms

- **Dependencies**: Event system already exists (`AppEvent::FileChanged`)

- **Implementation Notes**:
  1. Create `FileWatcher` service using `notify` crate
  2. Subscribe to `AppEvent::FileChanged` events
  3. Publish `BlockChanged` notifications when external edits detected
  4. Trigger incremental FTS re-index

---

### Feature: Search Retry Policy with Resilience

- **LLM Rationale**: AI agents depend on search working reliably. Without retry:
  - Transient failures (DB lock, index rebuild) cause agent failures
  - Agents can't trust search results
  - User experience degrades

- **Effort**: **Low**
  - `backoff` crate already in Cargo.toml (line 1302)
  - Wrap search calls with exponential backoff

- **Security**: No concerns

- **Performance**: Improves perceived reliability, minimal overhead

- **Dependencies**: None blocking

- **Implementation Notes**:
  1. Wrap `SearchService::search()` with retry logic
  2. Use exponential backoff: 100ms, 200ms, 400ms, max 3 retries
  3. Log failures for debugging
  4. Return partial results if some shards fail (graceful degradation)

---

### Feature: Implement Morning Briefing Dashboard

- **LLM Rationale**: This is the **day 1 experience** for users. The briefing:
  - Shows AI agent is alive and working
  - Provides cognitive pulse (what user thought about recently)
  - Surfaces serendipity discoveries
  - Highlights knowledge decay warnings
  Per quilt-ui-workflows.md, this is the "3 minute morning ritual"

- **Effort**: **Medium** (requires UI + cognitive engines)
  - Backend: Generate briefing from cognitive engines
  - Frontend: Dashboard view in Tauri/Leptos

- **Security**: No concerns — generates from local data

- **Performance**: Pre-computed at night, served from cache in morning

- **Dependencies**: Requires cognitive engines wired + background task scheduler

- **Implementation Notes**:
  1. Create `MorningBriefingService` that runs as nightly background task
  2. Aggregates: recent pages, cognitive pulse, serendipity findings, decay alerts
  3. Stores compiled briefing as special journal page or cached JSON
  4. UI renders briefing at journal open

---

### Feature: Sync Strategy Clarification (LWW vs CRDT)

- **LLM Rationale**: AI agents need predictable sync behavior. Ambiguity causes:
  - Conflicting expectations about conflict resolution
  - Potential data loss in multi-device scenarios

- **Effort**: **Low** (architectural decision, then doc)
  - Spec says "CRDT con Loro" (loro crate dependency)
  - Impl uses custom LWW in `quilt-sync/src/crdt.rs`
  - Decision: Either adopt Loro or clarify LWW is intentional

- **Security**: N/A (architectural)

- **Performance**: LWW is simpler/faster; Loro handles more complex merge scenarios

- **Dependencies**: None blocking, but affects future sync architecture

- **Implementation Notes**:
  1. **Option A** (LWW is correct): Remove Loro from spec, document LWW strategy
  2. **Option B** (Need true CRDT): Integrate Loro crate per spec design
  3. Document decision in architecture decision record

---

## TIER 2 — MEDIUM (Later)

Important features that enhance the experience but aren't blockers.

### Feature: Weekly Review Automation

- **LLM Rationale**: Automated review keeps knowledge fresh. AI agents can:
  - Generate weekly cognitive summary
  - Identify stale notes requiring updates
  - Surface evolution of mental models
  - Suggest focus areas for next week

- **Effort**: **Medium**
  - Depends on cognitive engines wired
  - Needs background scheduler

- **Dependencies**: Cognitive engine wiring (TIER 0)

---

### Feature: Agent Room UI (Multi-Agent Debate)

- **LLM Rationale**: The **defining feature** of Quilt vs other PKMs. Six agents debate using your own knowledge graph as evidence. This is the "thinking companion" experience.

- **Effort**: **High**
  - Complex UI: 6 agent panels + synthesis panel
  - Agent orchestration for debate flow
  - Evidence gathering from graph

- **Dependencies**: 
  - Cognitive engines wired
  - ArgumentCartographer implemented
  - Background task scheduler

- **Security**: AI agents read from local graph only — no data exfiltration

---

### Feature: Cognitive Map Visualization

- **LLM Rationale**: Visual representation of knowledge structure. Users see:
  - Clusters (dense knowledge areas)
  - Frontiers (superficial mentions)
  - Gaps (topics avoided)
  - Evolution over time

- **Effort**: **High**
  - Graph visualization (2D/3D)
  - CognitiveMirror engine integration
  - Interactive exploration

- **Dependencies**: CognitiveMirror wired first

---

## TIER 3 — LOW (Future)

Complex features that are important but require significant work.

### Feature: Presence/Collaboration (Real-time Cursors)

- **LLM Rationale**: Real-time collaboration shows who else is working in the graph. Useful for:
  - Team knowledge bases
  - Mentor/mentee scenarios
  - Collaborative research

- **Effort**: **Very High**
  - WebSocket infrastructure for real-time
  - Cursor position sharing
  - Conflict-free block editing (need true CRDT)
  - Presence indicators

- **Dependencies**: True CRDT sync (if not using LWW)

---

### Feature: E2EE Encryption

- **LLM Rationale**: Sensitive knowledge graphs need end-to-end encryption. Server (if cloud) never sees plaintext.

- **Effort**: **Very High**
  - Key management
  - Cryptographic operations on every read/write
  - Key rotation
  - Performance impact

- **Dependencies**: After sync architecture stabilized

---

### Feature: Onboarding Flow

- **LLM Rationale**: First impression matters. Onboarding:
  - Sets up cognitive seed
  - Imports existing notes
  - Establishes initial mental models
  - Teaches basic interactions

- **Effort**: **Medium**
  - UI wizard flow
  - Import adapters (Obsidian, Markdown)
  - Initial cognitive analysis

- **Dependencies**: Basic MCP tools working

---

## QUICK WINS (< 1 day each)

These require minimal effort but provide immediate value:

1. **Wire cognitive_mirror only** — The simplest cognitive engine. Just add `.with_cognitive(cognitive_mirror.clone(), None, None, None, None, None, None)` and cognitive mirror tool appears.

2. **Add search retry** — 10 lines of code with `backoff` crate already in dependencies.

3. **Document the LWW/CRDT decision** — Write an ADR clarifying sync strategy. 2 hours.

4. **Enable `logseq://` resource handlers** — Handler methods exist but never execute because engines are None. Wire engines = resources work.

5. **Add health check for FTS index** — `logseq_index_health` tool already exists, just needs testing.

---

## BLOCKERS & DEPENDENCIES GRAPH

```
TIER 0 (Must do first):
├── Wire Cognitive Engines ──────────┬──┐
│                                     │  │
│  ├─ CognitiveMirror ──────────────>│  │
│  ├─ SerendipityEngine ────────────>│  │
│  ├─ AgentMemory ─────────────────>│  │
│  ├─ ArgumentCartographer ────────>│  │
│  ├─ MentalModelGardener ────────>│  │
│  └─ KnowledgeEvolutionTracker ───>│  │
│                                     │  │
│  After wiring, enables:            │  │
│  ├─ logseq_cognitive_mirror tool ──┘  │
│  ├─ logseq_serendipity tool ──────────|
│  ├─ logseq://cognitive/* resources ───┤
│  ├─ Morning Briefing (needs scheduler) ├
│  └─ Weekly Review ────────────────────┘
│
├── Deep Link Handler ────────────────────┐
│  (independent of cognitive)            │
│  Enables:                              │
│  ├─ External apps can open Quilt ───────┘
│  └─ AI agents share links ──────────────┘
│
└── Search Retry ─────────────────────────┐
   (independent)                          │
   Enables:                               │
   └─ Resilient search for AI agents ─────┘

TIER 1 (After TIER 0):
├── File System Watcher ──────────────────┐
│  Needs: Event system (exists)           │
│  Enables: External change detection ────┘
│
├── Morning Briefing ──────────────────────┐
│  Needs: Cognitive engines + scheduler   │
│  Enables: Day 1 experience ─────────────┘
│
└── Sync Decision ─────────────────────────┐
    (architectural, independent)          │
    Enables: Clear spec/implementation ─────┘

TIER 2 (Requires TIER 0):
├── Weekly Review ────────────────────────┐
│  Needs: Cognitive engines              │
└──┐
│
├── Agent Room ───────────────────────────┐
│  Needs: All cognitive engines + UI      │
└──┘
│
└── Cognitive Map Visualization ────────────┐
    Needs: CognitiveMirror + graph lib     │
    └──────────────────────────────────────┘

TIER 3 (Future):
├── Presence/Collaboration ───────────────┐
│  Needs: True CRDT (if LWW retained)     │
└──┘
│
└── E2EE Encryption ──────────────────────┐
    Needs: Stable sync + key management   │
    └──────────────────────────────────────┘
```

---

## RECOMMENDED IMPLEMENTATION ORDER

### 1. First: Wire Cognitive Engines (2-4 hours)
**Reason**: This is the #1 value prop. AI agents currently can't do anything cognitive. This single change enables all cognitive tools and resources.

**Verification**: After wiring, `tools/list` should show 19+ tools instead of 13.

### 2. Second: Search Retry Policy (1 hour)
**Reason**: AI agents fail on transient search errors. Quick win that improves reliability immediately.

### 3. Third: Deep Link Handler (4-8 hours)
**Reason**: Independent of cognitive engines. Enables external integration. Useful for testing and user workflows.

### 4. Fourth: File System Watcher (1-2 days)
**Reason**: External change detection is foundational for multi-tool workflows. Agents need to know when the graph changes.

### 5. Fifth: Morning Briefing (2-3 days)
**Reason**: This is the day-1 user experience. Shows Quilt is alive and thinking with you. Requires cognitive engines + UI work.

### 6. Sixth: Sync Decision + Documentation (2 hours)
**Reason**: Clarify LWW vs CRDT before building more sync infrastructure. Prevents costly rewrites later.

### 7. Seventh: Weekly Review (2-3 days)
**Reason**: Complements morning briefing. Depends on same cognitive infrastructure.

---

## SECURITY CONSIDERATIONS

| Feature | Security Notes |
|---------|---------------|
| **Cognitive Engines** | Read-only analysis, no data exfiltration, no network calls |
| **Deep Links** | Validate all URL params, sanitize graph/page names, check existence before navigation |
| **File Watcher** | Only watches configured graph directory, no recursive symlink traversal |
| **Search** | FTS5 queries are sandboxed to graph DB |
| **Sync (LWW)** | Custom LWW may have timestamp manipulation risk — need clock sync |
| **Sync (CRDT)** | True CRDT has better conflict safety but more complex |
| **E2EE** | Future work — must not store keys in DB, need HSM or KDF |

---

## Entropy Analysis (Connascence Landscape)

**Method**: CogniCode + Heuristic

| Component A | Component B | Connascence Type | I(bits) | Severity |
|------------|-------------|------------------|---------|----------|
| `McpServer` | `CognitiveMirror` | Name (optional wiring) | 0.0 | ✅ OK |
| `McpServer` | `quilt-sync/crdt` | Meaning (LWW vs CRDT spec) | 4.17 | 🔴 CRITICAL |
| `SearchService` | `McpServer` | Name (retry missing) | 0.58 | ✅ OK |
| `FileWatcher` | `AppEvent::FileChanged` | Position (event coupling) | 1.0 | ⚠️ MEDIUM |
| `MorningBriefing` | `CognitiveMirror` | Meaning (aggregation) | 2.0 | ⚠️ MEDIUM |
| `AgentRoom` | `ArgumentCartographer` | Meaning (debate orchestration) | 3.0 | ⚠️ HIGH |

**Critical Pairs (I > 3.0 bits)**: 
- `quilt-sync` impl vs spec (LWW vs CRDT) — **unresolved architectural mismatch**

**Hidden Connascence (Meaning/Timing)**:
- Cognitive engines initialized but not wired: `cognitive_mirror: None` default implies silent failure
- Search retry: no explicit policy = unpredictable failures under load

**Coupling Score**: H_external = 4.2 (High — cognitive engines are tightly coupled to server via Option type)

**Estimation Method**: CogniCode call graph + Heuristic

**Confidence**: estimated (quantitative not available without runtime trace)

---

## Ready for Proposal

**Yes** — This analysis provides a clear implementation roadmap.

### Summary for Orchestrator:

1. **Immediate action**: Wire cognitive engines to MCP server. This is 2-4 hours and unlocks the entire cognitive layer.

2. **Quick wins**: Search retry (1hr), deep links (4-8hrs), LWW/CRDT decision (2hrs)

3. **Next sprint**: File watcher + Morning Briefing + Weekly Review

4. **Future**: Agent Room, Cognitive Map, Presence, E2EE

The most critical insight: **all cognitive engines exist but none are wired**. This is the #1 blocker for LLM-first utility. Everything else is secondary.