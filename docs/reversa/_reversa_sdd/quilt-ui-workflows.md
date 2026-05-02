# Quilt — UI, Flujos de Trabajo y Experiencia de Usuario

> Propuesta de diseño UX para Quilt: el compañero cognitivo
> Fecha: 2026-05-02

---

## 0. Panorama Competitivo — Lo que ya existe

| Herramienta | Paradigma | AI | Fuerza | Debilidad |
|-------------|-----------|----|--------|-----------|
| **Obsidian** | Archivos locales, plugins | Comunidad (mínimo) | Privacidad, extensibilidad | AI es afterthought |
| **Logseq** | Outliner, DB graph | No | Queries, open-source | UI densa |
| **Notion** | Bloques, database, equipo | Notion AI (asistente) | Todo-en-uno | Pesado, no local-first |
| **Tana** | Supertags, live queries | Agentes en meetings | AI nativo, ejecuta trabajo | Nuevo pivot a meetings |
| **Heptabase** | Whiteboard, cards, AI Tutor | Tutor estructurado | Aprendizaje visual, investigación | Nicho educativo |
| **Capacities** | Objetos, no archivos | No | Modelo mental limpio | Pocas integraciones |

**Patrón común:** Todos añaden AI como una capa sobre su interfaz existente. Nadie ha rediseñado la experiencia desde cero para un mundo AI-first.

---

## 1. Principios de Diseño Quilt

```
1. AI no es un botón. Es el aire que respira la interfaz.
2. Cada vista tiene un "modo AI" y un "modo humano".
3. El agente pregunta, no solo responde.
4. La UI revela lo invisible: patrones, conexiones, evolución.
5. El contexto lo es todo. El agente siempre sabe dónde estás.
```

---

## 2. Las 5 Vistas del Quilt

```
┌─────────────────────────────────────────────────┐
│                  QUILT                            │
│                                                   │
│  ┌─────────┬─────────┬─────────┬─────────┬─────┐ │
│  │ DAILY   │  GRAPH  │  FOCUS  │  QUERY  │ AGENT│ │
│  │ JOURNAL │  VIEW   │  MODE   │  BUILDER│ ROOM │ │
│  └─────────┴─────────┴─────────┴─────────┴─────┘ │
└─────────────────────────────────────────────────┘
```

### 2.1 DAILY JOURNAL — Donde empieza cada día

**Lo que hace diferente:** No es una página en blanco. Es un **briefing cognitivo matutino** generado por tu agente.

```
┌──────────────────────────────────────────────────┐
│ 📅 Viernes, 2 de Mayo 2026                     │
│ ────────────────────────────────────────────────│
│                                                  │
│ 📊 TU BRIEFING MATUTINO                         │
│ ┌──────────────────────────────────────────────┐│
│ │ 🧠 Cognitive Pulse                           ││
│ │ Las últimas 72h has explorado: Rust, MCP,    ││
│ │ CRDTs. Tu modelo "Rust para sistemas" se ha  ││
│ │ reforzado con 3 nuevas evidencias.           ││
│ │                                              ││
│ │ ⚡ 2 emergencias detectadas:                 ││
│ │ → "Quilt architecture" (14 notas nuevas)     ││
│ │ → "MCP protocol limits" (8 notas)            ││
│ │                                              ││
│ │ 🔗 El Serendipity Engine encontró:           ││
│ │ "Tu nota sobre actor model y tu nota sobre   ││
│ │  MCP comparten una estructura idéntica.      ││
│ │  ¿Es MCP un actor model aplicado a AI?"      ││
│ └──────────────────────────────────────────────┘│
│                                                  │
│ 🔔 NECESITAN TU ATENCIÓN                        │
│ ⚠ "Rust async patterns" — 14 meses sin revisar  │
│ ⚠ "PKM market analysis" — datos de 2024         │
│                                                  │
│ ✍️ ┌─────────────────────────────────────────┐  │
│    │ Escribe aquí...                          │  │
│    └─────────────────────────────────────────┘  │
│                                                  │
│ 📋 TASKS PARA HOY                               │
│ ☐ Revisar modelo "Rust para sistemas"           │
│ ☐ Conectar nota MCP con actor model             │
│ ☐ Actualizar PKM market analysis                │
│                                                  │
│ [Agent] Buenos días. Veo que estás explorando   │
│ MCP intensamente. ¿Quieres que prepare un       │
│ debate sobre MCP vs REST para agentes AI?       │
│ [Si] [No, luego]                                │
│                                                  │
│ ─── TUS NOTAS DE HOY ─────────────────────────  │
│                                                  │
│ 09:30 Continuando con la implementación...      │
│      de Quilt. El modelo de propiedades...      │
│                                                  │
│ 11:45 Reunión con equipo. Decidimos...          │
│      que petgraph se pospone a Fase 3...        │
│                                                  │
│ (La página del journal se llena normalmente)    │
│                                                  │
└──────────────────────────────────────────────────┘
```

