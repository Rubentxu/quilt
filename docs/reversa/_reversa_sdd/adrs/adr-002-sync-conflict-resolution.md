# ADR-002: Sync Conflict Resolution

> Architecture Decision Record - Logseq
> Generado por: reversa-detective
> Fecha: 2026-05-02

---

## Status

🟢 CONFIRMADO - Implementado y en producción

---

## Context

Quilt usa sincronización en tiempo real con múltiples clientes. Cuando dos clientes modifican el mismo bloque simultáneamente, pueden возникнуть conflictos.

**Problemas identificados**:
- Transacciones concurrentes pueden crear inconsistencias
- Necesidad de CRDT (Conflict-free Replicated Data Types) o similar
- El servidor necesita validar transacciones para prevenir estados corruptos

---

## Decision

Implementar un sistema de **sincronización optimista con validación de estado**:

1. **Checksum-based validation**: Cada transacción incluye un checksum del estado previo
2. **Transaction replay**: Transacciones se reaplican en orden
3. **Stale operation rejection**: Transacciones obsoletas son rechazadas
4. **UUID preservation**: Los UUIDs de bloques se preservan en redo/undo

---

## Evidence (Git Log)

```
fix(sync): reject stale numeric history ops and surface worker ex-data
fix(outliner): resolve stale numeric ids in semantic ops
fix(sync): preserve apply-template block uuids on redo
fix(sync): remap apply-template internal value refs on redo
fix: validate client tx and ensures monotonically increasing
fix(sync): handle snapshot reset and tx epoch rollback
fix: client/server checksum mismatch
```

---

## Implementation Details

**Checksum validation**:
```clojure
;; Cada tx incluye checksum del estado antes de aplicar
;; Si checksum no coincide, tx es rechazada
(let [current-checksum (client-op/get-local-checksum repo)
      new-checksum (sync-checksum/update-checksum current-checksum tx-report)]
  (client-op/update-local-checksum repo new-checksum))
```

**Conflict detection**:
```clojure
(defn- remote-sync-conflicts
  [rebase-db-before local-txs remote-txs]
  ;; Compara checksums para detectar conflictos
)
```

**Transaction ordering**:
- Transacciones tienen timestamp monotonically increasing
- TX epoch permite rollback a estado anterior
- Historia de operaciones protegida contra modificación

---

## Consequences

**Positive**:
- ✅ Conflictos detectados y manejados
- ✅ Estado consistente entre clientes
- ✅ Operaciones stale detectadas tempranamente

**Negative**:
- ❌ Complejidad en el cliente de sincronización
- ❌ Overhead de validación de checksums

---

## Related ADRs

- [ADR-001: E2EE Sync](./adr-001-e2ee-sync.md)

---

*Documento generado automáticamente por Reversa Detective*
