use clap::Parser;
use quilt_platform::cli::QuiltCLI;

#[test]
fn test_cli_help() {
    let result = QuiltCLI::try_parse_from(["quilt", "--help"]);
    // Should show help (this is an error in clap terms since --help triggers exit)
    assert!(result.is_err());
}

#[test]
fn test_cli_version() {
    let result = QuiltCLI::try_parse_from(["quilt", "--version"]);
    assert!(result.is_err()); // --version triggers exit
}

#[test]
fn test_cli_no_args_error() {
    let result = QuiltCLI::try_parse_from(["quilt"]);
    assert!(result.is_err());
}

#[test]
fn test_cli_list_pages_parses() {
    let cli = QuiltCLI::try_parse_from(["quilt", "list-pages"]).unwrap();
    assert!(cli.verbose == false);
}

#[test]
fn test_cli_verbose_flag() {
    let cli = QuiltCLI::try_parse_from(["quilt", "--verbose", "list-pages"]).unwrap();
    assert!(cli.verbose);
}

#[test]
fn test_cli_custom_db_path_deprecated() {
    // --db-path remains as a deprecated alias; resolved_graph_dir maps
    // a `.db` file to its parent.
    let cli =
        QuiltCLI::try_parse_from(["quilt", "--db-path", "/tmp/test.db", "list-pages"]).unwrap();
    let (gd, used) = cli.resolved_graph_dir();
    assert!(used);
    assert_eq!(gd.to_string_lossy(), "/tmp");
}

#[test]
fn test_cli_custom_graph_dir() {
    let cli = QuiltCLI::try_parse_from(["quilt", "--graph-dir", "/tmp/g1", "list-pages"]).unwrap();
    assert_eq!(cli.graph_dir.to_string_lossy(), "/tmp/g1");
    assert!(cli.db_path.is_none());
}
