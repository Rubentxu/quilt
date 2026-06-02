# DESIGN.md — Interfaz de Gestión de Conocimiento tipo Logseq

## 1. Propósito

Este documento define las normas visuales, de UX y de implementación para una aplicación web de gestión de conocimiento, diarios, páginas enlazadas, referencias, documentación técnica y paneles de trabajo.

La interfaz toma como referencia conceptual herramientas tipo Logseq, Obsidian y Notion, pero debe ofrecer una experiencia más profesional, clara, moderna y adecuada para entornos corporativos.

El diseño debe seguir los principios de Material Design 3 de Google:

* Jerarquía visual clara.
* Superficies limpias y estructuradas.
* Uso semántico del color.
* Tipografía consistente.
* Componentes reutilizables.
* Estados interactivos evidentes.
* Accesibilidad por defecto.
* Diseño adaptable a escritorio y responsive.

---

## 2. Principios de diseño

### 2.1 Claridad antes que decoración

La interfaz debe priorizar lectura, navegación y edición de conocimiento. Los efectos visuales deben reforzar la comprensión, nunca competir con el contenido.

**Norma:**

* No usar fondos saturados.
* No usar sombras pesadas.
* No abusar de bordes, iconos o gradientes.
* No ocultar acciones principales detrás de menús si son de uso frecuente.

### 2.2 Jerarquía visual explícita

Cada pantalla debe comunicar claramente:

1. Dónde estoy.
2. Qué contenido estoy viendo.
3. Qué elementos son navegables.
4. Qué acciones puedo realizar.
5. Qué contenido está relacionado.

**Norma:**

* El área activa debe estar claramente resaltada.
* Los títulos de página o fecha deben dominar visualmente.
* Las referencias, enlaces y metadatos deben diferenciarse del texto normal.
* Las acciones secundarias deben tener menor peso visual.

### 2.3 Densidad equilibrada

La aplicación puede manejar mucha información, pero no debe parecer saturada.

**Norma:**

* Mantener espaciado generoso en el contenido principal.
* Usar densidad media en el sidebar.
* Permitir modo compacto opcional para usuarios avanzados.
* Evitar listas sin agrupación visual.

### 2.4 Continuidad mental

El usuario debe poder pasar de diario a página, de página a referencia y de referencia a grafo sin perder contexto.

**Norma:**

* Mantener navegación persistente.
* Usar pestañas o breadcrumbs.
* Conservar estado de scroll cuando se cambia de vista.
* Mostrar referencias vinculadas cerca del bloque que las origina.

---

## 3. Arquitectura visual de la pantalla

La pantalla principal se divide en tres zonas:

```text
┌────────────────────────────────────────────────────────────┐
│ Top bar / Tabs / Acciones globales                         │
├───────────────┬────────────────────────────────────────────┤
│ Sidebar       │ Área principal de contenido                 │
│ navegación    │ Diarios, páginas, referencias, bloques      │
│               │                                            │
└───────────────┴────────────────────────────────────────────┘
```

---

## 4. Layout general

### 4.1 Sidebar izquierdo

El sidebar contiene:

* Selector de workspace.
* Buscador global.
* Navegación principal.
* Favoritos.
* Recientes.
* Acción para crear nueva página.

#### Anchura

```css
--sidebar-width: 280px;
--sidebar-collapsed-width: 72px;
```

#### Normas

* El sidebar debe ser persistente en escritorio.
* En pantallas pequeñas debe convertirse en drawer lateral.
* El elemento activo debe tener fondo suave y color primario.
* Los grupos largos deben poder plegarse.
* Los textos largos deben truncarse con `ellipsis`.
* El botón de nueva página debe permanecer accesible al final del sidebar.

#### Ejemplo de navegación

