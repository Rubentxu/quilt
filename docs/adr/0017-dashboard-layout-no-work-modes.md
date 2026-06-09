# ADR: DashboardLayout en lugar de Work Mode Switcher

Status: implemented

## Context

El documento de investigación `docs/research/ux-workflow-portfolio-analysis.md` propuso un "Work Mode Switcher" como alternativa a las rutas planas actuales del SPA. La idea lateral sugería modos (Write, Review, Structure, Explore, Automate) que eligen superficies y paneles automáticamente.

El auto-grill (Q001-P1 + Q008-P2, sesión 2026-06-07) rechazó Work Modes por tres razones:

1. **Bundling**: la propuesta mezclaba 3 decisiones independientes (panel visibility, feature gating, keyboard shortcuts)
2. **ADR-0002**: la UI debe ser Logseq-like con paneles integrados, no vistas separadas. "Work Mode" introduce un concepto de navegación que no existe en Logseq.
3. **Colisión con InputMode**: `InputMode` ya está definido en `docs/quilt-keyboard-shortcuts.md` §1.1 como modo de input del editor (normal/insert). "Work Mode" sobrecargaría el término.

## Decision

**No se implementa Work Mode Switcher.** En su lugar:

1. **DashboardLayout**: presets de paneles persistibles a nivel workspace. Define qué paneles son visibles y su disposición. No es una entidad de dominio — es configuración de layout del frontend.

2. **PanelVisibility contract**: contrato en TypeScript para visibilidad condicional de paneles. Reemplaza feature gating por modo:
```typescript
interface PanelVisibility {
  panelId: string;
  visible: boolean;
  condition?: (ctx: PanelContext) => boolean;
}
```

3. **Feature gating a nivel workspace** (no a nivel modo): la configuración de workspace existente controla qué features están disponibles. Sin entidad "modo" en el dominio.

4. **Atajos de teclado como implementación, no arquitectura**: los shortcuts para cambiar layouts son detalles de UI, no decisiones arquitectónicas. Se ligan al CommandRegistry existente como comandos `layout:switch-to-*`.

## Considered Options

1. **Work Mode Switcher** (rechazado por Q001-P1 + Q008-P2) — bundling de decisiones, colisión con InputMode, viola ADR-0002
2. **Rutas por modo** (rechazado) — introduce concepto de navegación que no existe en Logseq, fragmenta el SPA
3. **DashboardLayout + PanelVisibility contract** — aceptado: presets de paneles sin entidad de dominio, alineado con ADR-0002

## Consequences

- Las rutas planas del SPA se mantienen (`/`, `/page`, `/journal`, etc.)
- Los paneles se muestran/ocultan por configuración de workspace, no por modo
- `DashboardLayout` no colisiona con `SavedView`: uno es layout de workspace, el otro es vista de datos
- El CommandRegistry expone comandos `layout:switch-to-*` para cambiar layouts vía teclado
- No se crea entidad "WorkMode" ni "DashboardLayout" en el dominio de Rust

## Implementation (2026-06-09)

- `PanelVisibilityContext` en `quilt-ui/src/features/dashboard/PanelVisibilityContext.tsx`
- `PANEL_LABELS` como fuente única de verdad (resuelve S2-01)
- Presets de layout en `quilt-ui/src/features/dashboard/presets.ts`
- `LayoutMenu` para cambiar entre presets
- Phase 2 #17 del roadmap

## References

- Q001-P1 y Q008-P2 (auto-grill 2026-06-07)
- ADR-0002: UI Logseq-like, features AI como paneles
- `docs/quilt-keyboard-shortcuts.md` §1.1: InputMode definido
- `docs/research/ux-workflow-portfolio-analysis.md` §1
