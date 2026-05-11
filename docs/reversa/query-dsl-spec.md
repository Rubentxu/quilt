# Quilt Query DSL Specification

This document provides the formal grammar specification for the Quilt Query DSL, a domain-specific language for querying the knowledge graph. The DSL is used to filter, sort, and transform block data stored in SQLite.

## Overview

The Query DSL is a Polish notation (S-expression) language where queries are enclosed in parentheses with an operator first, followed by arguments. Queries can be nested to create complex boolean logic.

**Parser**: Pest-based parser with pre-processing for bare integers.
**AST**: `QueryExpr` enum with variants for each operator.
**Executor**: Generates SQL WHERE clauses from the AST.

---

## Grammar

### EBNF Notation

```ebnf
query       = S-expr | page-ref | self-ref ;
S-expr      = "(" operator (expr | value)* ")" ;
operator    = and | or | not | between | property | task | priority
            | page | tags | full-text-search | sample | table
            | sort-by | exists | missing | namespace ;
expr        = S-expr | page-ref | self-ref ;
value       = string | integer | date | time-helper | boolean ;
```

### Terminal Rules

```ebnf
string          = "\"" quoted-string "\"" ;
quoted-string   = (!"\"" ANY)* ;
integer         = digit+ ;
date            = integer "-" integer "-" integer ;   -- YYYY-MM-DD
time-helper     = ["-"] integer ("d" | "w" | "m" | "y" | "h" | "n") ;
boolean         = "true" | "false" ;
page-ref        = "[[" page-name "]]" ;
self-ref        = "self" ;
page-name       = (!"]]" ANY)* ;
task-marker     = "now" | "later" | "todo" | "done" | "cancelled" ;
priority-level  = "a" | "b" | "c" ;
direction       = "asc" | "desc" ;
property-op     = "!=" | ">=" | "<=" | ">" | "<" | "contains" | "between" ;
```

### Non-Terminal Rules

#### Boolean Operators

```ebnf
and     = "(" "and" expr+ ")" ;
or      = "(" "or" expr+ ")" ;
not     = "(" "not" expr ")" ;
```

#### Comparison Operators

```ebnf
between = "(" "between" value value ")" ;
```

#### Property Operators

```ebnf
property = "(" "property" string property-op? value+ ")" ;
```

#### Filter Operators

```ebnf
task     = "(" "task" task-marker+ ")" ;
priority = "(" "priority" priority-level+ ")" ;
page     = "(" "page" string ")" ;
tags     = "(" "tags" string ")" ;
full-text-search = "(" "full-text-search" string ")" ;
sample   = "(" "sample" integer ")" ;
```

#### Output Operators

```ebnf
table   = "(" "table" expr+ ")" ;
sort-by = "(" "sort-by" (string | integer) direction? expr ")" ;
```

#### Existence Operators

```ebnf
exists    = "(" "exists" string ")" ;
missing   = "(" "missing" string ")" ;
namespace = "(" "namespace" string ")" ;
```

---

## Operators

### Boolean Operators

#### `and`

Combines multiple expressions with logical AND.

**Grammar**: `and = "(" "and" expr+ ")"`

**Syntax**:
```
(and (task todo) (priority a))
(and (page "Home") (property "author" "John"))
(and (or (task todo) (task later)) (not (priority c)))
```

**AST**: `QueryExpr::And(Vec<QueryExpr>)`

**SQL Generation**: All inner expressions are AND-combined in the WHERE clause.

---

#### `or`

Combines multiple expressions with logical OR.

**Grammar**: `or = "(" "or" expr+ ")"`

**Syntax**:
```
(or (task todo) (task done))
(or (priority a) (priority b))
(and (page "X") (or (task todo) (task later)))
```

**AST**: `QueryExpr::Or(Vec<QueryExpr>)`

**SQL Generation**: All inner expressions are OR-combined in the WHERE clause.

---

#### `not`

Negates an expression.

**Grammar**: `not = "(" "not" expr ")"`

**Syntax**:
```
(not (task done))
(not (and (task todo) (priority c)))
```

**AST**: `QueryExpr::Not(Box<QueryExpr>)`

**SQL Generation**: Inner expression is negated with SQL NOT.

---

### Comparison Operators

#### `between`