```text
Evolutio-Correos

Buscar...

Diarios
Pizarras
Tarjetas de memorización
Vista de Grafo
Lista de páginas

Favoritos
- Dashboard Diseño Lógicos
- Doc Arquitectura de referencia
- Dashboard DDAs
- Work Orders

Recientes
- Incidencias falta credenciales...
- DDA Huella v1.0.0
- Documentación Pipelines RDS...
```

---

### 4.2 Top bar

La top bar contiene:

* Pestañas abiertas.
* Botón de nueva pestaña.
* Acciones globales.
* Buscador rápido.
* Notificaciones.
* Ayuda.
* Avatar de usuario.

#### Normas

* Altura recomendada: `56px`.
* Debe permanecer sticky.
* Las pestañas deben mostrar estado activo.
* Las pestañas inactivas deben tener menor contraste.
* El cierre de pestaña solo aparece en hover o foco.
* Las acciones globales deben estar alineadas a la derecha.

#### Estados de pestañas

| Estado   | Apariencia                         |
| -------- | ---------------------------------- |
| Activa   | Borde inferior o contorno primario |
| Inactiva | Fondo neutro suave                 |
| Hover    | Fondo ligeramente elevado          |
| Focus    | Anillo visible de foco             |
| Error    | Indicador rojo discreto            |

---

### 4.3 Área principal

El área principal muestra contenido de diario, páginas y bloques.

#### Anchura máxima

```css
--content-max-width: 1040px;
```

#### Normas

* El contenido debe estar centrado.
* Debe haber margen lateral suficiente.
* El ancho de línea de texto no debe ser excesivo.
* Los bloques complejos deben usar tarjetas.
* Las acciones de bloque deben estar disponibles en hover.

---

## 5. Sistema de color

El diseño usa un tema claro profesional con acento azul/índigo.

### 5.1 Tokens base

```css
:root {
  --color-primary: #2563EB;
  --color-primary-hover: #1D4ED8;
  --color-primary-container: #EEF4FF;
  --color-on-primary: #FFFFFF;

  --color-secondary: #4F46E5;
  --color-accent: #0EA5E9;

  --color-background: #F8FAFC;
  --color-surface: #FFFFFF;
  --color-surface-subtle: #F1F5F9;
  --color-surface-elevated: #FFFFFF;

  --color-border: #E2E8F0;
  --color-border-strong: #CBD5E1;

  --color-text-primary: #0F172A;
  --color-text-secondary: #475569;
  --color-text-muted: #64748B;
  --color-text-disabled: #94A3B8;

  --color-link: #2563EB;
  --color-link-hover: #1D4ED8;

  --color-success: #16A34A;
  --color-warning: #D97706;
  --color-danger: #DC2626;
  --color-info: #0284C7;
}
```

### 5.2 Uso semántico

| Uso              | Color                    |
| ---------------- | ------------------------ |
| Acción primaria  | `--color-primary`        |
| Enlaces internos | `--color-link`           |
| Fondo general    | `--color-background`     |
| Tarjetas         | `--color-surface`        |
| Metadatos        | `--color-surface-subtle` |
| Texto principal  | `--color-text-primary`   |
| Texto secundario | `--color-text-secondary` |
| Bordes           | `--color-border`         |
| Errores          | `--color-danger`         |
| Avisos           | `--color-warning`        |

### 5.3 Reglas

* El color primario no debe usarse para todo.
* Los enlaces internos deben ser azules.
* Las etiquetas deben usar fondos suaves, no colores saturados.
* Las alertas deben reservar rojo, naranja y verde para significado real.
* El texto principal debe tener contraste suficiente sobre fondo claro.
* No usar azul para texto que no sea interactivo, salvo etiquetas o estados concretos.

---

## 6. Tipografía

### 6.1 Fuente recomendada

```css
font-family: Inter, Roboto, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
```

### 6.2 Escala tipográfica

