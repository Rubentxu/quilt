# Questions for User Validation — Logseq Reverse Engineering

**Proyecto:** Logseq  
**Fecha:** 2026-05-02  
**Nivel:** detalhado

---

## Preguntas sobre 🔴 Lacunas Críticas

### Q-001: graph-name en deep links sin fallback definido
**Archivo:** `electron.md` línea 151  
**Contexto:** La spec indica que si un deep link `logseq://` no incluye `graph-name`, se abre "el último grafo usado o la pantalla de selección". No hay más detalle.

**Preguntas:**
1. ¿Cómo se determina cuál es el "último grafo usado"?
   - ¿Se persiste en localStorage?
   - ¿Es el grafo que estaba abierto cuando se cerró la app?
   - ¿O es aleatorio/no-determinístico?
2. ¿Qué pasa en la primera ejecución cuando no hay "último grafo"?
   - ¿Se muestra la pantalla de selección de grafo?
   - ¿Se intenta abrir un grafo default?
   - ¿Se muestra un error?
3. ¿Hay algún log o metric tracking para deep links sin graph-name?

**Relevancia:** Si el usuario hace clic en un link `logseq://` malformado, ¿qué experiencia tendrá?

---

### Q-002: Query DSL como sistema crítico sin spec
**Archivo:** `frontend-db.md` (mención superficial), `query_dsl.cljs` (sin spec)  
**Contexto:** El sistema de queries DSL es el corazón de la funcionalidad de Logseq. Sin embargo, no tiene una especificación formal SDD.

**Preguntas:**
1. ¿Se planea crear una spec formal para Query DSL?
   - Si no: ¿por qué se considera innecesario?
   - Si sí: ¿en qué fase/iteración?
2. ¿Cuál es la prioridad del equipo para mantener Query DSL estable vs cambiarlo?
3. ¿Hay planes de extender la gramática DSL en el corto plazo?

**Relevancia:** Sin spec, cualquier refactor de Query DSL es de alto riesgo.

---

### Q-003: Event loop error handling y Sentry
**Archivo:** `handler/events.cljs`  
**Contexto:** La spec indica que errores en handlers se capturan con try/catch, se loggean y se envían a Sentry.

**Preguntas:**
1. ¿Qué tipos de errores NO se envían a Sentry?
   - ¿Errores de usuario (invalid input) van a Sentry?
   - ¿Errores de red?
2. ¿Hay alerts automáticas en Sentry para ciertos tipos de errores?
3. ¿Se hace triage manual de errores o todo es automático?

**Relevancia:** Para entender la observabilidad real del sistema.

---

## Preguntas sobre 🟡 Contradicciones

### Q-004: Orden del Agency pattern en búsqueda
**Archivo:** `frontend-search.md` línea 63 vs `agency.cljs:23-26`  
**Contexto:** La spec dice "Browser primero, luego Plugins". El código hace "Plugins primero, luego Browser".

**Preguntas:**
1. ¿El orden de ejecución de motores de búsqueda es importante para la semántica?
   - ¿Los resultados de plugins pueden sobrescribir resultados del browser?
   - ¿O simplemente se concatenan?
2. ¿Deberíamos corregir la spec o el código?
3. ¿Hay tests que verifiquen el orden?

**Relevancia:** Si el orden importa, el código actual está mal y debe corregirse.

---

## Preguntas sobre 🟡 Incomplete Specs

### Q-005: journal-day como integer vs string
**Múltiples archivos:** `frontend-db.md`, `graph-parser.md`  
**Contexto:** Múltiples specs mencionan `journal-day` como "YYYYMMDD integer". Pero nunca se verificó el schema de DataScript.

**Preguntas:**
1. ¿El tipo de `journal-day` en el schema es realmente `:db.type/int`?
2. ¿Hay alguna razón para que sea string en algunos contextos?
3. ¿Hay tests que depended del tipo para comparaciones?

**Relevancia:** Un cambio de tipo podría romper comparaciones y queries.

---

### Q-006: LRU Cache eviction strategy
**Archivo:** `frontend-format.md` línea 64  
**Contexto:** Se menciona un LRU cache de 5000 entries pero no se especifica la estrategia de eviction.

