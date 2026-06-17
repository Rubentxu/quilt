//! CLI argument parsing tests using clap's try_parse_from.
//!
//! Covers: all commands (Init, Page, Block, Journal, Query, Search,
//! Serve, ListPages, PageInfo), flags (--graph-dir, --db-path deprecated,
//! --verbose), and error cases.

use clap::Parser;
use quilt_platform::cli::{Command, QuiltCLI};
use std::path::PathBuf;

// ── Global flags ────────────────────────────────────────────

#[test]
fn test_default_graph_dir() {
    let cli = QuiltCLI::try_parse_from(["quilt", "list-pages"]).unwrap();
    assert_eq!(cli.graph_dir, PathBuf::from("."));
    assert!(cli.db_path.is_none());
}

#[test]
fn test_custom_graph_dir() {
    let cli = QuiltCLI::try_parse_from(["quilt", "--graph-dir", "mydir", "list-pages"]).unwrap();
    assert_eq!(cli.graph_dir, PathBuf::from("mydir"));
    assert!(cli.db_path.is_none());
}

#[test]
fn test_long_db_path_flag_is_deprecated_alias() {
    // --db-path remains a deprecated alias; resolved_graph_dir maps
    // a `.db` value to its parent. The raw field is now Option<PathBuf>.
    let cli =
        QuiltCLI::try_parse_from(["quilt", "--db-path", "/tmp/test.db", "list-pages"]).unwrap();
    assert_eq!(cli.db_path, Some(PathBuf::from("/tmp/test.db")));
    let (gd, used) = cli.resolved_graph_dir();
    assert!(used);
    assert_eq!(gd, PathBuf::from("/tmp"));
}

#[test]
fn test_verbose_flag() {
    let cli = QuiltCLI::try_parse_from(["quilt", "-v", "list-pages"]).unwrap();
    assert!(cli.verbose);
}

#[test]
fn test_verbose_default_false() {
    let cli = QuiltCLI::try_parse_from(["quilt", "list-pages"]).unwrap();
    assert!(!cli.verbose);
}

// ── Init command ────────────────────────────────────────────

#[test]
fn test_init_command() {
    let cli = QuiltCLI::try_parse_from(["quilt", "init", "-n", "my-graph"]).unwrap();
    match cli.command {
        Command::Init { name } => assert_eq!(name, "my-graph"),
        _ => panic!("expected Init"),
    }
}

#[test]
fn test_init_command_long_flag() {
    let cli = QuiltCLI::try_parse_from(["quilt", "init", "--name", "my-graph"]).unwrap();
    match cli.command {
        Command::Init { name } => assert_eq!(name, "my-graph"),
        _ => panic!("expected Init"),
    }
}

#[test]
fn test_init_requires_name() {
    let result = QuiltCLI::try_parse_from(["quilt", "init"]);
    assert!(result.is_err());
}

// ── Page command ────────────────────────────────────────────

#[test]
fn test_page_command() {
    let cli = QuiltCLI::try_parse_from(["quilt", "page", "-n", "my-page"]).unwrap();
    match cli.command {
        Command::Page { name } => assert_eq!(name, "my-page"),
        _ => panic!("expected Page"),
    }
}

#[test]
fn test_page_requires_name() {
    let result = QuiltCLI::try_parse_from(["quilt", "page"]);
    assert!(result.is_err());
}

// ── Block command ───────────────────────────────────────────

#[test]
fn test_block_command() {
    let cli = QuiltCLI::try_parse_from(["quilt", "block", "-p", "my-page", "-c", "Hello"]).unwrap();
    match cli.command {
        Command::Block {
            page,
            content,
            parent,
        } => {
            assert_eq!(page, "my-page");
            assert_eq!(content, "Hello");
            assert_eq!(parent, None);
        }
        _ => panic!("expected Block"),
    }
}

#[test]
fn test_block_with_parent() {
    let cli = QuiltCLI::try_parse_from([
        "quilt",
        "block",
        "-p",
        "my-page",
        "-c",
        "Child",
        "--parent",
        "some-uuid",
    ])
    .unwrap();
    match cli.command {
        Command::Block { parent, .. } => assert_eq!(parent, Some("some-uuid".to_string())),
        _ => panic!("expected Block"),
    }
}

