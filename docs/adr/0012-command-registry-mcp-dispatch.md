# ADR: CommandRegistry como React Context con dispatch MCP híbrido

Status: implemented

## Context

El documento de investigación proponía expandir `Cmd/Ctrl+K` en un "Universal Command Center" que unifique búsqueda, comandos, templates, queries y agentes en un solo punto de entrada. La pregunta clave: ¿dónde vive la lógica de dispatch y cómo se registran los comandos?

El auto-grill (Q003-P1 + Q010-P2, sesión 2026-06-07) rechazó dos propuestas:
1. Q003-P1: extender SearchModal con dispatch embebido → god modal anti-pattern, layer leak
2. Q010-P2 (inicial): shortcut conflictivo, CommandContext sin definir, sin dispatch MCP

La decisión final (MODIFIED) definió el contrato completo.

## Decision

**CommandRegistry es un React context en `quilt-ui/src/features/command-center/` con dispatch híbrido client/server vía MCP.**

### Interface TypeScript

```typescript
// quilt-ui/src/features/command-center/types.ts

interface CommandContext {
  query: string;
  scope: 'global' | 'page' | 'block';
}

interface Command {
  id: string;
  label: string;
  category: string;        // string union extensible, no enum cerrado
  shortcut?: string;
  priority: number;         // tiebreaking (menor = más alto)
  target: 'client' | 'server';  // dispatch MCP híbrido
  execute: (ctx: CommandContext) => Promise<void>;
}

interface CommandRegistry {
  register(command: Command): void;
  unregister(id: string): void;
  search(query: string, ctx: CommandContext): Command[];
}
```

### Componentes

1. **CommandRegistry context**: registra comandos. Features registran sus comandos vía `useEffect` con cleanup.
2. **CommandCenter modal**: activado por `Cmd+Shift+K`. Consume `useCommandRegistry()` para buscar y ejecutar. Independiente del SearchModal (`Cmd+K`).
3. **SearchModal**: sigue siendo búsqueda pura de contenido. No recibe dispatch de comandos.
4. **MCP dispatch**: comandos con `target: 'server'` se ejecutan vía `quilt_execute_command` MCP tool. Comandos `target: 'client'` se ejecutan localmente.

### Shortcut: Cmd+Shift+K

El shortcut `Cmd+Shift+J` fue rechazado por conflicto con Chrome DevTools. `Cmd+Shift+K` no tiene conflicto conocido en macOS/Windows/Linux.

## Considered Options

1. **Extender SearchModal** (rechazado por Q003-P1) — god modal, acopla búsqueda con dispatch
2. **Backend command endpoint** (rechazado) — introduce API REST para algo que es UI-level
3. **CommandRegistry como React context + CommandCenter separado** — aceptado: separación de concerns, extensible, MCP-first

## Consequences

- SearchModal y CommandCenter son componentes independientes con shortcuts distintos
- Features registran comandos declarativamente via hooks
- `target: 'client' | 'server'` permite comandos MCP sin backend dedicado
- `priority` permite orden determinista de resultados
- `category` como string union permite que features registren nuevas categorías sin modificar el core
- No requiere cambios en el backend (los comandos server-side van por MCP)

## Implementation (2026-06-09)

- `CommandRegistry` en `quilt-ui/src/features/command-center/`
- Shortcut `Cmd+Shift+K` para activar
- Quick Capture builtin command
- Phase 1 #9 del roadmap

## References

- Q003-P1 y Q010-P2 (auto-grill 2026-06-07)
- `quilt-ui/src/features/search/SearchModal.tsx` — búsqueda actual (602 líneas)
- `quilt-ui/src/features/slash-actions/SlashActionRegistry.ts` — acciones slash existentes