**El briefing matutino NO es un chat.** Es un dashboard informativo generado automáticamente. El agente te muestra:
- Tu pulso cognitivo (qué has pensado)
- Conexiones que no has visto
- Decay alerts (qué se está oxidando)
- Emergencias (patrones nuevos)

Solo después de eso, el espacio de escritura normal.

### 2.2 GRAPH VIEW — El espejo de tu mente

No es el típico grafo de bolitas de Obsidian. Es un **Cognitive Map** vivo.

```
┌──────────────────────────────────────────────────┐
│ 🧠 COGNITIVE MAP                     [3D] [2D]  │
│ ──────────────────────────────────────────────── │
│                                                  │
│           ○ Rust                                 │
│          /│\                                     │
│         / │ \                                    │
│   ○ MCP ○───○ WASM                              │
│      \   │   /                                   │
│       \  │  /                                    │
│        ○─○─○                                     │
│       Quilt    ○ TypeScript (débil)              │
│                                                  │
│ ───────────────────────────────────────────────  │
│ LEYENDA:                                         │
│ 🟢 Área densa (muchas conexiones)                │
│ 🟡 Frontera (mencionas pero no profundizas)     │
│ 🔴 Gap (rodeas pero nunca atacas directamente)   │
│ ⚪ Abandonado (sin actividad en 6+ meses)         │
│                                                  │
│ ┌──────────────┐  ┌────────────────────────────┐ │
│ │ TIMELINE     │  │ INSIGHTS                   │ │
│ │              │  │                            │ │
│ │ Feb ────○    │  │ "Rust" es tu nodo más      │ │
│ │ Mar ──○─○   │  │ denso (47 conexiones).     │ │
│ │ Abr ○○○○○○  │  │ Pero "TypeScript" perdió   │ │
│ │ May ○○○     │  │ 80% actividad desde Marzo.  │ │
│ │              │  │                            │ │
│ │ Desliza para │  │ "MCP" y "WASM" están      │ │
│ │ ver evolución│  │ convergiendo rápidamente.  │ │
│ └──────────────┘  │ ¿Nueva área de expertise?  │ │
│                   └────────────────────────────┘ │
│                                                  │
│ [Agent] Veo que TypeScript está decayendo en     │
│ tu grafo mientras Rust crece. ¿Es un cambio      │
│ consciente de stack o abandono por falta de      │
│ tiempo? [Es consciente] [Falta de tiempo]        │
└──────────────────────────────────────────────────┘
```

**Diferencias con Obsidian Graph View:**
- No es estático. Es un **organismo vivo** que muestra evolución temporal.
- Los colores codifican **estado cognitivo** (densidad, frontier, gap, abandono), no solo tipo de nodo.
- El agente **comenta** lo que ve en tiempo real.
- Tiene timeline para ver cómo cambió tu conocimiento en el tiempo.

### 2.3 FOCUS MODE — El editor que piensa contigo

El editor de bloques tradicional pero con **presencia AI lateral**.

