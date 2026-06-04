-- 006_add_page_properties.sql
--
-- Add typed properties to the `pages` table (F5: Page Carries Typed Properties).
--
-- This is an ADDITIVE migration — no data backfill, no row rewriting.
-- Pre-existing pages get `'{}'` (empty JSON object) which the Rust
-- deserializer (parse_properties in repositories.rs) treats as an empty
-- HashMap<String, DefaultPropertyEntry<PropertyValue>>.
--
-- Rollback: ALTER TABLE pages DROP COLUMN properties;
--
-- Column type: TEXT (not BLOB) per design decision. TEXT is human-readable
-- in SQLite CLI, json_extract works on both TEXT and BLOB, and the spec
-- explicitly chose TEXT. The blocks table uses BLOB for its properties
-- column (pre-existing pattern); this inconsistency is acceptable because
-- Page and Block have different merge semantics and the difference is
-- contained to the repository layer.

ALTER TABLE pages ADD COLUMN properties TEXT NOT NULL DEFAULT '{}';
