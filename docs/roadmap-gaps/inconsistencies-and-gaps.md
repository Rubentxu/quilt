# Inconsistencias y gaps detectados en `docs/reversa/`

## Resumen general

La documentación de `docs/reversa/` tiene buena cobertura conceptual, pero el
problema principal no es la falta de documentos sino la **deriva entre ellos**.

Los problemas detectados se agrupan en dos tipos:

1. **Inconsistencias entre documentos**
2. **Huecos documentales que dificultan implementación o mantenimiento**

---

## 1. Inconsistencias entre documentos

### Alta severidad

#### 1.1 Sync visionario vs sync real

**Archivos implicados**

- `rust-mcp-ai-deep-dive.md`
- `rust-reimplementation-proposal.md`
- documentación de estado/confianza

**Problema**

La documentación histórica sugiere una visión de sync más sofisticada
(CRDT/Loro), mientras que la implementación verificada parece apoyarse en una
estrategia más cercana a LWW con variantes.

**Riesgo**

- expectativas erróneas sobre resolución de conflictos
- decisiones de producto basadas en semánticas no implementadas

---

#### 1.2 Task markers con casing inconsistente

**Archivos implicados**

- `domain.md`
- `erd.md`
- `query-dsl-spec.md`
- `rust-mcp-ai-deep-dive.md`

**Problema**

Los markers aparecen en distintas formas: `NOW`, `now`, `Now`, etc.

**Riesgo**

- bugs de parsing
- discrepancias entre persistencia, DSL, UI y enums Rust

---

#### 1.3 Drift entre `erd.md` y `data-dictionary.md`

**Archivos implicados**

- `erd.md`
- `data-dictionary.md`

**Problema**

Ambos describen el schema, pero no contienen exactamente los mismos campos ni
el mismo nivel de detalle.

**Riesgo**

- no existe una fuente canónica clara del modelo de datos
- riesgo de errores al implementar, migrar o depurar

---

### Severidad media

#### 1.4 Reglas de normalización de properties no alineadas

**Archivos implicados**

- `domain.md`
- `data-dictionary.md`

**Problema**

Las reglas de normalización no están descritas igual en ambos documentos.

**Riesgo**

- bugs sutiles en nombres de propiedades

---

#### 1.5 Tipos de fecha/timestamp descritos de forma ambigua

**Archivos implicados**

- `domain.md`
- `erd.md`
- `data-dictionary.md`
- `rust-mcp-ai-deep-dive.md`

**Problema**

Se mezclan términos como `timestamp`, `Long`, `DateTime<Utc>` sin separar con
claridad representación persistida, transporte y modelo interno.

---

#### 1.6 Dos sistemas de hooks/eventos sin relación explícita

**Archivos implicados**

- `domain.md`
- `quilt-mcp-agent-capabilities.md`

**Problema**

Aparecen hooks tipo Logseq heredado y hooks tipo MCP, pero no queda del todo
claro cuál es el sistema canónico para Quilt Rust.

---

## 2. Gaps documentales

### Críticos

#### 2.1 Falta `spec-impact-matrix.md`

Sin una matriz de impacto/dependencias, es difícil evaluar cambios, deuda o
riesgos de regresión.

#### 2.2 Query DSL no completamente cerrado como contrato formal

Aunque existe `query-dsl-spec.md`, convendría una reconciliación explícita con
la implementación real y con `domain.md`.

#### 2.3 Event loop sin contrato formal claro

La documentación no deja suficientemente cerrado el modelo de eventos,
reintentos, contratos de payload y side effects.

---

### Moderados

#### 2.4 Sync sin spec consolidada final

Falta una especificación definitiva que aclare:

- estrategia real de conflicto
- cola offline
- semántica de sincronización
- cifrado/E2EE si aplica

#### 2.5 Deep-link fallback poco especificado

No queda completamente definido el comportamiento en primera ejecución o cuando
falta contexto previo.

#### 2.6 Recycle/orphans con reglas insuficientemente definidas

Faltan reglas claras para reciclado, restore, undo y timing.

#### 2.7 Sistema de clases poco integrado en el documento central

Existe en documentos técnicos, pero no está absorbido del todo en la narrativa
principal de dominio.

#### 2.8 Features cognitivas descritas con más madurez que su estado visible

La documentación de capacidades y workflows propone una experiencia más rica de
la que hoy parece existir end-to-end.

---

## 3. Riesgos prácticos

### Riesgo alto

#### Documentación más avanzada que el producto real

Especialmente en:

- sync avanzado
- features cognitivas visibles
- UI de agentes

**Impacto**: onboarding confuso, estimaciones incorrectas, expectativas
desalineadas.

### Riesgo alto

#### Schema sin fuente canónica única

**Impacto**: errores de modelado, migración, debug y persistencia.

### Riesgo medio

#### Ambigüedad semántica en markers, timestamps y normalization

**Impacto**: bugs pequeños pero recurrentes.

---

## 4. Recomendaciones documentales prioritarias

### Prioridad 1

1. Unificar `erd.md` y `data-dictionary.md`
2. Decidir y documentar la estrategia real de sync
3. Normalizar task markers y representaciones canónicas

### Prioridad 2

4. Crear `spec-impact-matrix.md`
5. Consolidar Query DSL como contrato formal
6. Etiquetar claramente features como implementadas, parciales o planificadas

### Prioridad 3

7. Integrar sistema de clases en la documentación central
8. Documentar el contrato de event loop y hooks
9. Cerrar el comportamiento esperado de recycle/orphans y deep links