```
┌──────────────────────────────────────────────────┐
│ ✍️ Rust async patterns                    [FOCUS]│
│ ──────────────────────────────────────────────── │
│                                                  │
│ # Rust Async Patterns                           │
│                                                  │
│ Tokio es el runtime dominante en Rust...         │
│ █                                                  │
│                                                  │
│ ## async/await                                   │
│ Las funciones async devuelven Future...          │
│                                                  │
│ ## Spawning                                      │
│ tokio::spawn permite ejecutar tareas...           │
│                                                  │
│ ═══════════════════ ═══════════════════════════  │
│  TU NOTA              AGENT (modo Focus)         │
│                      ┌──────────────────────────┐│
│                      │ 📊 NOTA ACTUAL            ││
│                      │ 217 palabras, 3 headings  ││
│                      │ 0 referencias a otras     ││
│                      │ notas de Quilt            ││
│                      │                          ││
│                      │ 🔗 CONEXIONES SUGERIDAS  ││
│                      │ → [[Actor Model]] (89%)  ││
│                      │ → [[Pin en Rust]] (72%)  ││
│                      │ → [[MCP Concurrency]]     ││
│                      │                          ││
│                      │ ⚠ DETECTADO               ││
│                      │ Dices "dominante" sin     ││
│                      │ citar fuente. Tu nota     ││
│                      │ [[Rust Survey 2025]]      ││
│                      │ tiene los datos.          ││
│                      │                          ││
│                      │ 💡 SUGERENCIA             ││
│                      │ Tu nota actual repite     ││
│                      │ conceptos de [[Rust       ││
│                      │ Runtime Comparison]].     ││
│                      │ ¿Mergear o referenciar?   ││
│                      └──────────────────────────┘│
│ ═══════════════════ ═══════════════════════════  │
│                                                  │
│ [Agent] Detecté que tu nota no referencia        │
│ ninguna otra nota de Quilt. ¿Quieres que         │
│ sugiera conexiones?                              │
│ [Auto-link] [Mostrar sugerencias] [Ignorar]      │
└──────────────────────────────────────────────────┘
```

**El agente en modo Focus:**
- **No interrumpe.** Está en un panel lateral.
- Sugiere conexiones en tiempo real mientras escribes.
- Detecta afirmaciones sin respaldo ("dices X sin citar fuente Y").
- Detecta duplicación de contenido ("esto ya lo escribiste en otra nota").
- Ofrece auto-linking con un clic.

### 2.4 QUERY BUILDER — Donde las preguntas se hacen código

No es un input de búsqueda. Es un **constructor visual de queries** con feedback inmediato.

```
┌──────────────────────────────────────────────────┐
│ 🔍 QUERY BUILDER                                 │
│ ──────────────────────────────────────────────── │
│                                                  │
│ ┌── FILTROS ──────────────────────────────────┐ │
│ │ tasks AND priority(high) AND deadline(before │ │
│ │ 2026-05-10) AND NOT property(status, done)   │ │
│ └──────────────────────────────────────────────┘ │
│                                                  │
│ ┌── VISUAL ───────────────────────────────────┐ │
│ │ [tasks] ──AND── [priority=high]             │ │
│ │    │                   │                     │ │
│ │    └──AND── [deadline<2026-05-10]           │ │
│ │                   │                          │ │
│ │                   └──NOT── [status=done]     │ │
│ └──────────────────────────────────────────────┘ │
│                                                  │
│ ┌── RESULTADOS (23 bloques) ──────────────────┐ │
│ │                                              │ │
│ │ 📄 Proyecto Quilt                            │ │
│ │  ☐ HIGH Implementar sistema de propiedades   │ │
│ │    📅 due: 2026-05-05  🏷️ rust, schema       │ │
│ │                                              │ │
│ │  ☐ HIGH Migrar de DataScript a SQLite        │ │
│ │    📅 due: 2026-05-08  🏷️ database, rust     │ │
│ │                                              │ │
│ │ 📄 Proyecto MCP                               │ │
│ │  ☐ HIGH Definir tools para agentes           │ │
│ │    📅 due: 2026-05-04  🏷️ mcp, api           │ │
│ └──────────────────────────────────────────────┘ │
│                                                  │
│ [Agent] Esta query devuelve 23 tareas de alta    │
│ prioridad con deadline próxima. ¿Quieres que     │
│ cree una página "Sprint Mayo 2026" con estas     │
│ tareas organizadas?                              │
│ [Crear página] [Agregar a Daily] [Ignorar]       │
└──────────────────────────────────────────────────┘
```

### 2.5 AGENT ROOM — Tu mesa redonda de pensamiento

Espacio dedicado a interactuar con tus agentes AI.

