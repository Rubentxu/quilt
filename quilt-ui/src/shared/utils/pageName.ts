// ──── Page name normalization ─────────────────────────────────────────
//
// The Quilt server normalises page names on insert (mirroring Quilt):
//
//     crates/quilt-domain/src/entities/page.rs
//       pub fn normalize_name(name: &str) -> Result<String, DomainError> {
//           let normalized = name.trim().to_lowercase();
//           ...
//       }
//
// In other words, every page name in the system is lowercase + trimmed.
// The lookup endpoints (get_by_name, get_journal) query the database
// case-sensitively, so callers MUST pass the canonical (lowercase) form
// or the query returns 404.
//
// This helper is the client-side mirror of `Page::normalize_name`. Use
// it whenever you compare, create, or navigate using a page name taken
// from user input or block content (a `[[Page]]` reference, a #tag, a
// search result picked from autocomplete, etc.). Without this, the
// typical case where a user types `[[My Notes]]` ends up creating
// `mynotes` on the server and then asking for `/page/My Notes` — which
// the server then 404s because it stores `mynotes`, not `My Notes`.
//
// Note: this client-side helper does NOT enforce the full server-side
// rules (no special characters, no template/ prefix handling, no
// all-numeric names). The server is the source of truth for those;
// the client just ensures case-insensitivity to avoid the bug above.

/**
 * Return the canonical page name for lookups, creation, and navigation.
 *
 * - Trims surrounding whitespace
 * - Lowercases the result
 *
 * Returns the empty string for an input that is all whitespace; callers
 * should treat that as "no page" and bail out before hitting the API.
 */
export function normalizePageName(name: string): string {
  return name.trim().toLowerCase()
}
