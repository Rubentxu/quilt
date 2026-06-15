# Properties as the Source of Truth for Block Semantics

Quilt treats Pages, Blocks, Properties, and Links as the primitive building blocks of the knowledge graph. We decided that semantic, structural, and projection-related block data belongs in typed Properties, not in duplicated special-purpose Block columns such as `marker`, `priority`, `scheduled`, `deadline`, or visual `block_type`. This follows Quilt's Logseq-inspired model and supports moldable projections: the same Block can be shown as text, task, media, query, card, calendar item, or annotation by reading its Properties in context.

This intentionally moves away from column-first marker/rendering logic. Columns may remain temporarily as indexes or compatibility shims during migration, but they are not the canonical source. Slash commands become property setters (`/DOING` writes `status:: doing`, `/Scheduled` writes `scheduled:: <date>`, `/Video` writes media properties), and renderers become projections over Properties.

Slash commands are **Property Presets**, not render commands. A slash command writes a named bundle of Properties to the current Block; the application then selects a Block Projection by reading those Properties. For example, `/TODO` writes task-related Properties, while `/Video` writes media-related Properties. Neither command directly selects a renderer.

Block Projections activate through declarative Projection Contracts: a projection declares the required Properties and predicates needed to match a Block. For example, TaskProjection requires `type:: task` plus a compatible `status::`, while VideoProjection requires `type:: media` and `media-type:: video`. A generic property such as `status::` does not activate TaskProjection by itself.

Projection is compositional, not replacement-based. Every block keeps a universal Base Block Surface made of rich text, links, children, and visible properties. Specialized projections add visual layers or richer interpretations on top of that base surface. A task may add a checkbox and status controls; media may add a preview/embed; neither removes the block text or properties.

Projection layout must not introduce domain primitives outside Quilt's building blocks. Any visual placement or behavior such as inline display, panel display, system visibility, preview/embed behavior, badges, indicators, or editability is expressed as Property Configuration inside the graph. The UI may render those configurations as slots or regions, but the source of truth remains Pages, Blocks, Properties, and Links.

Projection Contracts are declarative by default and may include a small code escape hatch for cases that cannot be expressed as a simple property predicate. The declarative part remains inspectable by agents and UI tooling; the code guard is reserved for genuinely complex constraints.

Properties carry independent visibility and mutability metadata. Visibility controls where a property appears (`inline`, `panel`, `system`, `hidden`), while mutability controls whether it can be edited from the UI. In block edit mode, the block's properties are visible as part of the editing surface, but immutable properties render read-only and are changed only by system rules, importers, or explicit privileged operations.

Blocks may carry a visual projection preference through `projection:: auto`. `auto` means the projection engine selects the best compatible projection using Projection Contracts and falls back to DefaultProjection when none match. Missing `projection` is equivalent to `projection:: auto` for ordinary blocks, while Property Presets materialize `projection:: auto` explicitly. A specific projection value can express an explicit preference, but it does not bypass the target projection's contract.

Applying a Property Preset produces a non-destructive Property Patch. The patch preserves the block's text, children, and unrelated existing properties. When a patch touches an existing property, a per-property/context merge policy decides the result: set-if-missing, overwrite, append, union, reject-on-conflict, or ask-on-conflict. Contract-incompatible collisions must fail or ask for confirmation instead of silently converting or deleting information.

Block text remains Block Content, not a property. User input is canonicalized into block content plus derived/applied properties: Markdown, paste, slash commands, pickers, API, and MCP all feed the same canonical model. For example, `# Title` keeps `Title` as editable block content while deriving heading-related properties; a pasted video link keeps the URL/text in the block content while deriving link/media properties. Slash commands are automation over this same canonicalization path.

Markdown canonicalization V1 derives properties from specific syntax while preserving all editable text as Block Content:

| Input syntax | Derived properties |
| --- | --- |
| `#`, `##`, `###` | `heading-level:: 1|2|3`, `block-role:: heading` |
| `[Text](url)` | `link:: url`, `link-label:: Text`, `link-kind:: external|media|page-ref` |
| `![](url)` | `embed-url:: url`, `embed-kind:: image|video|unknown` |
| `[[Page]]` | `page-ref:: Page` |
| `((block-id))` | `block-ref:: block-id` |
| `TODO` / `DOING` / `DONE` prefix | `type:: task`, `status:: todo|doing|done`, `projection:: auto` |

Markdown outside those V1 patterns remains plain Block Content.

Properties derived from Markdown are system-owned and immutable from the UI. They change by editing the Block Content that produced them, not by direct property editing. This avoids two competing sources of truth: for example, `# Title` owns `heading-level:: 1`; changing the heading level means editing the Markdown to `## Title`.

Derived properties are materialized on the block as queryable system properties. They carry source ownership such as `derived-from:: block-content`, regenerate when Block Content changes, and are removed when their source syntax disappears. They are visible only in system/debug views unless another property configuration exposes them differently.

Derived and applied properties may coexist. Conflicts are resolved by Projection Contracts, not by automatic deletion or conversion. If contracts are incompatible or ambiguous, Quilt preserves Block Content and all properties, materializes the projection conflict as system properties (`projection-conflict`, `projection-conflict-reason`, `projection-conflict-candidates`), and falls back to the Generic Text Visualization: rich text, links, children, and visible properties. Generic text visualization is mandatory and universal.

V1 merge policies:

| Property | Policy |
| --- | --- |
| `type` | set-if-missing; ask-on-conflict when an incompatible type already exists |
| `projection` | set-if-missing; never overwrite a distinct explicit projection preference |
| `status` | overwrite when `type:: task` or no type exists; ask-on-conflict for incompatible roles such as media/query |
| `focus` | single-value; set/overwrite as the current focus marker |
| `tags` | union |
| `scheduled` / `deadline` | overwrite only when the user explicitly chooses a date |
| `media-type` | ask-on-conflict when changing image/video/etc. |
| `source-url` | ask-on-conflict when a different URL already exists |
| content/text/children | never touched by a preset |

`/NOW` uses `focus:: now`, not `status:: doing`. Focus marks current attention and remains independent from task state.

Incremental refactor order:

1. Extend the domain property model to support property configuration: visibility, mutability, derived ownership, and merge policy.
2. Introduce the input canonicalization pipeline that turns Markdown, slash commands, paste, API, and MCP input into Block Content plus Property Patches.
3. Rewrite slash commands as non-destructive Property Presets/Patches.
4. Add declarative Projection Contracts with mandatory fallback to Generic Text Visualization.
5. Update UI surfaces so the panel/editor read Property Configuration instead of hardcoded block fields such as `marker`, `scheduled`, or `deadline`.

First implementable slice: **Property Configuration Domain Model**. Scope is intentionally limited to domain concepts and tests: visibility, mutability, derived ownership (`derived-from`), merge policy, and metadata for `system`/`immutable` properties. This slice must not rewrite slash commands, projection resolution, or UI rendering yet.