Filters values within a range (inclusive).

**Grammar**: `between = "(" "between" value value ")"`

**Default Field**: `created_at` when no field is specified.

**Syntax**:
```
(between "100" "200")
(between "2024-01-01" "2024-12-31")
(between -30d 7d)
```

**Pre-processing**: Bare integers are normalized to quoted strings before parsing.

**AST**: `QueryExpr::Between { field: String, start: QueryValue, end: QueryValue }`

**SQL Generation**: Generates `field BETWEEN ? AND ?` with parameters.

**Backward Compatibility**: The `field` parameter defaults to `"created_at"`. Existing queries using quoted integers continue to work identically.

---

### Property Operators

#### `property`

Filters blocks by JSON property values.

**Grammar**: `property = "(" "property" string property-op? value+ ")"`

**Operators**:

| Operator | Description | Example |
|----------|-------------|---------|
| (default) | Equality | `(property "author" "John")` |
| `!=` | Not equals | `(property "status" != "done")` |
| `>` | Greater than | `(property "count" > 10)` |
| `<` | Less than | `(property "count" < 100)` |
| `>=` | Greater than or equal | `(property "count" >= 10)` |
| `<=` | Less than or equal | `(property "count" <= 100)` |
| `contains` | String contains | `(property "name" contains "test")` |
| `between` | Range (requires 2 values) | `(property "count" between 10 100)` |

**Syntax**:
```
(property "author" "John")
(property "count" > 10)
(property "name" contains "search-term")
(property "count" between 10 100)
```

**AST**: `QueryExpr::Property { key: String, op: PropertyOp, value: QueryValue, value2: Option<QueryValue> }`

**SQL Generation**: Uses `json_extract(properties, '$.key')` for property access.

---

### Filter Operators

#### `task`

Filters blocks by task marker.

**Grammar**: `task = "(" "task" task-marker+ ")"`

**Markers**: `now`, `later`, `todo`, `done`, `cancelled`

**Syntax**:
```
(task todo)
(task todo done)
(task now later cancelled)
```

**AST**: `QueryExpr::Task(Vec<String>)`

**SQL Generation**: `marker IN (?, ?, ...)` with marker values.

---

#### `priority`

Filters blocks by priority level.

**Grammar**: `priority = "(" "priority" priority-level+ ")"`

**Levels**: `a`, `b`, `c`

**Syntax**:
```
(priority a)
(priority a b)
```

**AST**: `QueryExpr::Priority(Vec<String>)`

**SQL Generation**: `priority COLLATE NOCASE IN (?, ?)` with priority values.

---

#### `page`

Filters blocks by page name.

**Grammar**: `page = "(" "page" string ")"`

**Syntax**:
```
(page "Home")
(page "Projects/Rust")
```

**AST**: `QueryExpr::Page(String)`

**SQL Generation**: `page = ?` with page name parameter.

---

#### `tags`

Filters blocks by tag.

**Grammar**: `tags = "(" "tags" string ")"`

**Syntax**:
```
(tags "rust")
(tags "documentation")
```

**AST**: `QueryExpr::Tags(String)`

**SQL Generation**: `tags IN (?)` or similar tag matching.

---

#### `full-text-search`

Performs full-text content search.

**Grammar**: `full-text-search = "(" "full-text-search" string ")"`

**Syntax**:
```
(full-text-search "query string")
```

**AST**: `QueryExpr::BlockContent(String)`

**SQL Generation**: `content MATCH ?` with FTS5 query.

---

#### `sample`

Returns a random sample of N results.

**Grammar**: `sample = "(" "sample" integer ")"`

**Constraints**: Count must be 1–1000.

**Syntax**:
```
(sample 10)
(sample 100)
```

**AST**: `QueryExpr::Sample(usize)`

**SQL Generation**: `ORDER BY RANDOM() LIMIT ?`

---

### Output Operators

#### `table`

Returns structured tabular output from inner expressions.

**Grammar**: `table = "(" "table" expr+ ")"`

**Syntax**:
```
(table (task todo))
(table (task todo) (priority a))
```

**AST**: `QueryExpr::Table(Vec<QueryExpr>)`

**SQL Generation**: Combines inner expressions as AND in WHERE clause, selects structured columns.

---

#### `sort-by`

