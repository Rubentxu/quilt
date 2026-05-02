# frontend/fs — File System Abstraction

## Visão Geral
Abstracción del sistema de archivos que soporta múltiples backends (Node.js para Electron, Memory para tests). Permite operaciones de archivo y directorio independientes de la plataforma mediante el protocolo `Fs`, seleccionando automáticamente la implementación adecuada según el entorno y tipo de activo.

## Responsabilidades
- Seleccionar el backend de archivos adecuado según plataforma (Electron vs Browser) y tipo de recurso (assets vs plain text)
- Proporcionar operaciones CRUD sobre archivos y directorios: crear, leer, escribir, renombrar, copiar, eliminar
- Gestionar la creación recursiva de directorios
- Obtener metadatos de archivos (`stat`: tipo, tamaño, fecha de modificación)
- Observar directorios para detectar cambios en tiempo real (`watch-dir!`)
- Abrir directorios como grafos nuevos (`open-dir`)

## Interface

### Protocolo `Fs`
```clojure
(defprotocol Fs
  (mkdir!       [this dir])
  (mkdir-recur! [this dir])
  (readdir      [this dir])
  (unlink!      [this repo path opts])
  (rmdir!       [this dir])
  (read-file    [this dir path opts])
  (read-file-raw [this dir path opts])
  (write-file!  [this repo dir path content opts])
  (rename!      [this repo old-path new-path])
  (copy!        [this repo old-path new-path])
  (stat         [this path])
  (open-dir     [this dir])
  (get-files    [this dir])
  (watch-dir!   [this dir options])
  (unwatch-dir! [this dir]))
```

### Funciones públicas (fs.cljs)

| Función | Parámetros | Retorno |
|---------|------------|---------|
| `get-fs` | `dir {:keys [repo rpath]}` | `fs-backend` |
| `mkdir-recur!` | `dir` | `promise` |
| `readdir` | `dir {:keys [path-only?]}` | `[string]` |
| `write-plain-text-file!` | `repo dir rpath content opts` | `promise` |
| `read-file` | `dir path options` | `string` |
| `stat` | `fpath` | `{:type :size :mtime}` |
| `file-exists?` | `fpath` | `boolean` |
| `create-if-not-exists` | `repo dir path initial-content` | `boolean` |

### Entidades de datos

| Entidad | Campos | Descripción |
|---------|--------|-------------|
| `FsBackend` | `:type` (`:node` \| `:memory`), `:supports-watch?` | Backend activo seleccionado |
| `FileStat` | `:type` (`"file"` \| `"dir"`), `:size`, `:mtime` | Metadatos de archivo/directorio |

### Implementaciones
- **`FsNode`** (`frontend/fs/node.cljs`): Backend para Electron usando `fs-extra`. Soporta todas las operaciones incluyendo `watch-dir!`.
- **`FsMemory`** (`frontend/fs/memory_fs.cljs`): Backend en memoria para tests y entornos browser. Implementa un sistema de archivos virtual con `mkdir-recur!` recursivo.

## Regras de Negócio
- 🟢 La selección de backend (`get-fs`) prioriza: si es Electron y el archivo es un asset → `node` backend; caso contrario → `node` en Electron, `memory` en browser
- 🟢 `write-plain-text-file!` publica un evento de error al state global si la escritura falla
- 🟢 Los assets en Electron usan `node` backend para escritura binaria (`fs.cljs:34-35`)
- 🟢 `create-if-not-exists` retorna `true` si el archivo ya existía, `false` si lo creó
- 🟡 La creación recursiva en `FsMemory` itera hacia arriba hasta encontrar un padre existente

## Fluxo Principal

### Escritura de archivo
1. Cliente llama a `write-plain-text-file!` con `repo`, `dir`, `rpath`, `content`, `opts`
2. `get-fs` selecciona el backend (`FsNode` o `FsMemory`) según el directorio y tipo de archivo
3. El backend escribe el contenido en la ruta especificada
4. Si la escritura falla, se publica el error en `state/pub-event!`
5. Retorna `promise` resuelta al completar

### Lectura de archivo
1. Cliente llama a `read-file` con `dir`, `path`, `options`
2. `get-fs` selecciona el backend
3. El backend lee el archivo y retorna el contenido como `string` (o bytes si `read-file-raw`)

### Apertura de directorio como grafo
1. `open-dir` recibe un `dir`
2. El backend lista recursivamente los archivos del directorio
3. Retorna `{:path string :files [{:path string :content string}]}` con todos los archivos y su contenido

## Fluxos Alternativos
- **Archivo no existe en `read-file`:** El backend lanza error o retorna `nil` según implementación
- **Directorio ya existe en `mkdir!` / `mkdir-recur!`:** No produce error — operación idempotente
- **Archivo no existe en `create-if-not-exists`:** Crea el archivo con `initial-content` y retorna `false`
- **Archivo ya existe en `create-if-not-exists`:** No modifica el archivo y retorna `true`
- **Watch no soportado:** Si el backend no soporta `watch-dir!` (`supports-watch? = false`), la operación es no-op

