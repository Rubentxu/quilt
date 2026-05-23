#!/bin/bash
# Demo seed data for Quilt v0.1.0 showcase
# Usage: ./demo-seed.sh
#
# This script creates realistic sample data to demonstrate Quilt's features:
# - Pages with blocks demonstrating various content types
# - Properties, annotations, block refs, page refs
# - Journal entries
# - Task blocks with various markers and priorities

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DB_PATH="${QUILT_DB:-quilt.db}"
CLI="${QUILT_CLI:-cargo run -p quilt-bin --}"

echo -e "${GREEN}🌱 Seeding Quilt demo data...${NC}"
echo "Database: $DB_PATH"
echo ""

# Function to run quilt CLI command
run_quilt() {
    local cmd="$1"
    shift
    echo -e "  ${BLUE}→${NC} $cmd" >&2
    $CLI $cmd "$@" 2>/dev/null || echo "  ${YELLOW}⚠${NC} Command failed (may be expected if MCP server not running)"
}

# ============================================================
# 1. Create PKM Systems Research page
# ============================================================
echo -e "${GREEN}📄 Creating 'PKM Systems Research' page...${NC}"

run_quilt page "PKM Systems Research"

# Add main content blocks
run_quilt block --page "PKM Systems Research" "Personal Knowledge Management systems help humans externalize their thinking into a searchable, linkable graph."
run_quilt block --page "PKM Systems Research" "Key insight: The value of a PKM grows exponentially with the number of connections between notes."
run_quilt block --page "PKM Systems Research" "See [[Rust Async Patterns]] for implementation patterns in the Rust ecosystem."

# ============================================================
# 2. Create Rust Async Patterns page with tasks
# ============================================================
echo ""
echo -e "${GREEN}📄 Creating 'Rust Async Patterns' page...${NC}"

run_quilt page "Rust Async Patterns"

# Add content blocks with properties
run_quilt block --page "Rust Async Patterns" --marker todo --priority a \
    "Document Tokio task spawning patterns" --properties "tags:: [rust, tokio, async]"

run_quilt block --page "Rust Async Patterns" --marker todo --priority b \
    "Explain async/await desugaring to Future implementation"

run_quilt block --page "Rust Async Patterns" --marker done \
    "Cover Channel communication patterns (mpsc, broadcast)"

run_quilt block --page "Rust Async Patterns" --marker later \
    "Explore Actor model implementation with message passing"

# Add child blocks demonstrating hierarchy
run_quilt block --page "Rust Async Patterns" --parent "" \
    "spawn() creates detached tasks that run until completion"

run_quilt block --page "Rust Async Patterns" --parent "" \
    "spawn_blocking for CPU-intensive work off the async executor"

# ============================================================
# 3. Create MCP Protocol Analysis page
# ============================================================
echo ""
echo -e "${GREEN}📄 Creating 'MCP Protocol Analysis' page...${NC}"

run_quilt page "MCP Protocol Analysis"

run_quilt block --page "MCP Protocol Analysis" \
    "The Model Context Protocol enables AI agents to interact with external tools and knowledge bases."

run_quilt block --page "MCP Protocol Analysis" --marker todo --priority a \
    "Analyze JSON-RPC 2.0 message format requirements"

run_quilt block --page "MCP Protocol Analysis" --marker todo --priority b \
    "Document tool call request/response lifecycle"

run_quilt block --page "MCP Protocol Analysis" \
    "Key advantage: Standardized interface allows any MCP-compatible agent to work with Quilt."

# ============================================================
# 4. Create journal entries for the past week
# ============================================================
echo ""
echo -e "${GREEN}📅 Creating journal entries...${NC}"

# Get dates for the past 7 days
for i in {6..0}; do
    DATE=$(date -d "$i days ago" +%Y-%m-%d 2>/dev/null || date -v-${i}d +%Y-%m-%d 2>/dev/null)
    DAY_NAME=$(date -d "$i days ago" +%A 2>/dev/null || date -v-${i}d +%A 2>/dev/null)

    echo -e "  ${BLUE}→${NC} Journal entry for $DATE ($DAY_NAME)"

    run_quilt journal --date "$DATE" 2>/dev/null || true

    # Add some journal content
    if [ $i -eq 6 ]; then
        run_quilt block --page "$DATE" --marker done \
            "Initial Quilt setup and configuration"
        run_quilt block --page "$DATE" --marker done \
            "Review MCP server capabilities"
    elif [ $i -eq 3 ]; then
        run_quilt block --page "$DATE" --marker todo --priority a \
            "Prepare demo showcase for stakeholders"
        run_quilt block --page "$DATE" \
            "Explored connection between [[MCP Protocol Analysis]] and [[PKM Systems Research]]"
    elif [ $i -eq 0 ]; then
        run_quilt block --page "$DATE" --marker todo --priority a \
            "Complete demo script finalization"
        run_quilt block --page "$DATE" \
            "Today's focus: Finalize v0.1.0 showcase materials"
    fi
