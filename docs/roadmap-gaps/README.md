# Roadmap de gaps y cobertura de features

Esta carpeta consolida el análisis realizado sobre la relación entre las
features propuestas en `docs/reversa/` y su cobertura real en Rust.

## Objetivo

Responder tres preguntas:

1. Qué features propuestas están realmente implementadas en Rust.
2. Qué gaps funcionales o documentales siguen abiertos.
3. Qué roadmap conviene seguir para cerrar la brecha entre visión,
   implementación backend y experiencia end-to-end.

## Contenido

- `feature-coverage-matrix.md`
  - Matriz de features propuestas → estado en Rust.
  - Distingue backend, parcial, stub y no encontrado.

- `inconsistencies-and-gaps.md`
  - Inconsistencias detectadas entre documentos de `docs/reversa/`.
  - Huecos documentales y riesgos prácticos para implementación.

- `roadmap.md`
  - Roadmap detallado y priorizado para cerrar gaps reales.
  - Separado entre normalización documental, backend, UI y features
    cognitivas/end-to-end.

## Resumen ejecutivo

La conclusión actual es:

- **El core backend en Rust está bien cubierto.**
- **No todas las features propuestas están cubiertas end-to-end.**
- La mayor brecha está en:
  - UI cognitiva
  - workflows de agente
  - algunas capacidades avanzadas de sync/producto
  - reconciliación documental entre specs históricas

## Convención de estado

- **Implementada**: hay evidencia clara en código Rust.
- **Parcial**: existe parte relevante, pero falta wiring o cobertura total.
- **Stub**: hay estructura o placeholder, pero no comportamiento completo.
- **No encontrada**: no se verificó implementación con la evidencia revisada.

## Nota importante

Estos documentos intentan separar con claridad:

- lo implementado en **backend Rust**,
- lo implementado **end-to-end**,
- y lo que sigue siendo **visión o propuesta** en `docs/reversa/`.
