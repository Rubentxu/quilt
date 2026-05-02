# ADR-004: Publishing System Architecture

> Architecture Decision Record - Logseq
> Generado por: reversa-detective
> Fecha: 2026-05-02

---

## Status

🟢 CONFIRMADO - Implementado y en producción

---

## Context

Quilt permite publicar páginas seleccionadas como sitio web estático. Necesita control granular sobre qué páginas son públicas y cómo se publican.

**Requerimientos**:
- Páginas individuales configurables como públicas/privadas
- Opción global para hacer todas las páginas públicas
- Páginas protegidas con contraseña
- SEO-friendly (sitemaps, slugs, etc.)

---

## Decision

Implementar **Publishing con visibility por página + defaults globales**:

1. **Per-page visibility**: `publishing-public?` property
2. **Global default**: `all-pages-public?` config flag
3. **Password protection**: Individual page password
4. **SEO features**: Custom slugs, sitemap, metadata

**Publishing filter logic**:
```clojure
;; Si all-pages-public? = true → todas públicas excepto con publishing-public? = false
;; Si all-pages-public? = false → solo públicas las que tienen publishing-public? = true
(defn filter-only-public-pages-and-blocks [db]
  ;; ... filter logic
)
```

---

## Evidence (Git Log)

```
fix(publish): hide protected pages from ref and user listings
fix(publish): hide protected page content from tag listings
fix(publish): remove public /pages listing endpoint
fix(publish): keep legacy short/page URL compatibility
fix(publish): hide hidden properties in page render
feat: configurable publish server URL
fix(publish): acquire DO stubs in sync let to avoid RpcPromise clone
```

**From code**:
```clojure
;; deps/publishing/src/logseq/publishing/db.cljs
(defn filter-only-public-pages-and-blocks
  "Prepares a database assuming all pages are private unless
   a page has a publishing-public? property set to true"
  [db]
  ;; ...
)
```

---

## Publishing Privacy Model

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│   all-pages-public? = false (default)                               │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │  Page publishing-public? = true  ──► PUBLIC                 │   │
│   │  Page publishing-public? = false ──► PRIVATE                │   │
│   │  Page with password ──► PASSWORD PROTECTED                  │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│   all-pages-public? = true                                          │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │  Page publishing-public? = false ──► PRIVATE                │   │
│   │  Page publishing-public? = true  ──► PUBLIC                │   │
│   │  Page without explicit flag ──► PUBLIC                      │   │
│   └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Consequences

**Positive**:
- ✅ Control granular por página
- ✅ Default privado seguro
- ✅ SEO friendly con configuración
- ✅ Legacy URL support

**Negative**:
- ❌ Complex logic para determinar visibilidad
- ❌ Password protection es separate de E2EE

---

## Related Decisions

| Decision | Status |
|----------|--------|
| Protected pages hidden from listings | 🟢 CONFIRMADO |
| Protected pages not in sitemap | 🟢 CONFIRMADO |
| Legacy short URLs supported | 🟢 CONFIRMADO |
| Custom publish server URL | 🟢 CONFIRMADO |

---

*Documento generado automáticamente por Reversa Detective*
