# C4 Context (C1) — Logseq

> **Escala**: 🟢 CONFIRMADO | 🟡 INFERIDO | 🔴 LACUNA
> **Nivel**: Contexto del sistema
> **Fecha**: 2026-05-02

---

## Diagrama de Contexto

```mermaid
C4Context
    accTitle: Logseq System Context
    accDescr: Nivel C1 - Vista de contexto del sistema Logseq

    Person(writer, "Escritor", "Usuario que crea y edita notas en Logseq")
    Person(reader, "Lector", "Usuario que consume contenido publicado")

    System(logseq, "Quilt", "Sistema de gestión de conocimiento personal<br/>con outliner, queries y grafos de conocimiento") {
        Container(frontend, "Frontend", "ClojureScript/Rum", "Editor de bloques, UI reactiva, sistema de queries")
        Container(electron, "Electron Shell", "JavaScript", "App de escritorio multiplataforma")
        Container(datascript, "DataScript", "Clojure", "Base de datos in-memory para documentos")
        Container(fs_abstraction, "File System", "ClojureScript", "Abstracción de archivos locales y cloud")
    }

    System_Ext(filesystem, "Sistema de Archivos", "Archivos Markdown y Org-mode en disco local o cloud")
    System_Ext(git, "Git Provider", "GitHub/GitLab", "Sincronización de archivos y versionado")
    System_Ext(api_external, "APIs Externas", "OpenAI, Zotero...", "Servicios de terceros consumidos por plugins")

    Rel(writer, logseq, "Crea/editar notas, queries, journals")
    Rel(reader, logseq, "Consulta y navega contenido")
    Rel(reader, logseq, "Consume publicaciones")
    Rel(logseq, filesystem, "Lee/escribe archivos .md y .org")
    Rel(logseq, git, "Sincroniza cambios via Git")
    Rel(electron, datascript, "Almacena estado de documentos")
    Rel(frontend, fs_abstraction, "Abstrae operaciones de archivo")
    Rel(fs_abstraction, filesystem, "Implementa operaciones de archivo")
    Rel(frontend, api_external, "Invoca APIs via plugins")

    UpdateRelStyle(writer, logseq, $strokeColor="#4CAF50", $textColor="#4CAF50")
    UpdateRelStyle(reader, logseq, $strokeColor="#2196F3", $textColor="#2196F3")
```

---

## Descripción de Actores

### Actores Externos

| Actor | Tipo | Descripción | 🟢🟡🔴 |
|-------|------|-------------|---------|
| **Escritor** | Primary | Crea, edita y organiza notas usando el outliner | 🟢 |
| **Lector** | Secondary | Consume contenido ya sea in-app o publicado | 🟢 |
| **Sistema de Archivos** | External | Almacena archivos Markdown/Org-mode | 🟢 |
| **Git Provider** | External | Proveedor Git para sincronización | 🟡 |
| **APIs Externas** | External | Servicios consumidos por plugins | 🟡 |

---

## Descripción del Sistema (Logseq)

### Responsabilidad Core
Quilt es un sistema de **gestión de conocimiento personal (PKM)** basado en:
- **Outliner**: Estructura jerárquica de bloques (como org-mode)
- **Grafo de conocimiento**: Links bidireccionales entre páginas y bloques
- **Queries**: Lenguaje DSL para consultar datos
- **Journal**: Notas diarias automáticas
- **Publicación**: Exporta a sitios estáticos

### Scope del Sistema

| Incluido ✅ | Excluido ❌ |
|------------|-------------|
| Edición de bloques con outliner | Procesamiento de texto offline |
| Navegación por grafo | Compilación de código |
| Queries DSL y Datalog | Manejo de email |
| Sincronización Git | Calendario completo |
| Publicación web | Gestión de proyectos completa |
| API de plugins | Base de datos externa |
| Multi-graph support | Mobile nativo (es un wrapper) |

---

## Fronteras del Sistema

### Interno (Logseq)
- Frontend ClojureScript
- DataScript in-memory DB
- Event handling con core.async
- File system abstraction

### Externo
- Sistema operativo (file system real)
- Git providers
- APIs de terceros
- Almacenamiento en la nube (opcional)

---

## Perspectivas técnicas

### Plataforma de ejecución

```yaml
Desktop:
  electron: App wrapper multi-plataforma
  storage: SQLite (desktop) / IndexedDB (web)
  
Web:
  browser: SPA sin servidor propio
  storage: IndexedDB via OpasFS
  
Mobile:
  capacitor: Wrapper de web app
  storage: IndexedDB
```

### Modelo de datos primario

```yaml
Block: Unidad básica de contenido (18 campos)
Page: Colección de bloques con nombre único (11 campos)  
File: Referencia a archivo en disco (6 campos)
Journal: Página especial con fecha (hereda de Page)
```

---

## Confianza del análisis

| Área | Confianza | Notas |
|------|-----------|-------|
| Frontend components | 🟢 Alta | 70+ componentes analizados |
| Event handling | 🟢 Alta | Sistema centralizado confirmado |
| DataScript schema | 🟢 Alta | Schema completo en modules.json |
| File system | 🟢 Alta | Protocolo confirmado |
| Plugins API | 🟡 Media | Documentación parcial |
| Sync/Git | 🟡 Media | Flujo inferido de code-analysis |
| Mobile | 🟡 Media | Wrapper de web, no analizados |

---

*Generado por Reversa Architect - 2026-05-02*