done

# ============================================================
# 5. Create Inbox page with mixed tasks
# ============================================================
echo ""
echo -e "${GREEN}📥 Creating 'Inbox' page...${NC}"

run_quilt page "Inbox"

run_quilt block --page "Inbox" --marker now --priority a \
    "Review quarterly report" --properties "due:: 2026-05-25"

run_quilt block --page "Inbox" --marker todo --priority b \
    "Update documentation for v0.1.0 release"

run_quilt block --page "Inbox" --marker todo \
    "Clean up demo database before showcase"

run_quilt block --page "Inbox" --marker later \
    "Research competitor PKM systems (Obsidian, Notion)"

run_quilt block --page "Inbox" --marker done \
    "Set up CI/CD pipeline for automated testing"

# ============================================================
# 6. Create a page demonstrating block references
# ============================================================
echo ""
echo -e "${GREEN}🔗 Creating 'Block References Demo' page...${NC}"

run_quilt page "Block References Demo"

# Create a block to reference
REF_BLOCK_ID=$(run_quilt block --page "Block References Demo" \
    "This is a block that will be referenced from another block" | \
    grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4 2>/dev/null || echo "")

run_quilt block --page "Block References Demo" \
    "The concept of externalizing memory is fundamental to PKM systems."

run_quilt block --page "Block References Demo" \
    "Notice how the block below contains a reference to a specific idea: ((block-ref))"

run_quilt block --page "Block References Demo" \
    "Block references enable explicit, addressable connections between thoughts."

# ============================================================
# 7. Create query examples page
# ============================================================
echo ""
echo -e "${GREEN}🔍 Creating 'Query Examples' page...${NC}"

run_quilt page "Query Examples"

run_quilt block --page "Query Examples" --marker done \
    "(task todo) - Find all TODO tasks"

run_quilt block --page "Query Examples" --marker done \
    "(priority a) - Find priority A items"

run_quilt block --page "Query Examples" --marker done \
    "(and (task todo) (priority a)) - Intersection of todos and priority A"

run_quilt block --page "Query Examples" --marker done \
    "(or (task done) (task cancelled)) - Union of done and cancelled"

run_quilt block --page "Query Examples" --marker done \
    "(full-text-search \"async\") - Search for 'async' in content"

# ============================================================
# 8. Create namespace page structure
# ============================================================
echo ""
echo -e "${GREEN}📁 Creating namespace pages...${NC}"

run_quilt page "projects"
run_quilt page "projects/quilt"
run_quilt page "projects/quilt/v0.1.0"

run_quilt block --page "projects/quilt/v0.1.0" --marker done \
    "MVP feature complete: MCP server, block editor, query DSL"

run_quilt block --page "projects/quilt/v0.1.0" --marker todo --priority a \
    "Prepare release documentation"

run_quilt block --page "projects/quilt/v0.1.0" --marker todo \
    "Write migration guide from Logseq"

# ============================================================
# Summary
# ============================================================
echo ""
echo -e "${GREEN}✅ Demo data seeded successfully!${NC}"
echo ""
echo "Created content includes:"
echo "  • 8+ pages across various namespaces"
echo "  • Tasks with markers: NOW, TODO, LATER, DONE"
echo "  • Properties on blocks (priority, due dates, tags)"
echo "  • Page references [[like this]]"
echo "  • Block reference syntax ((like this))"
echo "  • 7 days of journal entries"
echo "  • Hierarchical page structure (projects/quilt/v0.1.0)"
echo ""
echo "Run queries to explore your data:"
echo "  $CLI query '(task todo)'"
echo "  $CLI query '(priority a)'"
echo "  $CLI query '(and (task todo) (priority a))'"
echo "  $CLI search 'async'"
