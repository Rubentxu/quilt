# ADR-0009: Formato inline Logseq-compatible — `^^text^^` para highlight

Status: accepted

Quilt usa sintaxis Logseq-compatible para formato inline. Bold (`**text**`), italic (`*text*`), strikethrough (`~~text~~`), code (`` `text` ``) y links (`[label](url)`) coinciden con Markdown estándar. Highlight usa `^^text^^`, sintaxis propia de Logseq (heredada de Org-mode), que no existe en Markdown estándar. El parser inline ya no es Markdown genérico — parsea `[[ ]]`, `(( ))`, `#tag`, `property:: value`, y ahora `^^highlight^^`.

## Considered Options

1. **Logseq-compatible (`^^text^^`)** — accepted: consistente con el posicionamiento "superar a Logseq", sintaxis familiar para usuarios migrados, extensible a colored highlights (`^^[color]text^^`), ya estamos fuera de Markdown estándar con `[[ ]]` y `(( ))`.
2. **Obsidian-compatible (`==text==`)** — rejected: ecosistema diferente, incompatible con Logseq, no alineado con el target de Quilt.
3. **Ambos con `^^` como canonical** — rejected: parser más complejo, confuso para el usuario (escribe `==`, ve `^^`).
4. **HTML fallback (`<mark>text</mark>`)** — rejected: feo en texto plano, no es la experiencia Logseq que buscamos replicar y superar.

## Consequences

- `Mark` enum en el dominio: `Bold`, `Italic`, `Strikethrough`, `Code`, `Highlight { color: Option<String> }`, `Link { url, label }`.
- `InlineParser` reconoce `^^..^^` como `Mark::Highlight`.
- Exportación a Markdown estándar: un renderer convierte `^^text^^` → `<mark>text</mark>`.
- CodeMirror 6 decorations aplican estilo visual (background color) al rango `^^..^^`.
- La sintaxis `^^` se puede extender en el futuro para highlights de color: `^^[#ff0]text^^` o similar.