```
┌──────────────────────────────────────────────────┐
│ 🎭 AGENT ROOM — "¿Debería lanzar Quilt OSS?"    │
│ ──────────────────────────────────────────────── │
│                                                  │
│ ┌─────────────────────┐ ┌──────────────────────┐ │
│ │ 🏛️ SKEPTIC           │ │ 🔬 SCIENTIST          │ │
│ │                     │ │                      │ │
│ │ "Tu nota de Marzo   │ │ "Datos de tu grafo:  │ │
│ │  dice que el mercado│ │  3 de 5 herramientas  │ │
│ │  OSS PKM está       │ │  PKM exitosas usan   │ │
│ │  saturado. ¿Hay     │ │  open-core, no OSS   │ │
│ │  espacio para otro?"│ │  puro. Evidencia en  │ │
│ │                     │ │  [[PKM Business]]."   │ │
│ └─────────────────────┘ └──────────────────────┘ │
│ ┌─────────────────────┐ ┌──────────────────────┐ │
│ │ 🎨 CREATIVE          │ │ 💼 PRAGMATIST         │ │
│ │                     │ │                      │ │
│ │ "¿Y si el OSS no es │ │ "Tu nota [[Runway]]  │ │
│ │  el producto sino   │ │  muestra 8 meses.    │ │
│ │  el canal? Tus notas│ │  SaaS-first da       │ │
│ │  sobre 'developer   │ │  revenue desde mes 1.│ │
│ │  advocacy' sugieren │ │  OSS no factura."    │ │
│ │  que los plugins son│ │                      │ │
│ │  growth, no revenue."│ │                      │ │
│ └─────────────────────┘ └──────────────────────┘ │
│ ┌─────────────────────┐ ┌──────────────────────┐ │
│ │ 📜 HISTORIAN         │ │ 🌍 SYSTEMS THINKER    │ │
│ │                     │ │                      │ │
│ │ "Escribiste en 2023 │ │ "Si es SaaS-first,   │ │
│ │  'Obsidian ganó por │ │  dependes de cloud.  │ │
│ │   plugins'. También │ │  Tu audiencia valora │ │
│ │   en 2024 'el OSS   │ │  self-hosting. ¿Esto │ │
│ │   sin comunidad es  │ │  crea un conflicto   │ │
│ │   un repositorio    │ │  con tu propuesta de │ │
│ │   muerto'."         │ │  valor?"             │ │
│ └─────────────────────┘ └──────────────────────┘ │
│                                                  │
│ ═══════════════════════════════════════════════  │
│ SÍNTESIS DEL AGENTE                              │
│                                                  │
│ Todos tus agentes convergen en: "Open-core con   │
│ SaaS para sync/colaboración". Recomiendan leer:  │
│ → [[PKM Business Models]] (tu nota de Marzo)     │
│ → [[Runway Q2 2026]] (tus finanzas)              │
│ → [[Developer Advocacy Strategy]] (tu growth)    │
│                                                  │
│ ¿Quieres que formalice esto como decision log?   │
│ [Crear ADR] [Agregar a Daily] [Ignorar]          │
└──────────────────────────────────────────────────┘
```

---

## 3. Flujos de Trabajo Principales

### 3.1 FLUJO MATUTINO (5 minutos)

```
08:00 Abrir Quilt → Daily Journal
      ↓
08:01 Leer Briefing Matutino (generado por el agente)
      → Cognitive Pulse: qué has pensado últimas 72h
      → Emergencias detectadas
      → Conexiones Serendipity
      → Decay Alerts
      ↓
08:03 Revisar Tasks sugeridas por el agente
      → Aceptar, rechazar o modificar
      ↓
08:05 Empezar a escribir
      → El agente sugiere conexiones mientras escribes
      → El agente detecta si repites contenido
```

### 3.2 FLUJO DE INVESTIGACIÓN (30-60 minutos)

```
1. ABRIR TEMA en Agent Room
   → "Quiero entender CRDTs para sync en Quilt"
   
2. EL AGENTE PREPARA EL TERRENO
   → Busca en tu grafo qué ya sabes del tema
   → Identifica gaps ("Sabes de CRDTs pero nunca has
     escrito sobre conflict resolution")
   → Sugiere lecturas de tus propias notas
   
3. INVESTIGAS (web, PDFs, papers)
   → Cada fuente que añades se anota con su referencia
   → El agente extrae claims y los vincula a tu grafo
   
4. EL AGENTE SINTETIZA
   → "Tus 7 fuentes convergen en 3 principios clave:
     1. CRDTs garantizan convergencia sin coordinator
     2. Loro es la implementación más madura en Rust
     3. El trade-off es memoria vs latencia"
   
5. ESCRIBES TU SÍNTESIS
   → El agente sugiere estructura basada en tus fuentes
   → Cada claim se vincula automáticamente a su fuente
```

### 3.3 FLUJO DE DECISIÓN (15-30 minutos)

