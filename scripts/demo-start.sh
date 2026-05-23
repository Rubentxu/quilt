#!/bin/bash
# Quilt v0.1.0 Quick Start Demo
# Usage: ./demo-start.sh
#
# This script guides you through starting Quilt and demonstrating its features.
# Run sections individually or all at once.

set -e

# Configuration
DB_PATH="${QUILT_DB:-quilt.db}"
CLI="${QUILT_CLI:-cargo run -p quilt-bin --}"
SERVER_PORT="${QUILT_PORT:-8080}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# ============================================================
# Helper Functions
# ============================================================

print_header() {
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
    echo ""
}

print_step() {
    echo -e "${YELLOW}▶ STEP $1:${NC} ${BOLD}$2${NC}"
    echo ""
}

print_command() {
    echo -e "  ${MAGENTA}$1${NC}"
    echo ""
}

print_explanation() {
    echo -e "  ${NC}$1${NC}"
    echo ""
}

print_success() {
    echo -e "  ${GREEN}✓${NC} $1"
    echo ""
}

print_warning() {
    echo -e "  ${YELLOW}!${NC} $1"
    echo ""
}

wait_for_key() {
    echo ""
    read -p "Press Enter to continue..." -r
    echo ""
}

# ============================================================
# Prerequisites Check
# ============================================================

print_header "QUILT v0.1.0 QUICK START DEMO"

echo -e "${BOLD}Welcome to the Quilt demo!${NC}"
echo "This script will guide you through Quilt's key features."
echo ""
echo "Let's verify your environment first..."

# Check if database exists
if [ -f "$DB_PATH" ]; then
    print_success "Database found: $DB_PATH"
else
    print_warning "Database not found: $DB_PATH"
    echo "  Run './scripts/demo-seed.sh' first to create sample data"
    echo ""
fi

# Check if project builds
echo -n "  Checking Rust build... "
if cargo build -p quilt-bin --quiet 2>/dev/null; then
    print_success "Build successful"
else
    print_warning "Build may need 'cargo build' first"
fi

echo ""
echo -e "${BOLD}Demo sections:${NC}"
echo "  1. Initialize database"
echo "  2. Seed demo data"
echo "  3. Start MCP server"
echo "  4. Run CLI queries"
echo "  5. Query DSL examples"
echo "  6. MCP tools overview"
echo ""
read -p "Press Enter to start or Ctrl+C to exit..." -r
echo ""

# ============================================================
# Step 1: Initialize Database
# ============================================================

print_step "1" "Initialize Database"

print_command "cargo run -p quilt-bin -- db init"
print_explanation "Creates the SQLite database with all required tables:"

echo "  • pages          - Page containers for blocks"
echo "  • blocks         - Content blocks with hierarchy"
echo "  • annotations    - Human-agent communication"
echo "  • block_refs     - Block-to-block references"
echo "  • page_refs      - Page references"
echo "  • tasks          - Task tracking"
echo "  • cognitive_*    - Cognitive engine tables"

wait_for_key

# ============================================================
# Step 2: Seed Demo Data
# ============================================================

print_step "2" "Seed Demo Data"

print_command "./scripts/demo-seed.sh"
print_explanation "Creates realistic sample data including:"

echo "  • PKM Systems Research page"
echo "  • Rust Async Patterns with tasks"
echo "  • MCP Protocol Analysis"
echo "  • 7 days of journal entries"
echo "  • Inbox with prioritized tasks"
echo "  • Namespace structure (projects/quilt/v0.1.0)"

wait_for_key

# ============================================================
# Step 3: Start MCP Server
# ============================================================

print_step "3" "Start MCP Server"

print_command "cargo run -p quilt-bin -- serve"
print_explanation "Starts the MCP server for AI agent integration."
echo ""
echo "  The MCP server exposes 30+ tools including:"
echo "  • Page tools: create_page, get_page, list_pages, delete_page"
echo "  • Block tools: create_block, get_block, update_block, move_block"
echo "  • Task tools: create_task, schedule_task, list_tasks"
echo "  • Search tools: search (FTS5), query (DSL)"
echo "  • Cognitive tools: morning_briefing, serendipity, cognitive_mirror"
echo ""
echo "  MCP uses JSON-RPC 2.0 over stdio for AI agent communication."

wait_for_key

# ============================================================
# Step 4: Run CLI Queries
# ============================================================

print_step "4" "Run CLI Queries"

print_command "$CLI list-pages"
print_explanation "Lists all pages in the knowledge graph"

print_command "$CLI query '(task todo)'"
print_explanation "Finds all blocks marked as TODO"

print_command "$CLI query '(priority a)'"
print_explanation "Finds all blocks with priority A"

print_command "$CLI query '(and (task todo) (priority a))'"
print_explanation "Intersection: TODO tasks with priority A"

print_command "$CLI search 'async'"
print_explanation "Full-text search across all blocks using FTS5"

wait_for_key

# ============================================================
# Step 5: Query DSL Examples
# ============================================================