Sorts results by a field.

**Grammar**: `sort-by = "(" "sort-by" (string | integer) direction? expr ")"`

**Directions**: `asc` (default), `desc`

**Syntax**:
```
(sort-by "created_at" asc (task todo))
(sort-by "priority" desc (task done))
(sort-by 0 desc (sample 10))
```

**AST**: `QueryExpr::SortBy { field: String, direction: SortDirection, inner: Box<QueryExpr> }`

**SQL Generation**: `ORDER BY field direction` preceding the inner WHERE clause.

---

### Existence Operators

#### `exists`

Filters blocks where a property exists.

**Grammar**: `exists = "(" "exists" string ")"`

**Syntax**:
```
(exists "due_date")
(exists "completed_at")
```

**AST**: `QueryExpr::Exists(String)`

**SQL Generation**: `json_extract(properties, '$.key') IS NOT NULL`

---

#### `missing`

Filters blocks where a property is missing.

**Grammar**: `missing = "(" "missing" string ")"`

**Syntax**:
```
(missing "completed_at")
(missing "due_date")
```

**AST**: `QueryExpr::Missing(String)`

**SQL Generation**: `json_extract(properties, '$.key') IS NULL`

---

#### `namespace`

Filters blocks by namespace (page hierarchy).

**Grammar**: `namespace = "(" "namespace" string ")"`

**Syntax**:
```
(namespace "projects")
(namespace "proyecto/alfa")
```

**AST**: `QueryExpr::Namespace(String)`

**SQL Generation**: Correlated subquery joining pages with `block/namespace` match.

---

### Reference Operators

#### `page-ref`

References a page by name.

**Grammar**: `page-ref = "[[" page-name "]]"`

**Syntax**:
```
[[Page Name]]
[[Projects/Rust]]
```

**AST**: `QueryExpr::PageRef(String)`

---

#### `self`

References the current block.

**Grammar**: `self-ref = "self"`

**Syntax**:
```
self
```

**AST**: `QueryExpr::SelfRef`

---

## Values

### String

Quoted text values.

**Syntax**: `"text content"`

**Examples**:
```
"John"
"2024-01-01"
"Hello World"
```

---

### Integer

Numeric integers.

**Syntax**: `digit+`

**Examples**:
```
42
100
0
```

**Note**: Bare integers in `between` expressions are pre-processed to quoted form for parser compatibility.

---

### Date

ISO 8601 date format.

**Syntax**: `YYYY-MM-DD`

**Examples**:
```
2024-01-15
2024-12-31
```

---

### Time Helper

Relative time offsets.

**Syntax**: `[-]integer(d|w|m|y|h|n)`

| Unit | Description |
|------|-------------|
| `d` | Days |
| `w` | Weeks |
| `m` | Months |
| `y` | Years |
| `h` | Hours |
| `n` | Minutes |

**Examples**:
```
-7d      (7 days ago)
30d      (30 days from now)
-2w      (2 weeks ago)
3m       (3 months from now)
```

**Direction Correction**: Positive offsets yield future dates, negative offsets yield past dates. `to_date` uses addition: `base + Duration::days(*n)`.

---

### Boolean

Logical boolean values.

**Syntax**: `true` | `false`

**Examples**:
```
true
false
```

---

## Error Handling

### ParseError Variants

```rust
enum ParseError {
    Syntax {
        msg: String,
        line: usize,
        col: usize,
        hint: Option<String>,
    },
    Invalid(String),
}
```

### Syntax Errors

Generated by Pest when the query string violates grammar rules.

| Scenario | Message | Location | Hint |
|----------|---------|----------|------|
| Unclosed parenthesis | "expected ')'" | Line 1, column N | "did you forget to close a parenthesis?" |
| Unknown operator | "unexpected token" | Line 1, column N | "valid operators: and, or, not, task, ..." |
| Invalid structure | Grammar rule mismatch | Line:Col | "check your parentheses balance" |

**Example**:
```rust
// Input: "(task todo"
let result = parser.parse("(task todo");
assert!(matches!(result, Err(ParseError::Syntax { line: 1, col: 11, .. })));
```

### Semantic Errors

Generated by the validator when the query is grammatically valid but semantically invalid.

