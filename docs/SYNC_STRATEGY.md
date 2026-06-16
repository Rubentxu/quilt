# Sync Strategy — LWW vs CRDT

> **Estado**: Implementado (LWW) | **Roadmap**: CRDT via Loro (diferido)
> **Última actualización**: 2026-06-16

---

## Estrategia Actual: LWW (Last-Write-Wins)

La implementación actual de sync en Quilt utiliza **LWW (Last-Write-Wins)** basado en timestamps.

### Implementación

El merge de propiedades se realiza en `crates/quilt-domain/src/properties/merge.rs`:

```rust
pub fn merge_properties<E: PropertyEntry + Clone>(
    existing: &HashMap<String, E>,
    incoming: HashMap<String, E>,
) -> HashMap<String, E>
```

### Contrato de Merge

| Escenario | Comportamiento |
|-----------|----------------|
| Claves distintas | Ambas sobreviven (aditivo) |
| Misma clave, ambos con timestamp | Gana el de `updated_at` más reciente |
| Misma clave, solo uno con timestamp | Gana el que tiene timestamp |
| Misma clave, ninguno con timestamp o timestamps iguales | Gana el valor existente (determinístico) |

### Características

- **Función pura**: no muta los inputs, retorna un map nuevo
- **Determinístico**: misma entrada produce misma salida (sin aleatoriedad)
- **Propiedad**: tested via proptest para 10,000+ ejecuciones

### Limitaciones de LWW

- No preserva historial de ediciones concurrentes
- No permite "undo" colaborativo
- Conflicto resuelto por timestamp, no por contenido

---

## Roadmap: CRDT via Loro (Diferido)

### Visión Original

Los documentos de diseño originales (`rust-mcp-ai-deep-dive.md`, `rust-reimplementation-proposal.md`)
mencionaban **Loro CRDT** como estrategia de sync:

```rust
use loro::{LoroDoc, LoroText, LoroList, LoroMap};
```

### Estado Actual

| Aspecto | Diseño original | Implementación real |
|---------|-----------------|-------------------|
| Biblioteca | `loro = "0.2"` | No incluida en `Cargo.toml` |
| Estrategia | CRDT (convergencia automática) | LWW (timestamp-based) |
| Resolución de conflictos | Automática via CRDT | Explícita con `merge_properties()` |
| Complejidad | Alta | Baja |

### Decision: LWW es Intencional (Por Ahora)

La estrategia LWW fue elegida por:

1. **Simplicidad**: menor superficie de bugs
2. **Determinismo**: fácil de testear y razonar
3. **Rendimiento**: sin overhead de CRDT
4. **Adecuado para el caso de uso**: sync de propiedades, no editing colaborativo

### Cuando Considerar CRDT

- Editing colaborativo en tiempo real (múltiples usuarios editando simultáneamente)
- Conflictos semánticos complejos (no solo timestamps)
- Offline-first con merge automático inteligente

### Referencias a Loro en Docs (Para Actualizar)

Los siguientes archivos mencionan Loro/CRDT como implemented pero son **documentación histórica/de diseño**,
no el estado actual:

| Archivo | Notas |
|---------|-------|
| `docs/reversa/rust-mcp-ai-deep-dive.md` §6 | Sección "PLANNED" — no implementada |
| `docs/reversa/rust-reimplementation-proposal.md` §sync | Diseño original — no implementado |
| `docs/reversa/quilt-ui-workflows.md` | Menciones genéricas de CRDT |
| `docs/reversa/_reversa_sdd/` | Documentos de diseño internos |

### ADR Relacionado

Ver `docs/reversa/_reversa_sdd/adrs/adr-002-sync-conflict-resolution.md` para la decisión
de diseño sobre CRDT.

---

## Links

- Implementación: `crates/quilt-domain/src/properties/merge.rs`
- Test de merge: `crates/quilt-domain/src/properties/merge.rs` (módulo tests)
- Tests de determinismo: `tests::deterministic_same_inputs_same_output`
