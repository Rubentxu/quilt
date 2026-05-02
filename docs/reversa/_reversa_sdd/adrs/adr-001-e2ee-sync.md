# ADR-001: End-to-End Encryption (E2EE) para Sync

> Architecture Decision Record - Logseq
> Generado por: reversa-detective
> Fecha: 2026-05-02

---

## Status

🟢 CONFIRMADO - Implementado y en producción

---

## Context

Quilt permite sincronizar grafos con un servidor remoto. El servidor almacena datos del usuario, lo que plantea preocupaciones de privacidad.

**Problemas identificados**:
- Servidor podría acceder al contenido de los grafos de usuarios
- Necesidad de cumplir políticas de privacidad
- Usuarios requieren garantías de confidencialidad

---

## Decision

Implementar **End-to-End Encryption (E2EE)** donde:
1. La clave de cifrado se deriva de la contraseña del usuario
2. El contenido se cifra localmente ANTES de subir al servidor
3. El servidor solo almacena contenido cifrado (no puede leer)
4. Los metadatos de sincronización permanecen sin cifrar (necesarios para sync)

---

## Evidence (Git Log)

```
fix: capitalize paid feature consistently like we do with Sync
fix(e2ee): use native secret storage and init remote sync config
fix(cli): sync status fails with unactionable e2ee-password-not-found error
fix(sync): preserve apply-template block uuids on redo
fix(sync): remap apply-template internal value refs on redo
```

---

## Implementation Details

**Key derivation**:
```clojure
;; Password → Key Derivation Function → Encryption Key
;; AES-256-GCM para cifrado de contenido
```

**Encrypted elements**:
- Block content
- Page titles (para graphs E2EE)
- Property values

**Non-encrypted elements**:
- UUIDs (necesarios para referencias)
- Sync metadata
- Transaction timestamps

---

## Consequences

**Positive**:
- ✅ Privacidad garantizada: servidor no puede leer contenido
- ✅ Cumplimiento de regulaciones de datos
- ✅ Los metadatos de sync siguen disponibles para funcionalidad

**Negative**:
- ❌ Si el usuario olvida la contraseña, los datos son irrecuperables
- ❌ Complejidad adicional en el cliente
- ❌ Feature de pago ("paid feature")

**Neutral**:
- ⚪ Los títulos de página enrypted requieren manejo especial para referencias

---

## Alternatives Considered

1. **No encryption**: Servidor almacena todo en texto plano
   - ❌ Problemas de privacidad
   - ✅ Simplicidad

2. **Encryption at rest only**: Cifrar en servidor
   - ❌ Servidor tiene acceso a claves
   - ✅ Simplicidad operacional

3. **E2EE seleccionada**: Cifrado local con clave del usuario
   - ✅ Máxima privacidad
   - ❌ Complejidad

---

## Related ADRs

- [ADR-002: Sync Conflict Resolution](./adr-002-sync-conflict-resolution.md)

---

*Documento generado automáticamente por Reversa Detective*