```
1. PLANTEAS UNA DECISIÓN
   → "¿Deberíamos usar petgraph o no en Quilt?"
   
2. EL AGENTE RECOPILA EVIDENCIA
   → Busca pros/contras en tus notas existentes
   → Identifica stakeholders implícitos
   → Encuentra decisiones similares pasadas
   
3. AGENT ROOM DEBATE
   → 6 agentes debaten desde sus perspectivas
   → Cada uno cita EVIDENCIA DE TU PROPIO GRAFO
   → El agente sintetiza convergencias
   
4. DECIDES Y DOCUMENTAS
   → Un clic: "Crear ADR de esta decisión"
   → El agente redacta el ADR con:
     - Contexto
     - Opciones consideradas
     - Evidencia de tu grafo
     - Decisión final
     - Consecuencias
```

### 3.4 FLUJO DE REVISIÓN SEMANAL (20 minutos)

```
Domingo 18:00 → "Quilt, haz mi revisión semanal"

EL AGENTE GENERA:
┌────────────────────────────────────────────┐
│ 📊 WEEKLY REVIEW — 28 Abr - 2 May 2026    │
│                                            │
│ 🧠 COGNITIVE SUMMARY                       │
│ • 47 notas nuevas (↑23% vs semana anterior)│
│ • 3 modelos mentales evolucionaron         │
│ • 1 modelo fue refutado por nueva evidencia│
│ • Top 3 áreas: Rust (32%), MCP (28%),      │
│   Arquitectura (18%)                       │
│                                            │
│ 🔗 CONEXIONES CREADAS                      │
│ • 28 nuevos links entre notas              │
│ • 12 sugeridos por el agente (aceptaste 9) │
│ • 3 descubiertos por Serendipity           │
│                                            │
│ ⚠ ATENCIÓN REQUERIDA                       │
│ • 5 notas sin actualizar en >12 meses      │
│ • 2 modelos mentales sin revisar en >3 mes │
│ • 1 decisión pendiente sin documentar      │
│                                            │
│ 🎯 SUGERENCIAS PARA PRÓXIMA SEMANA         │
│ • Profundizar en "CRDT conflict resolution"│
│ • Conectar "Rust" con "WASM" (gap detectado)│
│ • Revisar modelo "MCP architecture"        │
└────────────────────────────────────────────┘
```

---

## 4. El Botón Mágico: AUTO-ORGANIZE

Una de las funciones más potentes y simples:

```
Situación: Tienes 50 notas desordenadas sobre un tema.

[Auto-Organize]

El agente:
1. Agrupa notas por similitud semántica
2. Detecta notas huérfanas y sugiere padres
3. Detecta duplicados y ofrece merge
4. Sugiere estructura de Table of Contents
5. Crea página índice con links a todas las notas

Resultado: 50 notas caóticas → estructura navegable en segundos.
```

---

## 5. El Modo Nocturno: AGENT WHISPER

Cuando cierras Quilt por la noche, el agente sigue trabajando:

```
23:00 Cierras Quilt
      ↓
      El agente ejecuta tareas nocturnas:
      • Re-indexa el grafo de conocimiento
      • Ejecuta Serendipity Engine (busca conexiones)
      • Detecta Decay (notas que necesitan actualización)
      • Detecta Emergence (nuevos patrones)
      • Prepara el Briefing Matutino
      • Si hay nuevos datos externos (RSS, papers):
        → Los procesa y sugiere lecturas relevantes
      ↓
08:00 Abres Quilt → Briefing listo
```

---

## 6. Diferenciación Radical: Lo que NADIE más hace

| Capacidad | Obsidian | Notion | Tana | Heptabase | **Quilt** |
|-----------|----------|--------|------|-----------|-----------|
| **Briefing matutino** | No | No | No | No | **Sí** |
| **Agent Room (debate)** | No | No | No | No | **Sí** |
| **Cognitive Map (vivo)** | Estático | No | No | No | **Sí** |
| **Serendipity notifications** | No | No | No | No | **Sí** |
| **Auto-Organize** | No | No | No | No | **Sí** |
| **Background Agent (nocturno)** | No | No | No | No | **Sí** |
| **Weekly Review automática** | No | No | No | No | **Sí** |
| **Decay Monitor** | No | No | No | No | **Sí** |
| **Evidence checking en editor** | No | No | No | No | **Sí** |
| **Agent memory (aprende de ti)** | No | No | Parcial | No | **Sí** |
| **Knowledge Evolution timeline** | No | No | No | No | **Sí** |

