# ADR-0010: Testing Strategy — unit, integration, E2E y calidad

Status: accepted

Quilt aplica una estrategia de testing en 3 capas: unit tests en dominio (mínimo 80% cobertura), integration tests en aplicación (flujos keystroke→intent→operación), y E2E Playwright en browser (smoke + regresión). Cobertura WASM se asegura compilando todos los tests con target `wasm32-unknown-unknown`. Los gates de CI bloquean merge si coverage baja, tests fallan, o clippy emite warnings.

## Decision

### 1. Unit Tests (domain-first)

- **Qué**: métodos puros del dominio: `Block`, `Page`, `RefIndex`, `InlineParser`, `HistoryStack`, `OutlinerCommand::invert`, validaciones de `OutlinerOperations`.
- **Dónde**: `#[cfg(test)] mod tests` en cada archivo de `quilt-domain`.
- **Coverage mínima**: 80% de líneas en `quilt-domain`.
- **WASM**: todos los tests de dominio deben compilar para `wasm32-unknown-unknown`. Si un test usa el filesystem o red, se feature-gatea con `#[cfg(not(target_arch = "wasm32"))]`.

### 2. Integration Tests (application layer)

- **Qué**: flujos completos: keystroke → `InputHandler::dispatch` → `OutlinerIntent` → `PageOutliner::transact` → `OutlinerEvent` → señal Leptos.
- **Dónde**: `crates/quilt-ui/tests/integration/` o `tests/` a nivel workspace.
- **Mock**: `MockRefRepository`, `MockBlockRepository` implementan los traits del dominio. Sin SQLite en tests de integración.
- **Cuantos**: al menos 1 test por interacción de la tabla canónica de keyboard shortcuts (MUST).

### 3. E2E Tests (browser)

- **Tool**: Playwright (`e2e/`).
- **Qué**: smoke tests (carga, render, interacción básica) + regresión visual (snapshots).
- **Gate**: smoke debe pasar antes de merge. Los snapshot tests pueden ser advisory (no bloquean merge).
- **CI**: GitHub Actions con `cargo-leptos build` y `playwright test`.

### 4. Snapshot Tests (visual regression)

- **Qué**: render de bloques con distintas decoraciones (refs, tags, properties, highlight, code blocks).
- **Tool**: Playwright `toMatchSnapshot()` o insta para Rust.
- **Gate**: advisory — no bloquean merge, pero se reportan como warning.

### Coverage Gate

| Layer | Target | Gate |
|-------|--------|------|
| `quilt-domain` | ≥80% lines | Blocker |
| `quilt-application` | ≥60% lines | Warning |
| `quilt-ui` (parser, outliner) | ≥70% lines | Warning |
| `quilt-infrastructure` | ≥50% lines | Advisory |
| E2E smoke | 100% pass | Blocker |

### WASM Testing

- `quilt-domain` tests compilan con `cargo test --target wasm32-unknown-unknown` (feature-gated donde necesario).
- `quilt-ui` tests corren en browser via `wasm-bindgen-test` o `wasm-pack test --headless`.
- Los tests de parser, history, y `RefIndex` son puros y no necesitan browser.

## Considered Options

1. **Solo unit tests** — rejected: sin integración, keystroke→intent→operación quedaría sin probar.
2. **Solo E2E** — rejected: lento, frágil, difícil de debugear en CI.
3. **Unit + integration + E2E con gates** — accepted: progresivo, cada capa protege distinto.
4. **No WASM testing** — rejected: bugs de WASM (memoria, serialización) son silenciosos y frecuentes.

## Consequences

- CI gate: `cargo test --workspace` + `cargo test --target wasm32-unknown-unknown -p quilt-domain` + `cargo clippy -- -D warnings` + `playwright test`
- Los tests de dominio deben usar `MockRefRepository` (no SQLite).
- Los tests de parser deben cubrir cada tipo de inline: `[[ ]]`, `(( ))`, `#tag`, `property:: value`, `^^highlight^^`, `**bold**`, `*italic*`, `~~strikethrough~~`, `` `code` ``.
- Los tests de history deben cubrir undo/redo de: `SetContent`, `CreateBlock`, `DeleteBlock`, `Indent`, `Outdent`, `Split`, `Merge`, `SetProperty`, `CycleStatus`, `AddRef`, `RemoveRef`.
- Los tests de referencia deben cubrir: backlinks O(1), forward refs, sync refs, rebuild from repo.