**Preguntas:**
1. ¿El cache usa LRU real, LFU, o simplemente ignora nuevas entradas cuando está lleno?
2. ¿Hay diferencia de comportamiento entre environments (dev vs prod)?
3. ¿Hay metrics sobre cache hit rate?

**Relevancia:** Para entender performance characteristics del parsing.

---

### Q-007: Páginas huérfanas y recycle bin
**Archivo:** `outliner.md` línea 85  
**Contexto:** La spec dice que páginas sin bloques "se mandan a recycle" pero no especifica el trigger.

**Preguntas:**
1. ¿La página se mueve a recycle inmediatamente cuando se elimina el último bloque?
   - ¿O hay un debounce?
   - ¿O se hace en batch durante el próximo sync?
2. ¿Qué pasa si una transacción se pierde (network failure) y la página queda huérfana transiently?
3. ¿Hay algún mecanismo de "undo" para esto?

**Relevancia:** Para entender integrity guarantees del sistema.

---

### Q-008: Search index retry policy
**Archivo:** `handler/events.cljs`  
**Contexto:** Se programa retry en 5s cuando falla el build del search index.

**Preguntas:**
1. ¿Cuántos reintentos se hacen máximo?
2. ¿Hay backoff exponencial o es siempre 5s?
3. ¿Después del último retry fallido, el sistema queda sin search index indefinidamente?

**Relevancia:** Para entender resiliencia del sistema de búsqueda.

---

## Preguntas sobre Cobertura

### Q-009: Extensiones y Plugin API
**Archivo:** `extensions/` (20 archivos sin specs)  
**Contexto:** Las extensiones (PDF, LaTeX, Graph, Zotero, etc.) son parte de la API pública para plugins.

**Preguntas:**
1. ¿Se planea formalizar la Plugin API?
2. ¿Cuáles extensiones son "core" vs "community maintained"?
3. ¿Hay stability guarantees para extensiones?

**Relevancia:** Para saber si vale la pena documentar extensiones.

---

### Q-010: Worker sync system
**Archivo:** `worker/sync/` (15+ archivos sin specs)  
**Contexto:** El sistema de sync maneja colaboración en tiempo real, encryption, y conflict resolution.

**Preguntas:**
1. ¿El sync system es activo en todos los grafos o solo en algunos?
2. ¿Qué pasa cuando hay conflictos de edición concurrentes?
   - ¿Last-write-wins?
   - ¿Se preserva history de ambos?
3. ¿Hay modo offline completo?

**Relevancia:** Para entender el modelo de colaboración.

---

## Preguntas sobre Validación Arquitectónica

### Q-011: Decisiones de diseño Legacy
**Archivos:** Múltiples  
**Contexto:** El proyecto tiene años de deuda técnica. Algunas decisiones parecen subóptimas (ej: journal-day como int, LRU cache simple).

**Preguntas:**
1. ¿Hay planes de refactorizar el sistema de storage (DataScript)?
2. ¿Se considera migrar a otro DB (SQLite, RocksDB)?
3. ¿Hay constraints técnicos que justifiquen decisiones actuales?

---

## Resumen

| # | Pregunta | Prioridad | Responde |
|---|----------|-----------|----------|
| Q-001 | Deep link fallback behavior | 🔴 ALTA | UX/Comportamiento |
| Q-002 | Query DSL spec plan | 🔴 ALTA | Priorización |
| Q-003 | Error handling en event loop | 🟡 MEDIA | Observabilidad |
| Q-004 | Agency order importante? | 🟡 MEDIA | Contradicción código/spec |
| Q-005 | journal-day type | 🟡 MEDIA | Verificación |
| Q-006 | LRU eviction strategy | 🟢 BAJA | Performance |
| Q-007 | Orphan page trigger | 🟢 BAJA | Integrity |
| Q-008 | Search retry policy | 🟢 BAJA | Resiliencia |
| Q-009 | Plugin API formal | 🟢 BAJA | Extensibilidad |
| Q-010 | Sync conflict resolution | 🟡 MEDIA | Colaboración |
| Q-011 | Legacy refactor plans | 🟢 BAJA | Arquitectura |

**Preguntas que requieren respuesta antes de proceder:**
1. Q-001 (deep links)
2. Q-002 (Query DSL priority)
3. Q-004 (Agency order)

---

*Reporte generado por reversa-reviewer*
