# Quilt MCP API Specification

> **Version**: 1.0
> **Protocol**: Model Context Protocol (MCP) 2024-11-05
> **Transport**: JSON-RPC 2.0

This document describes the MCP tools, resources, and notifications provided by the Quilt MCP server for AI agent integration.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Tools](#2-tools)
3. [Resources](#3-resources)
4. [Notifications](#4-notifications)
5. [Error Handling](#5-error-handling)
6. [Examples](#6-examples)

---

## 1. Overview

### Server Info

```json
{
  "name": "quilt-mcp",
  "version": "0.1.0"
}
```

### Capabilities

| Capability | Value | Description |
|------------|-------|-------------|
| `tools.list_changed` | `false` | Tool list is static |
| `resources.subscribe` | `false` | No resource subscriptions |
| `resources.list_changed` | `false` | Resource list is static |

### Protocol Version

`2024-11-05`

---

## 2. Tools

All tools follow the MCP tool call format:

**Request:**
```json
{
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": { ... }
  }
}
```

**Response:**
```json
{
  "content": [
    {
      "type": "text",
      "text": "{ ... result JSON ... }"
    }
  ],
  "is_error": false
}
```

---

### 2.1 `logseq_query` — Execute DSL Query

Execute a Logseq DSL query against the knowledge graph.

**Arguments:**
```json
{
  "dsl": "string",      // Required: DSL query string
  "limit": 100          // Optional: Max results (default: 100)
}
```

**DSL Query Examples:**

| Query | Description |
|-------|-------------|
| `(task todo)` | All TODO tasks |
| `(task done)` | All completed tasks |
| `(priority a)` | Priority A items |
| `(page "Page Name")` | Blocks on specific page |
| `(and (task todo) (priority a))` | TODO tasks with priority A |
| `(between :created_at "2024-01-01" "2024-12-31")` | Created in date range |

**Response:**
```json
{
  "count": 2,
  "blocks": [
    {
      "id": "uuid",
      "page_id": "uuid",
      "parent_id": null,
      "order": 1.0,
      "level": 1,
      "content": "Block content",
      "marker": "Todo",
      "priority": "A",
      "collapsed": false,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ],
  "sql": "SELECT b.* ..."
}
```

---

### 2.2 `logseq_create_block` — Create Block

Create a new content block on a page (auto-creates page if needed).

**Arguments:**
```json
{
  "page_name": "string",     // Required: Page name
  "content": "string",       // Required: Block content (markdown)
  "parent_id": "uuid",       // Optional: Parent block UUID
  "marker": "string"         // Optional: Task marker (now, later, todo, done, cancelled)
}
```

**Response:**
```json
{
  "id": "uuid",
  "page_id": "uuid",
  "page_name": "Page Name",
  "content": "Block content",
  "parent_id": null,
  "marker": "Todo"
}
```

---

### 2.3 `logseq_search` — Full-Text Search

Search blocks and pages using FTS5.

**Arguments:**
```json
{
  "query": "string",         // Required: Search query
  "limit": 50                // Optional: Max results (default: 50)
}
```

**Response:**
```json
{
  "count": 2,
  "results": [
    {
      "block_id": "uuid",
      "page_name": "Page Name",
      "snippet": "...matched content...",
      "score": -1.5
    }
  ]
}
```

---

### 2.4 `logseq_get_block_tree` — Get Block with Children

Get a block and all its descendants recursively.

**Arguments:**
```json
{
  "block_id": "uuid"         // Required: Block UUID (root)
}
```

**Response:**
```json
{
  "block": { ... },
  "children": [
    {
      "id": "uuid",
      "page_id": "uuid",
      "parent_id": "parent-uuid",
      "order": 1.0,
      "level": 2,
      "content": "Child content",
      "marker": null,
      "priority": null,
      "collapsed": false,
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ],
  "children_count": 1
}
```

---

### 2.5 `logseq_get_page_blocks` — Get Page Blocks

Get all blocks on a page.

**Arguments:**
```json
{
  "page_name": "string",     // Required: Page name
  "format": "markdown"       // Optional: "markdown" or "org" (default: "markdown")
}
```

**Response:**
```json
{
  "page": {
    "id": "uuid",
    "name": "Page Name"
  },
  "blocks": [ ... ],
  "count": 3
}
```

---

### 2.6 `logseq_list_pages` — List All Pages

List all pages in the knowledge graph.

**Arguments:**
```json
{}
```

**Response:**
```json
{
  "count": 5,
  "pages": [
    {
      "id": "uuid",
      "name": "Page Name",
      "title": null,
      "journal": false
    }
  ]
}
```

---

### 2.7 `logseq_get_journal` — Get/Create Journal Page

Get or create a journal page for a specific date.

**Arguments:**
```json
{
  "date": "YYYY-MM-DD"       // Required: Date string
}
```

**Response:**
```json
{
  "page": {
    "id": "uuid",
    "name": "2024-05-07",
    "journal_day": 20260507
  },
  "blocks": [ ... ],
  "block_count": 0
}
```

---

### 2.8 `logseq_create_task` — Create Task

Create a task (block with TODO marker) on a page.

**Arguments:**
```json
{
  "page_name": "string",     // Required: Page name
  "content": "string",       // Required: Task content
  "deadline": "YYYY-MM-DD", // Optional: Deadline date
  "priority": "string"       // Optional: Priority (a, b, or c)
}
```

**Response:**
```json
{
  "id": "uuid",
  "page_name": "Page Name",
  "content": "Task content",
  "marker": "TODO"
}
```

---

### 2.9 `logseq_link_blocks` — Link Blocks

Create a reference (link) from one block to another.

**Arguments:**
```json
{
  "source_id": "uuid",        // Required: Source block UUID
  "target_id": "uuid"         // Required: Target block UUID
}
```

**Response:**
```json
{
  "status": "linked",
  "source_id": "uuid",
  "target_id": "uuid"
}
```

---

### 2.10 `logseq_get_backlinks` — Get Backlinks

Get all blocks that reference a specific block.

**Arguments:**
```json
{
  "block_id": "uuid"          // Required: Target block UUID
}
```

**Response:**
```json
{
  "block_id": "uuid",
  "backlinks": [ ... ],
  "count": 2
}
```

---

### 2.11 `logseq_delete_block` — Delete Block

Soft-delete a block (moves to recycle bin).

**Arguments:**
```json
{
  "block_id": "uuid"          // Required: Block UUID
}
```

**Response:**
```json
{
  "status": "deleted",
  "block_id": "uuid"
}
```

---

### 2.12 `logseq_restore_block` — Restore Block

Restore a soft-deleted block from the recycle bin.

**Arguments:**
```json
{
  "block_id": "uuid"          // Required: Block UUID to restore
}
```

**Response:**
```json
{
  "status": "restored",
  "block_id": "uuid"
}
```

---

### 2.13 `logseq_recycle_bin` — List Recycle Bin

List all soft-deleted blocks.

**Arguments:**
```json
{}
```

**Response:**
```json
{
  "recycle_bin": [
    {
      "block_id": "uuid",
      "page_id": "uuid",
      "content": "Deleted content",
      "deleted_at": "2024-01-01T00:00:00Z"
    }
  ],
  "count": 1
}
```

---

### 2.14 `logseq_orphan_pages` — List Orphan Pages

List pages with no blocks.

**Arguments:**
```json
{}
```

**Response:**
```json
{
  "orphan_pages": [
    {
      "page_id": "uuid",
      "name": "Orphan Page",
      "title": null,
      "journal": false
    }
  ],
  "count": 1
}
```

---

### 2.15 `logseq_rebuild_index` — Rebuild Search Index

Rebuild the full-text search index.

**Arguments:**
```json
{
  "mode": "full",             // Optional: "full" or "incremental" (default: "full")
  "since": "ISO8601"          // Optional: Timestamp for incremental mode
}
```

**Response:**
```json
{
  "status": "rebuilt",
  "mode": "full",
  "indexed_blocks": 150
}
```

---

### 2.16 `logseq_index_health` — Check Index Health

Check the health of the search index.

**Arguments:**
```json
{}
```

**Response:**
```json
{
  "fts_count": 150,
  "blocks_count": 150,
  "in_sync": true,
  "status": "healthy"
}
```

---

### 2.17 Cognitive Tools

These tools are available when cognitive engines are configured:

#### `logseq_cognitive_mirror`

Analyze a page's cognitive structure.

**Arguments:**
```json
{
  "page_name": "string"        // Required: Page name to analyze
}
```

#### `logseq_serendipity`

Find unexpected connections between knowledge blocks.

**Arguments:**
```json
{
  "since": "ISO8601",          // Optional: Filter by timestamp
  "limit": 20,                 // Optional: Max results (default: 20)
  "min_confidence": 0.3        // Optional: Min confidence 0.0-1.0 (default: 0.3)
}
```

#### `logseq_agent_memory`

Query the agent memory store.

**Arguments:**
```json
{
  "domain": "string",          // Required: Memory domain (agent ID)
  "query": "string",           // Optional: FTS query
  "limit": 10                  // Optional: Max results (default: 10)
}
```

#### `logseq_argument_map`

Map argument structure in a page.

**Arguments:**
```json
{
  "page_name": "string",       // Required: Page name to analyze
  "max_depth": 5               // Optional: Max traversal depth (default: 5)
}
```

#### `logseq_mental_model`

Get the mental model for an agent from journal entries.

**Arguments:**
```json
{
  "agent_id": "string",        // Required: Agent ID (journal prefix)
  "time_window": "string"       // Optional: Time window in days
}
```

#### `logseq_counterfactual`

Explore counterfactual scenarios and alternative branches.

**Arguments:**
```json
{
  "scenario": "string",       // Required: The scenario to explore
  "decision_point": "string"   // Required: The decision point to analyze
}
```

#### `logseq_knowledge_evolution`

Track how knowledge and beliefs evolve over time.

**Arguments:**
```json
{
  "topic": "string",           // Required: Topic to track
  "timespan_days": 30          // Optional: Time window in days (default: 30)
}
```

#### `logseq_morning_briefing`

Get a daily cognitive briefing with pulse, serendipity highlights, and decay alerts.

**Arguments:**
```json
{}
```

---

## 3. Resources

Resources provide access to graph data via the `resources/read` method.

### 3.1 URI Scheme

All resources use the `logseq://` URI scheme:

| URI | Description |
|-----|-------------|
| `logseq://graph` | Full graph statistics |
| `logseq://pages` | All pages list |
| `logseq://journals` | Journal pages list |
| `logseq://tags` | All tags with usage counts |
| `logseq://cognitive/map` | Overall cognitive analysis (when cognitive mirror configured) |
| `logseq://cognitive/serendipity` | Recent serendipity discoveries (when engine configured) |
| `logseq://cognitive/arguments/{page}` | Argument map for a page (when cartographer configured) |
| `logseq://cognitive/mental-models` | Mental model garden (when gardener configured) |

### 3.2 Request Format

```json
{
  "method": "resources/read",
  "params": {
    "uri": "logseq://pages"
  }
}
```

### 3.3 Response Format

```json
{
  "contents": [
    {
      "uri": "logseq://pages",
      "mime_type": "application/json",
      "text": "[... JSON array of pages ...]"
    }
  ]
}
```

---

## 4. Notifications

Notifications are emitted by the server when events occur. Subscribe via `subscribe()` method.

### 4.1 Notification Types

#### `notifications/block_changed`

Emitted when a block is created, updated, or deleted.

```json
{
  "method": "notifications/block_changed",
  "params": {
    "event": {
      "block_id": "uuid",
      "change_type": "Created" | "Updated" | "Deleted"
    }
  }
}
```

#### `notifications/page_created`

Emitted when a page is created.

```json
{
  "method": "notifications/page_created",
  "params": {
    "event": {
      "page_id": "uuid",
      "page_name": "Page Name"
    }
  }
}
```

---

## 5. Error Handling

### 5.1 Error Response Format

```json
{
  "content": [
    {
      "type": "text",
      "text": "Error message describing what went wrong"
    }
  ],
  "is_error": true
}
```

### 5.2 Common Error Messages

| Error | Cause |
|-------|-------|
| `"Missing 'X' parameter"` | Required argument not provided |
| `"Invalid UUID: ..."` | Malformed UUID format |
| `"Block not found: ..."` | Block does not exist |
| `"Page not found: ..."` | Page does not exist |
| `"SearchIndexManager not configured"` | Search index not set up |
| `"CognitiveMirror not configured"` | Cognitive engine not available |

---

## 6. Examples

### 6.1 Initialize Connection

```json
// Request
{
  "method": "initialize",
  "params": {
    "protocol_version": "2024-11-05",
    "capabilities": {
      "roots": { "list": true },
      "sampling": {}
    }
  }
}

// Response
{
  "protocol_version": "2024-11-05",
  "capabilities": {
    "tools": { "list_changed": false },
    "resources": { "subscribe": false, "list_changed": false },
    "notifications": {}
  },
  "server_info": {
    "name": "quilt-mcp",
    "version": "0.1.0"
  }
}
```

### 6.2 List Tools

```json
// Request
{
  "method": "tools/list"
}

// Response (truncated)
{
  "tools": [
    { "name": "logseq_query", "description": "Execute a Logseq DSL query", ... },
    { "name": "logseq_create_block", "description": "Create a new block", ... },
    ...
  ]
}
```

### 6.3 Create a Task

```json
// Request
{
  "method": "tools/call",
  "params": {
    "name": "logseq_create_task",
    "arguments": {
      "page_name": "Inbox",
      "content": "Review quarterly report",
      "priority": "a"
    }
  }
}

// Response
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"id\": \"018d1e5c-1234-7890-abcd-ef0123456789\",\n  \"page_name\": \"Inbox\",\n  \"content\": \"Review quarterly report\",\n  \"marker\": \"TODO\"\n}"
    }
  ],
  "is_error": false
}
```

### 6.4 Full-Text Search

```json
// Request
{
  "method": "tools/call",
  "params": {
    "name": "logseq_search",
    "arguments": {
      "query": "Rust async",
      "limit": 10
    }
  }
}

// Response
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"count\": 2,\n  \"results\": [\n    {\n      \"block_id\": \"018d1e5c-...\",\n      \"page_name\": \"Async Programming\",\n      \"snippet\": \"...Rust async/await pattern...\",\n      \"score\": -1.234\n    }\n  ]\n}"
    }
  ],
  "is_error": false
}
```

### 6.5 Get Block Tree

```json
// Request
{
  "method": "tools/call",
  "params": {
    "name": "logseq_get_block_tree",
    "arguments": {
      "block_id": "018d1e5c-1234-7890-abcd-ef0123456789"
    }
  }
}

// Response
{
  "content": [
    {
      "type": "text",
      "text": "{\n  \"block\": {\n    \"id\": \"018d1e5c-...\",\n    \"content\": \"Root block\",\n    ...\n  },\n  \"children\": [\n    {\n      \"id\": \"018d1e5c-...\",\n      \"content\": \"Child block\",\n      ...\n    }\n  ],\n  \"children_count\": 1\n}"
    }
  ],
  "is_error": false
}
```

---

## Appendix A: Block Object Schema

```json
{
  "id": "string (UUID)",
  "page_id": "string (UUID)",
  "parent_id": "string (UUID) | null",
  "order": "number (floating point for fractional indexing)",
  "level": "integer (1-255)",
  "content": "string (markdown/org content)",
  "marker": "string | null (Now, Later, Todo, Done, Cancelled)",
  "priority": "string | null (A, B, C)",
  "collapsed": "boolean",
  "created_at": "string (ISO8601)",
  "updated_at": "string (ISO8601)"
}
```

## Appendix B: Page Object Schema

```json
{
  "id": "string (UUID)",
  "name": "string (unique page name)",
  "title": "string | null",
  "journal": "boolean",
  "journal_day": "integer | null (YYYYMMDD format)",
  "format": "string (markdown | org)"
}
```

---

*Document generated for Quilt MCP Server v0.1.0*