| Token         | Tamaño | Peso | Uso                |
| ------------- | -----: | ---: | ------------------ |
| `display-sm`  |   36px |  700 | Fechas principales |
| `headline-md` |   28px |  700 | Títulos de página  |
| `title-lg`    |   20px |  600 | Títulos de bloque  |
| `title-md`    |   16px |  600 | Secciones          |
| `body-lg`     |   16px |  400 | Texto principal    |
| `body-md`     |   14px |  400 | Texto de interfaz  |
| `body-sm`     |   13px |  400 | Sidebar, metadatos |
| `label-md`    |   12px |  600 | Badges, etiquetas  |
| `caption`     |   11px |  500 | Ayudas, shortcuts  |

### 6.3 Normas

* Las fechas de diario deben ser grandes y escaneables.
* Los títulos de bloques deben tener peso medio-alto.
* El sidebar debe usar tamaño menor pero legible.
* Los metadatos deben tener menor peso visual.
* No usar más de 4 tamaños diferentes en una misma vista.
* La altura de línea mínima para texto largo debe ser `1.5`.

---

## 7. Espaciado

### 7.1 Escala

```css
--space-1: 4px;
--space-2: 8px;
--space-3: 12px;
--space-4: 16px;
--space-5: 20px;
--space-6: 24px;
--space-8: 32px;
--space-10: 40px;
--space-12: 48px;
```

### 7.2 Normas

* Padding interno de tarjetas: `20px` o `24px`.
* Separación entre bloques de diario: `48px`.
* Separación entre título de fecha y contenido: `16px`.
* Separación entre elementos del sidebar: `4px` a `8px`.
* Separación entre grupos del sidebar: `24px`.
* Separación entre pestañas: `4px`.

---

## 8. Bordes, radios y sombras

### 8.1 Radios

```css
--radius-sm: 6px;
--radius-md: 10px;
--radius-lg: 14px;
--radius-xl: 18px;
--radius-pill: 999px;
```

### 8.2 Uso

| Componente | Radio           |
| ---------- | --------------- |
| Botones    | `--radius-md`   |
| Pestañas   | `--radius-md`   |
| Inputs     | `--radius-md`   |
| Tarjetas   | `--radius-lg`   |
| Badges     | `--radius-pill` |
| Avatar     | `--radius-pill` |

### 8.3 Sombras

```css
--shadow-sm: 0 1px 2px rgba(15, 23, 42, 0.06);
--shadow-md: 0 8px 24px rgba(15, 23, 42, 0.08);
--shadow-lg: 0 16px 40px rgba(15, 23, 42, 0.12);
```

### 8.4 Normas

* Usar sombras suaves solo para elevar elementos.
* No usar sombras oscuras o dramáticas.
* Las tarjetas normales pueden usar borde sin sombra.
* Menús flotantes, tooltips y modales sí deben usar sombra.

---

## 9. Componentes principales

## 9.1 SidebarItem

Representa una entrada de navegación.

### Estructura

```text
[icono] Etiqueta [contador opcional]
```

### Estados

| Estado   | Apariencia                                |
| -------- | ----------------------------------------- |
| Default  | Texto secundario                          |
| Hover    | Fondo `surface-subtle`                    |
| Active   | Fondo `primary-container`, texto primario |
| Focus    | Anillo primario                           |
| Disabled | Texto disabled                            |

### Norma

El elemento activo debe ser evidente sin depender solo del color.

---

## 9.2 SearchInput

Campo de búsqueda global.

### Requisitos

* Placeholder: `Buscar`.
* Shortcut visible: `Ctrl K` o `⌘ K`.
* Icono de lupa.
* Soporte de teclado.
* Debe abrir command palette si el usuario pulsa el shortcut.

---

## 9.3 Tabs

Las pestañas permiten mantener páginas abiertas.

### Requisitos

* Icono de tipo de página.
* Título truncado.
* Botón de cierre.
* Estado activo.
* Botón `+` para nueva pestaña.

### Norma

Las pestañas no deben ocupar más de una línea. Si hay muchas pestañas, debe aparecer scroll horizontal o menú de overflow.

---

## 9.4 JournalDateHeader