| Rule | Invalid Input | Error Message |
|------|---------------|--------------|
| between-arity | `(between 100)` | "between requires 2 arguments" |
| property-between-arity | `(property "x" between 1)` | "between operator requires start AND end value" |
| sample-range | `(sample 0)` | "sample count must be 1–1000" |
| sample-range | `(sample 1001)` | "sample count must be 1–1000" |
| sort-by-arity | `(sort-by)` | "sort-by requires a field and an expression" |
| exists-arity | `(exists)` | "exists requires an argument" |
| namespace-arity | `(namespace)` | "namespace requires an argument" |
| table-arity | `(table)` | "table requires at least one expression" |

**Example**:
```rust
let result = parser.parse("(sample 0)");
assert!(matches!(result, Err(ParseError::Invalid(msg)) if msg.contains("sample count must be 1–1000")));
```

### Empty Query

```rust
let result = parser.parse("");
assert!(matches!(result, Err(ParseError::Invalid(msg)) if msg.contains("Empty query")));
```

---

## Backward Compatibility

### Pre-processor for Bare Integers

The parser includes a pre-processor that normalizes bare integers in `between` expressions:

```rust
// Input: "(between 100 200)"
// Output: "(between \"100\" \"200\")"
```

This fixes a Pest limitation with unquoted integers in sequence while maintaining full backward compatibility.

### Existing Query Behavior

| Change | Backward Compatible |
|--------|---------------------|
| `between` field defaults to `"created_at"` | Yes — existing queries omit explicit field |
| Bare integer normalization | Yes — quoted form continues to work |
| New `Table` variant | Yes — no removal of existing variants |
| New `SortBy` variant | Yes — no removal of existing variants |
| New `Exists`/`Missing` variants | Yes — no removal of existing variants |
| New `Namespace` variant | Yes — no removal of existing variants |

### TimeOffset Direction Fix

The `TimeOffset::to_date` method was corrected to use addition instead of subtraction:

```rust
// Before (incorrect):
base - Duration::days(*n)

// After (correct):
base + Duration::days(*n)
```

This affects relative time queries like `-7d`:

| Expression | Before (incorrect) | After (correct) |
|------------|-------------------|-----------------|
| `-7d` from 2024-01-15 | 2024-01-22 (future) | 2024-01-08 (past) |

**Rollback**: If this change proves disruptive, it can be reverted independently via `git revert` while keeping other hardening changes.

---

## Examples

### Simple Filters

```
(task todo)
```
Returns all blocks marked as TODO.

```
(priority a)
```
Returns all blocks with priority A.

```
(page "Home")
```
Returns all blocks on the Home page.

---

### Compound Queries

```
(and (task todo) (priority a))
```
Returns blocks that are both TODO and priority A.

```
(or (task todo) (task done))
```
Returns blocks that are either TODO or DONE.

```
(and (page "Projects") (not (task cancelled)))
```
Returns blocks on the Projects page that are not cancelled.

---

### Property Queries

```
(property "author" "John")
```
Returns blocks where the author property equals "John".

```
(property "count" > 10)
```
Returns blocks where count is greater than 10.

```
(property "tags" contains "rust")
```
Returns blocks where tags contain "rust".

```
(property "date" between "2024-01-01" "2024-06-30")
```
Returns blocks where date is between the given range.

---

### Time-Based Queries

```
(between -30d 0d)
```
Returns blocks created in the last 30 days.

```
(between -1y 0d)
```
Returns blocks created in the last year.

---

### Existence Queries

```
(exists "due_date")
```
Returns blocks that have a due_date property.

```
(missing "completed_at")
```
Returns blocks that do not have a completed_at property.

---

### Output Formatting

```
(table (task todo))
```
Returns structured table output for TODO blocks.

```
(sort-by "created_at" desc (task todo))
```
Returns TODO blocks sorted by creation date, newest first.

---

### Namespace Queries

```
(namespace "projects")
```
Returns blocks in the projects namespace.

```
(and (namespace "projects") (task todo))
```
Returns TODO blocks in the projects namespace.

---

### Full-Text Search

```
(full-text-search "rust implementation")
```
Returns blocks containing "rust implementation" in content.

---

### Page References

```
[[Another Page]]
```
Returns all blocks that reference "Another Page".

```
self
```
Returns the current block (used in block context).
