# Quilt — Technical Debt & Production Roadmap

> **Updated**: 2026-06-01 — Stack migration complete (Leptos→React, CM6→TipTap). Tauri removed per ADR-0005. See [AUDIT_REPORT.md](./AUDIT_REPORT.md) for full audit.
> **Document Status**: Draft for Review
> **Created**: 2026-05-07
> **Target**: Production Ready (MVP)
> **Stack**: Rust 2024, SQLite, React/TypeScript, MCP

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Technical Debt Inventory](#2-technical-debt-inventory)
3. [Feature Gaps](#3-feature-gaps)
4. [Implementation Phases](#4-implementation-phases)
5. [Detailed Roadmap](#5-detailed-roadmap)
6. [Milestones & Success Criteria](#6-milestones--success-criteria)
7. [Risks & Mitigations](#7-risks--mitigations)
8. [Appendix: File References](#8-appendix-file-references)

---

## 1. Executive Summary

### Current State Assessment

| Area | Status | Score | Notes |
|------|--------|-------|-------|
| **Domain Layer** | ✅ Complete | 95% | Entities, value objects, repositories well implemented |
| **Query Engine** | ⚠️ Functional | 80% | Works but has 3 self-cycle bugs in parser |
| **MCP Server** | ✅ Complete | 90% | 12 tools + 8 cognitive tools, needs integration |
| **Infrastructure** | ⚠️ Partial | 75% | SQLite works, event bus missing |
| **Sync Engine** | ✅ Implemented | 85% | LWW CRDT exists, not integrated |
| **Cognitive/AI** | ⚠️ Skeleton | 50% | Engines defined, needs AI provider |
| **UI (React/TypeScript)** | ✅ Complete | 90% | TipTap editor, CM6→TipTap migration done, real API via MCP |
| **Platform (Web)** | ✅ Complete | 90% | React SPA, Tauri removed per ADR-0005 |
| **CLI** | ✅ Complete | 90% | Binary works |

### Target State

Production-ready local-first PKM with:
- Full MCP integration for AI agents
- Working query DSL with FTS5 search
- Sync-ready architecture (CRDT in place)
- Functional web UI (React/TypeScript)
- Cognitive AI features (with external AI provider)

### Critical Path to Production

```
Phase 1 (Weeks 1-4)     Phase 2 (Weeks 5-8)      Phase 3 (Weeks 9-12)
├── Fix parser cycles     ├── Event Bus impl       ├── Cognitive integration
├── Event Bus skeleton   ├── MCP-Event integration ├── E2E testing
├── Clean warnings       ├── Sync integration      ├── Production hardening
├── File collision fix   └── Production polish      └── React maintenance
└── Integration tests
```

---

## 2. Technical Debt Inventory

### 2.1 CRITICAL Priority Issues

#### [CRITICAL-001] Parser Self-Cycles
- **Location**: `crates/quilt-query/src/parser.rs`
- **Lines**: 310, 554, 625
- **Issue**: Self-recursive calls causing potential stack overflow
- **CogniCode Violation**: `no_cycles` rule
- **Impact**: High — malformed queries may cause panics
- **Effort**: 2-4 hours
- **Status**: Unassigned

**Root Cause**: Recursive descent parser without proper cycle detection in:
- `parse_expr` line 310
- `parse_value` line 554
- `validate` line 625

**Fix Approach**:
```rust
// Add depth counter to prevent infinite recursion
const MAX_PARSE_DEPTH: u32 = 100;

fn parse_expr_impl(&self, input: &str, depth: u32) -> Result<QueryExpr, ParseError> {
    if depth > MAX_PARSE_DEPTH {
        return Err(ParseError::Syntax {
            msg: "Maximum expression depth exceeded".into(),
            line: 0,
            col: 0,
            hint: Some("Query too complex".into()),
        });
    }
    // ... rest of parser
}
```

---

#### [CRITICAL-002] Output Filename Collision
- **Location**: `crates/quilt-bin` and (removed) `crates/quilt-platform`
- **Issue**: Both produced `quilt` binary (Tauri crate removed per ADR-0005)
- **Impact**: Resolved — Tauri crate no longer exists
- **Effort**: Resolved by ADR-0005 (crate removal)
- **Status**: ✅ Resolved

---

#### [CRITICAL-003] Event Bus Not Implemented
- **Location**: Missing module
- **Issue**: No pub/sub system for internal events
- **Impact**: High — MCP notifications, hook system can't work properly
- **Effort**: 8-12 hours
- **Status**: Unassigned

**Required Components**:
1. `quilt-domain/src/events/event_bus.rs` — Tokio broadcast channel
2. `AppEvent` enum — All domain events
3. `EventHandler` trait — For processors
4. Integration with `McpServer::emit_*` methods

---

### 2.2 HIGH Priority Issues

#### [HIGH-001] Clippy Warnings (~24)
- **Effort**: 2-3 hours
- **Impact**: Code quality / potential bugs
- **Breakdown**:
  - 1 unused import in `classes/validator.rs`
  - 2 unused imports in `argument_cartographer/engine.rs` and `ai_providers.rs`
  - 4 unused variables in `ai_client.rs`
  - 2 unused variables in `parser.rs`
  - 2 unused variables in `executor.rs`
  - 3 clippy-specific warnings (too_many_arguments, should_implement_trait)
  - ~10 other warnings across the codebase

**Fix**: Run `cargo clippy --fix --allow-dirty` and manual review

---

#### [HIGH-002] Mock Bridge in UI
- **Location**: `crates/quilt-ui/src/bridge.rs:71-89` (Leptos, now removed)
- **Issue**: UI returned mock data instead of calling real backend
- **Impact**: Resolved — Leptos UI replaced by React/TypeScript with real MCP API calls
- **Effort**: Resolved by React migration
- **Status**: ✅ Resolved

---

#### [HIGH-003] Unused `#[allow(dead_code)]` in MCP Server
- **Location**: `crates/quilt-mcp/src/server.rs:229-244`
- **Issue**: Cognitive services marked `#[allow(dead_code)]` — not wired
- **Impact**: Cognitive engines are dead code
- **Effort**: 2-4 hours (wiring, not implementation)
- **Status**: Unassigned

---

### 2.3 MEDIUM Priority Issues

#### [MEDIUM-001] Documentation Gaps
- **Location**: Various
- **Issue**: Missing doc comments on some public APIs
- **Effort**: 2-3 hours
- **Status**: Unassigned

#### [MEDIUM-002] Test Coverage
- **Location**: Throughout
- **Issue**: Core domain tested, other areas lacking
- **Effort**: 8-12 hours for adequate coverage
- **Status**: Unassigned

#### [MEDIUM-003] Missing Soft-Delete Cleanup
- **Location**: `quilt-domain` (delete method in repositories)
- **Issue**: No background cleanup of deleted items
- **Effort**: 4-6 hours
- **Status**: Unassigned

---

### 2.4 LOW Priority Issues

#### [LOW-001] Workspace Manifest Warnings
- **Location**: `Cargo.toml`
- **Issue**: `unused manifest key: workspace.features`
- **Effort**: 15 minutes
- **Status**: Unassigned

---

## 3. Feature Gaps

### 3.1 MCP Server Integration Gaps

| Feature | Status | Priority | Notes |
|---------|--------|----------|-------|
| Tool execution wired to repositories | ⚠️ Partial | HIGH | Basic wired, needs full CRUD |
| Resource handlers | ⚠️ Partial | HIGH | Return mock data |
| Notification emission | ❌ Not wired | HIGH | Event bus missing |
| Hook system | ⚠️ Skeleton | MEDIUM | Registry exists, dispatcher not called |

**Required Work**:
1. Wire `McpServer` to real `SqliteBlockRepository` etc.
2. Implement actual resource handlers (not mock)
3. Integrate event bus for notification emission
4. Wire hook dispatcher into block/page operations

---

### 3.2 Cognitive AI Features

| Engine | Implementation | AI Provider | Status |
|--------|---------------|-------------|--------|
| CognitiveMirror | ✅ Engine exists | ❌ Not wired | 50% |
| SerendipityEngine | ✅ Engine exists | ❌ Not wired | 50% |
| AgentMemory | ⚠️ Store exists | ❌ No persistence | 40% |
| ArgumentCartographer | ✅ Engine exists | ❌ Not wired | 50% |
| MentalModelGardener | ✅ Engine exists | ❌ Not wired | 50% |
| CounterfactualExplorer | ✅ Engine exists | ❌ Not wired | 50% |
| KnowledgeEvolutionTracker | ✅ Engine exists | ❌ Not wired | 50% |
| MorningBriefing | ✅ Complete | ✅ Wired | 90% |

**Required for Production**:
1. Configure Ollama endpoint (local) or OpenAI API
2. Implement `AgentMemory` persistence
3. Wire AI client to engines

---

### 3.3 UI Feature Gaps (React/TypeScript)

> **Status**: React migration complete (Leptos→React, CM6→TipTap). Real API connected via MCP.

| Page/Component | Status | Notes |
|----------------|--------|-------|
| Journal Page | ✅ Complete | Working via MCP API |
| Page List | ✅ Complete | Working via MCP API |
| Search | ✅ Complete | FTS5 search via MCP |
| Query | ✅ Complete | Query DSL via MCP |
| Cognitive Dashboard | ⚠️ Partial | Backend exists, React UI not wired |
| Argument Map | ⚠️ Partial | Backend exists, React UI not wired |
| Mental Model | ⚠️ Partial | Backend exists, React UI not wired |
| Serendipity | ⚠️ Partial | Backend exists, React UI not wired |
| Outliner Block | ✅ Complete | TipTap-based outliner working |
| Block Editor | ✅ Complete | TipTap inline editing |
| Page Editor | ✅ Complete | Create/edit pages via MCP |
| Auth | ✅ Complete | JWT-based auth |
| E2E Tests | ✅ Complete | Playwright E2E suite for React |

---

### 3.4 Sync Integration Gaps

| Component | Status | Notes |
|-----------|--------|-------|
| CrdtSyncEngine | ✅ Implemented | LWW, version vectors |
| OfflineQueue | ⚠️ Skeleton | `offline.rs` exists, not integrated |
| Transport | ⚠️ Skeleton | `transport.rs` exists, mock only |
| Repository Integration | ❌ Missing | BlockRepo doesn't emit sync events |
| Sync UI | ❌ Missing | No sync status UI |

**Required for Sync**:
1. Emit sync events from BlockRepository operations
2. Implement actual transport (HTTP/WebSocket)
3. Add sync status to UI
4. Implement conflict resolution UI

---

## 4. Implementation Phases

### Phase 1: Foundation Fixes (Weeks 1-4)

**Goal**: Fix critical bugs, clean codebase, establish event system

#### 1.1 Week 1: Critical Bug Fixes

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Fix parser self-cycles | TBD | 4 | Parser passes cognicode quality |
| Fix filename collision | TBD | 0.5 | Unique binary names |
| Setup CI pipeline | TBD | 4 | Automated build + test |
| Create integration test suite | TBD | 8 | 80% coverage on core |

**Definition of Done**:
- `cargo build` succeeds without warnings
- `cargo test` passes 100%
- Parser handles malformed input without panic

---

#### 1.2 Week 2: Event Bus Implementation

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Define AppEvent enum | TBD | 2 | All domain events listed |
| Implement EventBus struct | TBD | 4 | Tokio broadcast channel |
| Implement BlockHandler | TBD | 4 | Process block events |
| Implement PageHandler | TBD | 4 | Process page events |
| Wire MCP to EventBus | TBD | 4 | Notifications work |

**API Sketch**:

```rust
// quilt-domain/src/events/event_bus.rs

pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }

    pub fn publish(&self, event: AppEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    BlockCreated { block_id: Uuid, page_id: Uuid },
    BlockUpdated { block_id: Uuid, changes: Vec<String> },
    BlockDeleted { block_id: Uuid, page_id: Uuid },
    BlockMoved { block_id: Uuid, from: Uuid, to: Uuid },
    PageCreated { page_id: Uuid, name: String },
    PageRenamed { page_id: Uuid, old: String, new: String },
    PageDeleted { page_id: Uuid },
    // ...
}
```

---

#### 1.3 Week 3: Code Quality Sprint

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Fix all clippy warnings | TBD | 3 | Zero warnings |
| Add missing doc comments | TBD | 2 | 100% public API documented |
| Implement soft-delete | TBD | 6 | Recycle bin functional |
| Implement orphan page detection | TBD | 4 | No orphaned pages |

---

#### 1.4 Week 4: Integration Foundation

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Wire MCP to repositories | TBD | 8 | Full CRUD via MCP |
| Wire MCP to search | TBD | 4 | FTS working via MCP |
| Add MCP integration tests | TBD | 8 | 90% MCP coverage |
| Document MCP API | TBD | 4 | OpenAPI-like spec |

---

### Phase 2: Core Feature Completion (Weeks 5-8)

**Goal**: Event bus, MCP integration, sync integration, production polish

#### 2.1 Week 5: Event Bus + MCP Integration

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Implement EventBus struct | TBD | 4 | Tokio broadcast channel |
| Wire MCP to EventBus | TBD | 4 | Real-time notifications via MCP |
| Wire MCP to repositories | TBD | 8 | Full CRUD via MCP |
| Wire MCP to search | TBD | 4 | FTS5 working via MCP |

---

#### 2.2 Week 6: Sync Integration

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Wire sync events to BlockRepo | TBD | 8 | Operations create sync changes |
| Implement transport layer | TBD | 8 | HTTP sync client |
| Sync status UI | TBD | 6 | Visual sync state |
| Conflict resolution UI | TBD | 6 | Manual resolution |

---

#### 2.3 Week 7: Query UI + Advanced Features

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Query results display | TBD | 4 | Table/tree view |
| Advanced FTS integration | TBD | 6 | Snippets, highlighting |
| Query history/saved queries | TBD | 4 | Persistence |
| Keyboard navigation | TBD | 4 | vim-like bindings |

---

#### 2.4 Week 8: Production Polish

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Performance testing | TBD | 8 | P95 < 100ms queries |
| Security audit | TBD | 8 | No vulnerabilities |
| Documentation finalization | TBD | 4 | README, API docs |
| MCP integration tests | TBD | 8 | 90% MCP coverage |

---

### Phase 3: Cognitive AI + Production Hardening (Weeks 9-12)

**Goal**: AI features functional, production-ready, tested

#### 3.1 Week 9: AI Provider Integration

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Configure AI provider (Ollama) | TBD | 8 | Local LLM working |
| Wire CognitiveMirror | TBD | 6 | Analysis works |
| Wire SerendipityEngine | TBD | 6 | Connection discovery |
| Wire AgentMemory persistence | TBD | 6 | Memory survives restart |

---

#### 3.2 Week 10: Advanced Cognitive Features

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Wire ArgumentCartographer | TBD | 6 | Argument mapping |
| Wire MentalModelGardener | TBD | 6 | Model tracking |
| Wire KnowledgeEvolutionTracker | TBD | 6 | Belief changes |
| MorningBriefing polish | TBD | 4 | Full briefing |

---

#### 3.3 Week 11: Cognitive UI + Agent Workflows

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Cognitive Dashboard (React) | TBD | 8 | Wire cognitive engines to UI |
| Argument Map UI | TBD | 6 | Visual argument mapping |
| Serendipity notifications | TBD | 4 | Real-time connection discovery |
| Agent Room UI | TBD | 8 | Multi-agent interaction |

---

#### 3.4 Week 12: Production Hardening

| Task | Owner | Hours | Deliverable |
|------|-------|-------|-------------|
| Performance testing | TBD | 8 | P95 < 100ms queries |
| Security audit | TBD | 8 | No vulnerabilities |
| E2E test coverage expansion | TBD | 8 | Additional scenarios |
| Documentation finalization | TBD | 4 | README, API docs |

---

## 5. Detailed Roadmap

```
2026 Timeline
                    Q2                    Q3
                  May  Jun  Jul  Aug  Sep  Oct
Phase 1: Foundation
  Week 1: Bugs      ████████
  Week 2: Events    ████████████████
  Week 3: Quality   ████████████████
  Week 4: Integration ████████████████

Phase 2: Core Features
  Week 5: Event Bus     ████████████████
  Week 6: Sync          ████████████████
  Week 7: Query UI      ████████████████
  Week 8: Polish        ████████████████

Phase 3: AI + Production
  Week 9: AI Provider  ████████████████
  Week 10: Cognitive   ████████████████
  Week 11: Sync        ████████████████
  Week 12: Hardening   ████████████████
```

---

### Milestone Checklist

#### M1: Codebase Stable (End of Week 3)
- [ ] Zero compiler warnings (~24 pre-existing as of 2026-06-01)
- [ ] Zero clippy warnings (~15 pre-existing as of 2026-06-01)
- [ ] Parser handles all valid/invalid input without panic
- [ ] Event bus functional
- [ ] 80% test coverage

#### M2: Core Functionality (End of Week 6)
- [x] MCP server fully wired to SQLite
- [x] Query DSL working end-to-end
- [x] FTS search working
- [x] UI displays real data (not mocks) via MCP
- [x] Journal pages functional

#### M3: Editor Complete (End of Week 8)
- [x] Outliner component working
- [x] Block edit-in-place working
- [x] Drag-and-drop reordering working
- [ ] React SPA fully wired to backend
- [ ] Cognitive dashboards integrated

#### M4: AI Features (End of Week 10)
- [ ] Ollama/OpenAI integration working
- [ ] CognitiveMirror returning real analysis
- [ ] SerendipityEngine finding connections
- [ ] MorningBriefing generating reports
- [ ] AgentMemory persisting

#### M5: Production Ready (End of Week 12)
- [ ] Sync functional (local-first)
- [ ] Conflict resolution UI
- [ ] Performance: P95 < 100ms
- [ ] Security audit passed
- [ ] E2E tests passing
- [ ] Release artifacts built

---

## 6. Milestones & Success Criteria

### Success Metrics

| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| Build Warnings | ~24 | 0 | `cargo build 2>&1` |
| Clippy Warnings | ~24 | 0 | `cargo clippy` |
| Test Coverage | ~40% | 80% | `cargo tarpaulin` |
| Parse Performance | Unknown | < 10ms | Benchmark |
| Query Performance | Unknown | < 100ms P95 | Load test |
| Binary Size | Unknown | < 50MB | `ls -lh` |
| Startup Time | Unknown | < 2s | Manual |
| React E2E Tests | ✅ Passing | — | Playwright suite |

### Definition of Done for Each Phase

**Phase 1 Done**:
```
cargo build --release  # Zero warnings
cargo test            # All pass
cargo clippy -- -D warnings  # Zero warnings
./target/release/quilt --version  # Works
```

**Phase 2 Done**:
```
MCP tools all return real data
Query "(task todo)" returns actual tasks
Search "test" returns matching blocks
Journal page shows today's entries
```

**Phase 3 Done**:
```
quilt --help  # Full CLI working
Cognitive tools return non-empty responses
Sync to local file works
Security audit passed
```

---

## 7. Risks & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Parser cycles cause runtime panic | Medium | High | Fix in Week 1, fuzz testing |
| UI complexity underestimated | High | High | Conservative 2x estimate |
| AI provider integration delays | Medium | Medium | Use Ollama (local) first |
| Sync scope creep | High | Medium | Focus on local-first first |
| React migration scope | Low | Low | Now complete (ADR-0006) |
| Web app performance | Medium | Medium | Bundle size, lazy loading |

---

## 8. Appendix: File References

### Critical Files to Modify

| File | Changes Needed |
|------|----------------|
| `crates/quilt-query/src/parser.rs` | Fix cycles, add depth limit |
| `crates/quilt-bin/Cargo.toml` | N/A - keep as is |
| `crates/quilt-domain/src/events/` | New module for EventBus |
| `crates/quilt-mcp/src/server.rs` | Wire cognitive services |
| `crates/quilt-ui/` (React) | Maintain React/TypeScript frontend |

### Files to Create

| File | Purpose |
|------|---------|
| `crates/quilt-domain/src/events/event_bus.rs` | EventBus implementation |
| `crates/quilt-domain/src/events/handlers/` | BlockHandler, PageHandler |

### Documentation to Update

| File | Update Needed |
|------|---------------|
| `AGENTS.md` | Update project status |
| `docs/reversa/*` | Archive completed docs |
| `README.md` | Update feature list |

---

## 9. Estimated Effort Summary

| Phase | Hours | Notes |
|-------|-------|-------|
| Phase 1 | 160 | 4 weeks x 40 hours |
| Phase 2 | 160 | 4 weeks x 40 hours |
| Phase 3 | 160 | 4 weeks x 40 hours |
| **Total** | **480** | **12 weeks** |

### Team Requirements (1 developer)

- Week 1-4: Backend focus (parser, events, tests)
- Week 5-8: Fullstack (event bus, sync, query UI)
- Week 9-12: AI + hardening

### Team Requirements (2 developers)

- Week 1-4: Parallel backend work
- Week 5-8: One UI, one backend
- Week 9-12: One AI integration, one hardening

---

*Document Version: 1.0*
*Next Review: After Phase 1 completion*