Cabecera de una entrada de diario.

### Estructura

```text
[icono calendario] 26-05-2026 ------------------- [acciones]
```

### Normas

* La fecha debe ser el elemento más visible.
* El icono debe ayudar a identificar que es una entrada diaria.
* La línea horizontal debe separar visualmente los días.
* Las acciones deben permanecer discretas.

---

## 9.5 Link interno

Representa una referencia a otra página.

### Apariencia

* Color azul.
* Hover con subrayado.
* Focus con anillo visible.
* Puede aparecer como texto o como chip.

### Ejemplo

```text
DDA Huella v1.0.0
```

### Norma

Todo enlace interno debe ser reconocible como navegable.

---

## 9.6 Tag / Badge

Representa metadatos o tipos de contenido.

### Ejemplos

```text
Huella
Incidencia
TODO
DDA
AWS
Jenkins
```

### Apariencia

```css
.badge {
  border-radius: 999px;
  background: var(--color-primary-container);
  color: var(--color-primary);
  font-size: 12px;
  font-weight: 600;
  padding: 2px 8px;
}
```

### Norma

Los badges deben ser compactos y no competir con los títulos.

---

## 9.7 ReferenceCard

Tarjeta para referencias vinculadas.

### Estructura

```text
[icono documento]

Título de referencia

dda-relacionada: DDA Huella v1.0.0
type: Incidencia
fecha-creacion: 26-05-2026

[abrir] [más acciones]
```

### Normas

* Debe mostrar claramente el título.
* Los metadatos deben estar alineados.
* Las acciones deben aparecer a la derecha.
* Debe soportar hover.
* Debe poder abrirse con teclado.
* El card completo puede ser clicable, pero las acciones internas deben tener foco independiente.

---

## 9.8 ContentCard

Tarjeta para bloques largos de documentación.

### Uso

* Documentación técnica.
* Resúmenes.
* Listas de pipelines.
* Referencias técnicas.
* Checklists.

### Normas

* Usar borde sutil.
* Mantener padding generoso.
* Separar secciones internas.
* No mezclar demasiados estilos en una misma tarjeta.
* Las listas deben tener indentación clara.

---

## 9.9 ChecklistItem

Representa tareas tipo TODO.

### Estructura

```text
[checkbox] [TODO] Añadir diagramas de flujo detallados
```

### Estados

| Estado      | Apariencia                       |
| ----------- | -------------------------------- |
| Pendiente   | Checkbox vacío                   |
| Completado  | Checkbox marcado, texto atenuado |
| Bloqueado   | Badge warning                    |
| Prioritario | Indicador discreto               |

---

## 9.10 FloatingHelpButton

Botón flotante de ayuda.

### Normas

* Posición: esquina inferior derecha.
* Debe ser accesible por teclado.
* Debe tener tooltip.
* No debe tapar contenido importante.
* Debe respetar safe area en pantallas pequeñas.

---

## 10. Diseño de la vista “Diarios”

### 10.1 Objetivo

La vista de diarios permite ver actividad diaria, páginas relacionadas, incidencias, documentación y decisiones.

### 10.2 Estructura recomendada

```text
26-05-2026
  Huella   DDA Huella v1.0.0
    Incidencia problemas Usuarios Sigua
    Incidencias problemas versión BBDD Aurora Postgres
    Incidencias falta credenciales huella_app y huella_own

  1 Referencia vinculada
    [ReferenceCard]

25-05-2026
  DDA Huella v1.0.0
    Incidencia problemas Usuarios Sigua
    Incidencias problemas versión BBDD Aurora Postgres

  Documentación Pipelines Correos creada
    [ContentCard]
```

### 10.3 Normas

* Cada día debe estar claramente separado.
* Las referencias vinculadas deben aparecer debajo del contenido que las genera.
* El número de referencias debe ser visible.
* Las tarjetas deben mejorar la lectura, no reemplazar la estructura de bloques.
* Los enlaces a páginas deben conservar color y estilo consistente.
* Los bloques anidados deben tener línea vertical suave o indentación clara.