## Dependências
- `electron.ipc` — Comunicación con el proceso principal de Electron para operaciones de archivo
- `promesa.core` — Manejo de promesas asíncronas
- `cljs-bean.core` — Conversión entre ClojureScript y JavaScript
- `logseq.common.path` — Utilidades de manipulación de rutas
- `fs-extra` (Node.js) — Librería nativa de sistema de archivos para el backend Node

## Requisitos Não Funcionais

| Tipo | Requisito inferido | Evidência no código | Confiança |
|------|--------------------|---------------------|-----------|
| Disponibilidad | Errores de escritura se capturan y publican como eventos al state global | `frontend/fs.cljs:87-94` | 🟢 |
| Portabilidad | El protocolo `Fs` permite cambiar entre `node` y `memory` sin modificar código cliente | `frontend/fs/protocol.cljs` | 🟢 |
| Extensibilidad | Nuevos backends pueden implementar el protocolo `Fs` sin modificar `get-fs` | `frontend/fs/protocol.cljs` | 🟢 |

> Inferido a partir del código. Validar con equipo de operaciones.

## Critérios de Aceitação

```gherkin
Dado un grafo existente con directorio "pages/"
Cuando se llama a write-plain-text-file! con ruta "pages/nueva-pagina.md" y contenido válido
Então el archivo se crea en disco (o memoria según backend)
Y la promesa se resuelve exitosamente

Dado un grafo existente con archivo "config.edn"
Cuando se llama a read-file con la ruta "config.edn"
Então se retorna el contenido del archivo como string

Dado un directorio "journals/" que no existe
Cuando se llama a mkdir-recur! con "journals/"
Então el directorio se crea recursivamente
Y la promesa se resuelve sin error

Dado un archivo que no existe en la ruta especificada
Cuando se llama a create-if-not-exists con initial-content
Então el archivo se crea con ese contenido
Y se retorna false indicando que fue creado

Dado un archivo que ya existe en la ruta especificada
Cuando se llama a create-if-not-exists
Então el archivo no se modifica
Y se retorna true indicando que ya existía

Dado un backend que no soporta watch (FsMemory o browser sin soporte)
Cuando se llama a watch-dir!
Então la operación es no-op sin lanzar error
```

## Prioridade

| Requisito | MoSCoW | Justificativa |
|-----------|--------|---------------|
| Selección de backend (`get-fs`) | Must | Punto de entrada único — toda operación de archivo depende de esta función |
| Escritura de archivos (`write-file!`) | Must | Crítico para persistir contenido de páginas y bloques |
| Lectura de archivos (`read-file`) | Must | Requerido para cargar grafos y mostrar contenido |
| Creación recursiva de directorios | Must | Necesario para inicializar estructura de grafos nuevos |
| `stat` | Should | Importante para verificar existencia y metadatos, pero con alternativas |
| `watch-dir!` | Should | Crucial en Electron para detectar cambios externos; no disponible en browser |
| `open-dir` / `get-files` | Should | Usado en restauración de grafos; no en flujo normal |
| `rename!` / `copy!` | Could | Operaciones administrativas poco frecuentes |

> Prioridad inferida por frecuencia de llamada y posición en la cadena de dependencias.

## Rastreabilidade de Código

| Arquivo | Função / Classe | Cobertura |
|---------|-----------------|-----------|
| `src/main/frontend/fs.cljs` | `get-fs`, `mkdir-recur!`, `readdir`, `write-plain-text-file!`, `read-file`, `stat`, `file-exists?`, `create-if-not-exists` | 🟢 |
| `src/main/frontend/fs/protocol.cljs` | Protocolo `Fs` (14 métodos) | 🟢 |
| `src/main/frontend/fs/node.cljs` | Implementación `FsNode` | 🟢 |
| `src/main/frontend/fs/memory_fs.cljs` | Implementación `FsMemory` | 🟢 |

## Cenários de Borda

### Archivo de gran tamaño en `read-file-raw`
- **Contexto**: Assets grandes (imágenes, PDFs) leídos como bytes raw
- **Comportamiento**: `FsNode` usa streams de Node.js para evitar cargar el archivo completo en memoria; `FsMemory` carga todo en memoria (limitado al entorno de tests)

### Directorio con caracteres especiales en ruta
- **Contexto**: Rutas que contienen espacios, tildes, o caracteres Unicode
- **Comportamiento**: `logseq.common.path` normaliza las rutas; el backend subyacente (`fs-extra` en Node) maneja la codificación nativa. En `FsMemory`, las rutas se almacenan como strings Clojure sin transformación adicional

### Concurrencia en escritura de archivos
- **Contexto**: Dos operaciones de escritura simultáneas sobre el mismo archivo
- **Comportamiento**: No hay locking explícito. La última escritura prevalece (last-write-wins). El sistema de eventos `core.async` del handler secuencia las operaciones a nivel aplicación, pero el backend de archivos no garantiza atomicidad entre escrituras concurrentes