print_step "5" "Query DSL Examples"

echo -e "${BOLD}The Query DSL is a Polish notation (S-expression) language${NC}"
echo ""

# Boolean operators
echo -e "${BOLD}Boolean Operators:${NC}"
print_command "(and (task todo) (priority a))"
print_explanation "AND: Intersection of TODO and priority A"

print_command "(or (task todo) (task later))"
print_explanation "OR: Union of TODO and LATER tasks"

print_command "(not (task done))"
print_explanation "NOT: Everything except DONE"

# Comparison operators
echo -e "${BOLD}Comparison Operators:${NC}"
print_command "(between :created_at \"2026-01-01\" \"2026-05-21\")"
print_explanation "Date range filter on created_at"

print_command "(property \"count\" > 10)"
print_explanation "Numeric comparison on property value"

# Filter operators
echo -e "${BOLD}Filter Operators:${NC}"
print_command "(task todo|done|cancelled)"
print_explanation "Multiple task markers"

print_command "(page \"Rust Async Patterns\")"
print_explanation "All blocks on a specific page"

print_command "(tags \"rust\")"
print_explanation "Blocks with specific tag"

print_command "(full-text-search \"async runtime\")"
print_explanation "FTS5 full-text search"

# Existence operators
echo -e "${BOLD}Existence Operators:${NC}"
print_command "(exists \"due_date\")"
print_explanation "Blocks that have a due_date property"

print_command "(missing \"completed_at\")"
print_explanation "Blocks missing a completed_at property"

print_command "(namespace \"projects\")"
print_explanation "Blocks in projects namespace"

wait_for_key

# ============================================================
# Step 6: MCP Tools Overview
# ============================================================

print_step "6" "MCP Tools Overview"

echo -e "${BOLD}Available MCP tools (30+):${NC}"
echo ""

echo "┌─────────────────────────────────────────────────────────────┐"
echo "│ Page Tools (5)                                              │"
echo "├─────────────────────────────────────────────────────────────┤"
echo "│ create_page   get_page       list_pages   delete_page       │"
echo "│ get_orphan_pages                                      │"
echo "└─────────────────────────────────────────────────────────────┘"
echo ""
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│ Block Tools (12)                                           │"
echo "├─────────────────────────────────────────────────────────────┤"
echo "│ create_block   get_block      update_block   delete_block  │"
echo "│ move_block     get_block_tree get_page_blocks link_blocks  │"
echo "│ get_backlinks  restore_block  recycle_bin    orphan_blocks │"
echo "└─────────────────────────────────────────────────────────────┘"
echo ""
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│ Task Tools (4)                                             │"
echo "├─────────────────────────────────────────────────────────────┤"
echo "│ create_task    schedule_task   list_tasks    get_daily_summary│"
echo "└─────────────────────────────────────────────────────────────┘"
echo ""
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│ Search Tools (5)                                           │"
echo "├─────────────────────────────────────────────────────────────┤"
echo "│ search        query          rebuild_index index_health     │"
echo "│ get_orphan_blocks                                    │"
echo "└─────────────────────────────────────────────────────────────┘"
echo ""
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│ Cognitive Tools (8)                                        │"
echo "├─────────────────────────────────────────────────────────────┤"
echo "│ morning_briefing   serendipity    cognitive_mirror  │"
echo "│ agent_memory       argument_map   mental_model      │"
echo "│ counterfactual     knowledge_evolution                  │"
echo "└─────────────────────────────────────────────────────────────┘"

echo ""
echo -e "${BOLD}Example MCP tool call via curl:${NC}"
print_command "curl -X POST http://localhost:$SERVER_PORT/mcp \\
  -H 'Content-Type: application/json' \\
  -d '{\"method\":\"tools/call\",\"params\":{\"name\":\"logseq_query\",\"arguments\":{\"dsl\":\"(task todo)\"}}}'"

wait_for_key

# ============================================================
# Completion
# ============================================================

print_header "DEMO COMPLETE"

echo -e "${GREEN}✓${NC} You've seen the basics of Quilt v0.1.0!"
echo ""
echo "Next steps:"
echo "  1. Explore the Web App: ${BOLD}just serve${NC} (starts HTTP server)"
echo "  2. Try the UI: ${BOLD}just dev-wasm${NC} (starts WASM dev server)"
echo "  3. Run tests: ${BOLD}just test${NC}"
echo "  4. Build release: ${BOLD}just release${NC}"
echo ""
echo "Documentation:"
echo "  • ${BOLD}docs/DEMO_SHOWCASE.md${NC} - Full demo guide"
echo "  • ${BOLD}docs/mcp-api.md${NC} - MCP API reference"
echo "  • ${BOLD}docs/reversa/query-dsl-spec.md${NC} - Query DSL specification"
echo "  • ${BOLD}docs/product-spec.md${NC} - Product specification"
echo ""
echo -e "${GREEN}Thank you for trying Quilt!${NC}"
echo ""