---

## 11. UX de edición

### 11.1 Edición inline

La aplicación debe permitir edición directa de bloques.

#### Normas

* El bloque en edición debe mostrar borde o fondo suave.
* El cursor debe aparecer en la posición esperada.
* Las acciones de bloque deben aparecer en hover o foco.
* Debe existir undo/redo.
* Se debe guardar automáticamente.

### 11.2 Bloques

Cada línea editable es un bloque.

Cada bloque debe soportar:

* Texto.
* Enlaces internos.
* Etiquetas.
* Tareas.
* Referencias.
* Propiedades.
* Bloques hijos.
* Acciones contextuales.

### 11.3 Acciones de bloque

Acciones mínimas:

* Crear bloque hijo.
* Mover arriba.
* Mover abajo.
* Convertir en tarea.
* Copiar enlace al bloque.
* Eliminar.
* Abrir menú contextual.

---

## 12. UX de navegación

### 12.1 Navegación por teclado

Debe soportarse:

| Acción                   | Shortcut      |
| ------------------------ | ------------- |
| Buscar / command palette | `Ctrl + K`    |
| Nueva página             | `Ctrl + N`    |
| Nueva pestaña            | `Ctrl + T`    |
| Cerrar pestaña           | `Ctrl + W`    |
| Ir a diario de hoy       | `G` luego `D` |
| Expandir bloque          | `Tab`         |
| Contraer bloque          | `Shift + Tab` |
| Guardar manual           | `Ctrl + S`    |
| Abrir grafo              | `Ctrl + G`    |

### 12.2 Breadcrumbs

Las páginas profundas deben mostrar contexto:

```text
Workspace / DDA Huella / Incidencias / Credenciales
```

### 12.3 Referencias bidireccionales

Las referencias vinculadas deben poder:

* Filtrarse.
* Contraerse.
* Ordenarse.
* Abrirse en nueva pestaña.
* Abrirse en panel lateral.
* Copiarse como enlace.

---

## 13. Accesibilidad

### 13.1 Contraste

Requisitos mínimos:

* Texto normal: contraste mínimo 4.5:1.
* Texto grande: contraste mínimo 3:1.
* Iconos interactivos: contraste mínimo 3:1.
* Estados no deben depender solo del color.

### 13.2 Foco visible

Todos los elementos interactivos deben tener foco visible.

```css
:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
}
```

### 13.3 Navegación con teclado

Debe ser posible usar la aplicación sin ratón.

Elementos obligatorios navegables:

* Sidebar.
* Buscador.
* Pestañas.
* Bloques.
* Enlaces.
* Tarjetas.
* Botones.
* Menús contextuales.
* Diálogos.

### 13.4 Lectores de pantalla

Normas:

* Los iconos decorativos deben usar `aria-hidden="true"`.
* Los botones solo con icono deben tener `aria-label`.
* Las listas deben usar semántica real.
* Las pestañas deben usar roles adecuados.
* Los menús deben anunciar estado abierto/cerrado.

---

## 14. Estados de interacción

Todos los componentes interactivos deben definir estos estados:

| Estado   | Obligatorio    |
| -------- | -------------- |
| Default  | Sí             |
| Hover    | Sí             |
| Active   | Sí             |
| Focus    | Sí             |
| Disabled | Sí             |
| Loading  | Cuando aplique |
| Error    | Cuando aplique |
| Empty    | Cuando aplique |

### 14.1 Hover

Debe ser sutil.

```css
.component:hover {
  background: var(--color-surface-subtle);
}
```

### 14.2 Active

Debe ser claro y estable.

```css
.component[data-active="true"] {
  background: var(--color-primary-container);
  color: var(--color-primary);
}
```

### 14.3 Loading

Usar skeletons, no spinners globales salvo en carga inicial.

---

