//! Unified bootstrap characterization tests
//!
//! These tests assert that server, CLI and MCP all resolve the same
//! canonical database path (`<graph-root>/.quilt/quilt.db`) for a
//! given graph directory. They are a regression net against the
//! duplication of `ensure_vault_exists` / `init_vault` that ADR-0030
//! (Slice A) was meant to remove.

use std::path::Path;
use tempfile::tempdir;

use quilt_platform::init::{ensure_graph_layout, init_graph};

/// Every entry point must resolve the canonical layout to the same
/// path for the same graph root.
#[test]
fn canonical_layout_is_resolved_consistently() {
    let tmp = tempdir().unwrap();
    let graph_path = tmp.path();

    // Resolver path (server / MCP startup)
    let from_resolver = ensure_graph_layout(graph_path);

    // Bootstrap path (CLI / server `init_graph` / MCP `init_graph`)
    let cfg = init_graph(graph_path.to_path_buf()).expect("init_graph should succeed");

    assert_eq!(from_resolver, cfg.db_path);
    assert_eq!(from_resolver, graph_path.join(".quilt").join("quilt.db"));
}

/// The path under any graph root must be `<root>/.quilt/quilt.db`,
/// regardless of whether the root is relative, absolute, contains
/// spaces or unicode.
#[test]
fn canonical_layout_under_various_roots() {
    let cases: Vec<(&'static str, Box<dyn Fn(&Path) -> std::path::PathBuf>)> = vec![
        (
            "relative",
            Box::new(|p| p.join("g1").join(".quilt").join("quilt.db")),
        ),
        (
            "absolute",
            Box::new(|p| p.join("g2").join(".quilt").join("quilt.db")),
        ),
        (
            "with-spaces",
            Box::new(|p| p.join("my graph").join(".quilt").join("quilt.db")),
        ),
        (
            "with-unicode",
            Box::new(|p| p.join("grafo-α").join(".quilt").join("quilt.db")),
        ),
    ];

    for (name, expected_fn) in cases {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join(match name {
            "relative" => "g1",
            "absolute" => "g2",
            "with-spaces" => "my graph",
            "with-unicode" => "grafo-α",
            _ => unreachable!(),
        });
        std::fs::create_dir_all(&root).unwrap();
        let resolved = ensure_graph_layout(&root);
        let expected = expected_fn(tmp.path());
        assert_eq!(
            resolved, expected,
            "case `{name}` failed: expected {expected:?}, got {resolved:?}"
        );
    }
}

/// `init_graph` must not recreate an existing valid layout.
#[test]
fn init_graph_preserves_existing_db() {
    let tmp = tempdir().unwrap();
    let graph_path = tmp.path().to_path_buf();
    let cfg1 = init_graph(graph_path.clone()).unwrap();
    // Pre-existing marker
    let marker = "pre-existing-marker";
    std::fs::write(&cfg1.db_path, marker).unwrap();

    // Re-run should be a no-op
    let cfg2 = init_graph(graph_path).unwrap();
    assert_eq!(cfg1, cfg2);
    let contents = std::fs::read_to_string(&cfg2.db_path).unwrap();
    assert_eq!(
        contents, marker,
        "init_graph must not overwrite an existing quilt.db"
    );
}

/// `ensure_graph_layout` is pure — it must not touch the filesystem.
#[test]
fn ensure_graph_layout_is_pure() {
    let tmp = tempdir().unwrap();
    let p = ensure_graph_layout(tmp.path());
    // The parent .quilt/ directory must NOT have been created.
    assert!(
        !tmp.path().join(".quilt").exists(),
        "ensure_graph_layout must not create directories"
    );
    assert_eq!(p, tmp.path().join(".quilt").join("quilt.db"));
}
