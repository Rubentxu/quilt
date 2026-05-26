# ADR-0001: Quilt no tiene IA interna — solo PKM + análisis estructural + MCP server

Status: accepted

Quilt es una aplicación PKM con UI estilo Logseq (Leptos 0.8 CSR en browser) que expone toda su funcionalidad via MCP server para que agentes AI externos (Claude, GPT) colaboren con el usuario. Quilt NO integra modelos de IA, clientes de LLM, ni proveedores de AI. El análisis semántico (embeddings, serendipity conceptual, auto-organize semántico) lo hacen los agentes externos con sus propios modelos. Quilt solo implementa análisis estructural en Rust: decay detection, orphan detection, graph connectivity, similitud estructural, y template expansion. Estas capacidades se exponen como MCP tools con prefijo `quilt_*`.

## Considered Options

1. **IA integrada** (Ollama/OpenAI dentro de Quilt) — rejected: añade complejidad, tamaño de binario, dependencia de modelos, y va contra el principio MCP-first
2. **Solo CRUD** (sin análisis) — rejected: el agente necesitaría demasiadas llamadas MCP para entender el grafo
3. **Análisis estructural en Rust, semántica en el agente** — accepted: Quilt provee "qué hay y cómo está conectado", el agente provee "por qué importa y qué hacer"

## Consequences

- `quilt-cognitive` debe eliminarse o reenfocarse: quitar `ai_client`, `ollama`, `openai`, `AIClient`
- Se crea `quilt-analysis` para el motor de análisis estructural puro
- Tauri se elimina completamente: UI es Leptos CSR, shell de desktop no es Tauri
- Sin prefijo `logseq_*` en ningún sitio: todo usa `quilt_*`