## 15. Empty states

Toda vista sin contenido debe mostrar:

* Icono ilustrativo discreto.
* Título claro.
* Descripción breve.
* Acción principal.

### Ejemplo

```text
No hay referencias vinculadas

Esta página todavía no está enlazada desde otras notas.
Crea enlaces usando [[Nombre de página]].
```

---

## 16. Responsive design

### 16.1 Breakpoints

```css
--breakpoint-sm: 640px;
--breakpoint-md: 768px;
--breakpoint-lg: 1024px;
--breakpoint-xl: 1280px;
--breakpoint-2xl: 1536px;
```

### 16.2 Escritorio

* Sidebar fijo.
* Top bar fija.
* Contenido centrado.
* Acciones visibles.

### 16.3 Tablet

* Sidebar plegable.
* Contenido ocupa más anchura.
* Pestañas con scroll horizontal.

### 16.4 Móvil

* Sidebar como drawer.
* Top bar simplificada.
* Pestañas opcionales o sustituidas por selector.
* Acciones secundarias dentro de menú.
* Bloques con mayor altura táctil.

---

## 17. Modo oscuro

Debe existir modo oscuro desde el inicio.

### Tokens mínimos

```css
[data-theme="dark"] {
  --color-background: #020617;
  --color-surface: #0F172A;
  --color-surface-subtle: #1E293B;
  --color-surface-elevated: #111827;

  --color-border: #334155;
  --color-border-strong: #475569;

  --color-text-primary: #F8FAFC;
  --color-text-secondary: #CBD5E1;
  --color-text-muted: #94A3B8;
  --color-text-disabled: #64748B;

  --color-primary: #60A5FA;
  --color-primary-hover: #93C5FD;
  --color-primary-container: #172554;

  --color-link: #93C5FD;
  --color-link-hover: #BFDBFE;
}
```

### Normas

* No invertir colores automáticamente.
* Revisar contraste de enlaces.
* Reducir sombras y usar bordes/superficies.
* Evitar fondos completamente negros salvo en zonas específicas.

---

## 18. Iconografía

### Librería recomendada

Usar una librería consistente como:

* Lucide Icons.
* Material Symbols.
* Heroicons.

### Normas

* Tamaño estándar: `18px`.
* Sidebar: `18px`.
* Acciones compactas: `16px`.
* Iconos de tarjetas: `24px`.
* No mezclar estilos outline y filled sin criterio.
* Todo icono interactivo necesita tooltip o etiqueta accesible.

---

## 19. Animación y movimiento

### Principio

La animación debe explicar cambios de estado, no decorar.

### Duraciones

```css
--motion-fast: 120ms;
--motion-normal: 180ms;
--motion-slow: 240ms;
```

### Easing

```css
--ease-standard: cubic-bezier(0.2, 0, 0, 1);
```

### Normas

* Hover: 120ms.
* Apertura de menú: 180ms.
* Drawer/sidebar: 240ms.
* Respetar `prefers-reduced-motion`.
* No animar texto largo ni contenido de lectura intensiva.

---

## 20. Componentes técnicos recomendados

Si se usa React + Tailwind:

### Stack recomendado

* React.
* TypeScript.
* Tailwind CSS v4.
* Radix UI o shadcn/ui para componentes base.
* Lucide React para iconos.
* TanStack Query para datos remotos.
* Zustand o Jotai para estado local.
* ProseMirror, TipTap o Lexical para editor avanzado.
* React Aria si se requiere máxima accesibilidad.

### Normas

* Los componentes deben ser accesibles por defecto.
* No implementar menús, dialogs o tooltips desde cero si existe una primitiva accesible.
* Separar tokens de diseño de componentes.
* Evitar estilos hardcodeados fuera de tokens.

---

## 21. Tokens de diseño mínimos