#[test]
fn test_block_requires_page_and_content() {
    let result = QuiltCLI::try_parse_from(["quilt", "block", "-p", "p"]);
    assert!(result.is_err());
}

// ── Journal command ─────────────────────────────────────────

#[test]
fn test_journal_without_date() {
    let cli = QuiltCLI::try_parse_from(["quilt", "journal"]).unwrap();
    match cli.command {
        Command::Journal { date } => assert_eq!(date, None),
        _ => panic!("expected Journal"),
    }
}

#[test]
fn test_journal_with_date() {
    let cli = QuiltCLI::try_parse_from(["quilt", "journal", "-d", "2026-06-02"]).unwrap();
    match cli.command {
        Command::Journal { date } => assert_eq!(date, Some("2026-06-02".to_string())),
        _ => panic!("expected Journal"),
    }
}

// ── Query command ───────────────────────────────────────────

#[test]
fn test_query_default_limit() {
    let cli = QuiltCLI::try_parse_from(["quilt", "query", "-d", "(and (task TODO))"]).unwrap();
    match cli.command {
        Command::Query { dsl, limit } => {
            assert_eq!(dsl, "(and (task TODO))");
            assert_eq!(limit, 100);
        }
        _ => panic!("expected Query"),
    }
}

#[test]
fn test_query_custom_limit() {
    let cli = QuiltCLI::try_parse_from(["quilt", "query", "-d", "test", "-l", "50"]).unwrap();
    match cli.command {
        Command::Query { limit, .. } => assert_eq!(limit, 50),
        _ => panic!("expected Query"),
    }
}

// ── Search command ──────────────────────────────────────────

#[test]
fn test_search_command() {
    let cli = QuiltCLI::try_parse_from(["quilt", "search", "-q", "rust testing"]).unwrap();
    match cli.command {
        Command::Search { query } => assert_eq!(query, "rust testing"),
        _ => panic!("expected Search"),
    }
}

// ── Serve command ───────────────────────────────────────────

#[test]
fn test_serve_default_port() {
    let cli = QuiltCLI::try_parse_from(["quilt", "serve"]).unwrap();
    match cli.command {
        Command::Serve { port } => assert_eq!(port, 3541),
        _ => panic!("expected Serve"),
    }
}

#[test]
fn test_serve_custom_port() {
    let cli = QuiltCLI::try_parse_from(["quilt", "serve", "-p", "8080"]).unwrap();
    match cli.command {
        Command::Serve { port } => assert_eq!(port, 8080),
        _ => panic!("expected Serve"),
    }
}

// ── ListPages command ───────────────────────────────────────

#[test]
fn test_list_pages_command() {
    let cli = QuiltCLI::try_parse_from(["quilt", "list-pages"]).unwrap();
    assert!(matches!(cli.command, Command::ListPages));
}

// ── PageInfo command ────────────────────────────────────────

#[test]
fn test_page_info_command() {
    let cli = QuiltCLI::try_parse_from(["quilt", "page-info", "-n", "my-page"]).unwrap();
    match cli.command {
        Command::PageInfo { name } => assert_eq!(name, "my-page"),
        _ => panic!("expected PageInfo"),
    }
}

// ── Unknown command ─────────────────────────────────────────

#[test]
fn test_unknown_command() {
    let result = QuiltCLI::try_parse_from(["quilt", "nonexistent"]);
    assert!(result.is_err());
}

// ── Combined flags ──────────────────────────────────────────

#[test]
fn test_combined_flags() {
    let cli = QuiltCLI::try_parse_from([
        "quilt",
        "--graph-dir",
        "custom-graph",
        "--verbose",
        "search",
        "--query",
        "test",
    ])
    .unwrap();
    assert_eq!(cli.graph_dir, PathBuf::from("custom-graph"));
    assert!(cli.db_path.is_none());
    assert!(cli.verbose);
}