---

## 7. Arquitectura Técnica de la UI

```
┌──────────────────────────────────────────────────┐
│               Tauri Shell (Rust)                  │
│  ┌──────────────────────────────────────────────┐│
│  │         Leptos/Yew WASM UI                    ││
│  │                                               ││
│  │  ┌─────────┐ ┌────────┐ ┌─────────────────┐ ││
│  │  │ Journal │ │ Graph  │ │ Agent Room      │ ││
│  │  │ View    │ │ View   │ │ (Web Component) │ ││
│  │  └─────────┘ └────────┘ └─────────────────┘ ││
│  │                                               ││
│  │  ┌──────────────────────────────────────────┐││
│  │  │        Agent Panel (ubiquitous)          │││
│  │  │  - Sidebar en Focus Mode                 │││
│  │  │  - Embedded en Graph View                │││
│  │  │  - Full-screen en Agent Room             │││
│  │  │  - Compact en Journal                    │││
│  │  └──────────────────────────────────────────┘││
│  └──────────────────────────────────────────────┘│
│                                                   │
│  ┌──────────────────────────────────────────────┐│
│  │         Rust Backend Services                 ││
│  │  - Block/Page Service                         ││
│  │  - Query Engine                               ││
│  │  - Agent Orchestrator (background tasks)      ││
│  │  - MCP Server (AI agent interface)           ││
│  │  - Serendipity Engine                         ││
│  │  - Knowledge Evolution Tracker                ││
│  │  - SQLite + FTS5                              ││
│  └──────────────────────────────────────────────┘│
└──────────────────────────────────────────────────┘
```

---

## 8. Onboarding: La Primera Experiencia

```
Día 1 — Abres Quilt por primera vez:

┌──────────────────────────────────────────────────┐
│                                                  │
│         🧵 Bienvenido a Quilt                    │
│                                                  │
│  Soy tu compañero cognitivo. No soy un chat.    │
│  No soy un asistente. Pienso contigo.           │
│                                                  │
│  Para empezar, necesito conocerte:              │
│                                                  │
│  ┌────────────────────────────────────────────┐ │
│  │ ¿En qué estás trabajando ahora?            │ │
│  │                                            │ │
│  │ [________________________________________] │ │
│  │                                            │ │
│  │ ¿Qué te gustaría entender mejor?           │ │
│  │                                            │ │
│  │ [________________________________________] │ │
│  │                                            │ │
│  │ ¿Tienes notas existentes para importar?    │ │
│  │                                            │ │
│  │ [Importar de Obsidian] [Importar Markdown] │ │
│  │ [Empezar desde cero]                       │ │
│  └────────────────────────────────────────────┘ │
│                                                  │
│  (Después de responder)                         │
│                                                  │
│  ┌────────────────────────────────────────────┐ │
│  │ 🧠 He creado tu Cognitive Seed:            │ │
│  │                                            │ │
│  │ Áreas iniciales:                           │ │
│  │ • [Tu proyecto actual]                     │ │
│  │ • [Lo que quieres entender]                │ │
│  │                                            │ │
│  │ Cada mañana te prepararé un briefing.      │ │
│  │ Cada noche, procesaré lo que escribiste.   │ │
│  │ Cuando quieras, abre el Agent Room y       │ │
│  │ pensemos juntos.                           │ │
│  │                                            │ │
│  │ [Empezar a escribir]                       │ │
│  └────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

---

## 9. Resumen: La Experiencia Quilt

```
┌──────────────────────────────────────────────────┐
│                                                  │
│  MAÑANA          DÍA               NOCHE        │
│  ───────         ───               ─────        │
│  Briefing    →   Escribes      →   El agente    │
│  matutino        investigas        procesa       │
│  (3 min)         decides           tu día        │
│                  (tu flujo)        (background)  │
│  El agente       El agente                       │
│  te pone         te acompaña     El agente       │
│  al día          en silencio     prepara         │
│                                  mañana          │
│                                                  │
│  ─────────────────────────────────────────────  │
│                                                  │
│  SEMANALMENTE: Review automática (Domingo)      │
│  CUANDO QUIERAS: Agent Room (debate)            │
│  SIEMPRE: Decay Monitor, Serendipity            │
│                                                  │
└──────────────────────────────────────────────────┘
```

**Quilt no te hace más productivo. Te hace pensar mejor.**