```css
:root {
  /* Layout */
  --sidebar-width: 280px;
  --topbar-height: 56px;
  --content-max-width: 1040px;

  /* Spacing */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-10: 40px;
  --space-12: 48px;

  /* Radius */
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 14px;
  --radius-xl: 18px;
  --radius-pill: 999px;

  /* Shadows */
  --shadow-sm: 0 1px 2px rgba(15, 23, 42, 0.06);
  --shadow-md: 0 8px 24px rgba(15, 23, 42, 0.08);
  --shadow-lg: 0 16px 40px rgba(15, 23, 42, 0.12);

  /* Motion */
  --motion-fast: 120ms;
  --motion-normal: 180ms;
  --motion-slow: 240ms;
}
```

---

## 22. Criterios de aceptación visual

Una pantalla se considera válida si cumple:

* El sidebar activo se identifica en menos de 1 segundo.
* La fecha principal del diario es el elemento más visible.
* Los enlaces internos se distinguen claramente del texto normal.
* Las tarjetas tienen bordes, espaciado y jerarquía consistentes.
* Las acciones secundarias no saturan la vista.
* La interfaz funciona con teclado.
* El contraste cumple accesibilidad mínima.
* El contenido principal no supera el ancho máximo definido.
* No hay textos cortados sin tooltip o alternativa.
* No hay iconos sin significado accesible.
* El diseño funciona en modo claro y oscuro.

---

## 23. Anti-patrones prohibidos

No se permite:

* Usar azul para todo el texto.
* Ocultar el foco de teclado.
* Crear tarjetas sin jerarquía interna.
* Usar sombras fuertes en exceso.
* Mezclar varias librerías de iconos.
* Depender solo del color para comunicar estado.
* Usar fuentes distintas sin justificación.
* Tener menús contextuales inaccesibles.
* Romper la navegación al abrir una página.
* Poner acciones críticas solo en hover.
* Crear bloques demasiado densos sin espaciado.

---

## 24. Checklist para diseñadores

Antes de entregar una pantalla, comprobar:

* [ ] ¿La jerarquía visual es clara?
* [ ] ¿Se entiende dónde está el usuario?
* [ ] ¿Se distingue contenido de navegación?
* [ ] ¿Los enlaces parecen enlaces?
* [ ] ¿Las tarjetas tienen estructura clara?
* [ ] ¿Los metadatos están visualmente separados?
* [ ] ¿Hay estados hover/focus/active?
* [ ] ¿El diseño soporta modo oscuro?
* [ ] ¿El diseño es usable en 1024px?
* [ ] ¿El diseño tiene empty states?
* [ ] ¿El diseño tiene loading states?
* [ ] ¿El diseño cumple contraste mínimo?

---

## 25. Checklist para maquetadores

Antes de implementar una pantalla, comprobar:

* [ ] Todos los colores salen de tokens.
* [ ] Todos los espaciados salen de escala.
* [ ] No hay valores mágicos repetidos.
* [ ] Los componentes tienen estados completos.
* [ ] Los botones de icono tienen `aria-label`.
* [ ] Los menús son navegables con teclado.
* [ ] Las pestañas usan roles accesibles.
* [ ] El sidebar es responsive.
* [ ] El contenido principal respeta `--content-max-width`.
* [ ] Las listas largas tienen truncado correcto.
* [ ] Los tooltips no sustituyen información crítica.
* [ ] Se respeta `prefers-reduced-motion`.

---

## 26. Definición final de calidad

La aplicación debe sentirse como una herramienta profesional de conocimiento técnico: limpia, rápida, fiable, cómoda para lectura prolongada y eficiente para usuarios avanzados.

El diseño no debe parecer una copia visual de Logseq. Debe conservar su potencia conceptual —bloques, diarios, enlaces bidireccionales, referencias y grafo— pero con una presentación más pulida, corporativa y accesible.

Este documento te sirve como base normativa para diseñadores, maquetadores y desarrolladores. Para llevarlo a implementación, lo siguiente sería convertir estos tokens en `theme.css` o en configuración de Tailwind v4.
