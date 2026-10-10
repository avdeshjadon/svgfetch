//! Command-line interface definition (clap).

use clap::{Parser, Subcommand};

const HELP_EXAMPLES: &str = "\
Variants:
  default   Best canonical asset for the entity
  icon      Icon/symbol/mark only
  wordmark  Text-based brand name / logotype
  full      Complete logo + wordmark / lockup
  mascot    Mascot or character asset

Examples:
  svgfetch instagram
  svgfetch instagram --variant icon
  svgfetch instagram --variant wordmark
  svgfetch instagram --variant full
  svgfetch docker --variant full
  svgfetch linux --variant mascot
  svgfetch facebook
  svgfetch github
  svgfetch python
  svgfetch info instagram --variant wordmark
  svgfetch instagram --dry-run

Behavior:
  By default SVGFetch automatically selects the best matching canonical asset.
  Use --variant when you need a specific asset type.";

/// svgfetch — discover, inspect, and download SVG assets from Wikimedia Commons.
#[derive(Debug, Parser)]
#[command(
    name = "svgfetch",
    bin_name = "svgfetch",
    version,
    about = "svgfetch — Discover. Download. Ship SVGs.",
    long_about = "svgfetch is a terminal-first client for discovering, inspecting, and \
downloading SVG assets from Wikimedia Commons.\n\nRun `svgfetch` with no arguments to \
launch the interactive interface.",
    after_help = HELP_EXAMPLES,
    propagate_version = true
)]
pub struct Cli {
    /// Enable debug logging and technical error details.
    #[arg(long, global = true)]
    pub debug: bool,

    /// Enable info logging.
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Asset variant to select: default, icon, wordmark, full, mascot.
    #[arg(long, global = true, value_enum)]
    pub variant: Option<crate::models::AssetVariant>,

    /// Optional destination directory or file path for direct brand download.
    #[arg(short = 'o', long = "output")]
    pub output: Option<std::path::PathBuf>,

    /// Overwrite existing files instead of skipping.
    #[arg(long)]
    pub overwrite: bool,

    /// Perform a dry run showing what would be done without modifying files or downloading.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Force refresh cached data.
    #[arg(long, global = true)]
    pub refresh: bool,

    /// Disable cache reads and writes.
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Explicitly enable project-aware placement.
    #[arg(long, global = true, conflicts_with = "no_project")]
    pub project: bool,

    /// Disable automatic project-aware placement.
    #[arg(long, global = true, conflicts_with = "project")]
    pub no_project: bool,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Direct brand/logo to download (e.g. `svgfetch amazon`, `svgfetch "amazon prime"`).
    /// If omitted, launches the interactive terminal UI.
    #[arg(num_args = 1..)]
    pub brand: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Search Wikimedia Commons for SVG files.
    Search {
        /// Search query (terms, phrases, and categories all work).
        query: String,

        /// Maximum number of results to return.
        #[arg(long, short = 'n', default_value_t = 25)]
        limit: usize,

        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value_t = crate::output::OutputFormat::Table)]
        format: crate::output::OutputFormat,

        /// Download matching results into this directory.
        #[arg(long, value_name = "DIR")]
        download: Option<std::path::PathBuf>,

        /// Pack matching results into a ZIP archive.
        #[arg(long, value_name = "FILE")]
        zip: Option<std::path::PathBuf>,

        /// Write attribution/license metadata into this directory.
        #[arg(long, value_name = "DIR")]
        metadata: Option<std::path::PathBuf>,

        /// Only include results whose license string contains this text.
        #[arg(long)]
        license: Option<String>,

        /// Page of results to fetch (page size = --limit).
        #[arg(long, default_value_t = 1)]
        page: u32,

        /// Skip confirmation prompts for bulk operations.
        #[arg(long, short = 'y')]
        yes: bool,

        /// Overwrite existing files instead of skipping them.
        #[arg(long)]
        overwrite: bool,

        /// Maximum parallel downloads (1-32).
        #[arg(long)]
        concurrency: Option<usize>,
    },

    /// Download a single file by name or MediaWiki title.
    Download {
        /// File name or `File:Title.svg`.
        file: String,

        /// Destination directory (defaults to your configured download dir).
        #[arg(long, short = 'o', value_name = "DIR")]
        output: Option<std::path::PathBuf>,

        /// Output format for the result record.
        #[arg(long, short = 'f', value_enum, default_value_t = crate::output::OutputFormat::Table)]
        format: crate::output::OutputFormat,

        /// Overwrite an existing file.
        #[arg(long)]
        overwrite: bool,
    },

    /// List SVG files in a Wikimedia Commons category.
    Category {
        /// Category name, with or without the `Category:` prefix.
        category: String,

        /// Maximum number of results.
        #[arg(long, short = 'n', default_value_t = 25)]
        limit: usize,

        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value_t = crate::output::OutputFormat::Table)]
        format: crate::output::OutputFormat,

        /// Download matching results into this directory.
        #[arg(long, value_name = "DIR")]
        download: Option<std::path::PathBuf>,

        /// Pack matching results into a ZIP archive.
        #[arg(long, value_name = "FILE")]
        zip: Option<std::path::PathBuf>,

        /// Skip confirmation prompts for bulk operations.
        #[arg(long, short = 'y')]
        yes: bool,

        /// Overwrite existing files instead of skipping them.
        #[arg(long)]
        overwrite: bool,
    },

    /// Search and download everything matching in one step.
    Batch {
        /// Search query.
        query: String,

        /// Destination directory (defaults to your configured download dir).
        #[arg(long, short = 'o', value_name = "DIR")]
        download: Option<std::path::PathBuf>,

        /// Pack results into a ZIP archive.
        #[arg(long, value_name = "FILE")]
        zip: Option<std::path::PathBuf>,

        /// Write attribution/license metadata here.
        #[arg(long, value_name = "DIR")]
        metadata: Option<std::path::PathBuf>,

        /// Maximum number of files to download.
        #[arg(long, short = 'n', default_value_t = 50)]
        limit: usize,

        /// Skip confirmation prompts.
        #[arg(long, short = 'y')]
        yes: bool,

        /// Overwrite existing files instead of skipping them.
        #[arg(long)]
        overwrite: bool,
    },

    /// Inspect or manage the local cache.
    #[command(subcommand)]
    Cache(CacheCommand),

    /// Show (or initialize) the configuration file.
    Config {
        /// Write a default config file if missing.
        #[arg(long)]
        init: bool,
    },

    /// Diagnose environment, network, and configuration problems.
    Doctor {
        /// Emit the report as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Print version information.
    Version,

    /// Completely remove svgfetch from this system.
    ///
    /// Deletes the config (settings, cache, recent searches), the default
    /// `svgfetch` download folder, global npm package, and installed
    /// binaries — leaving no trace.
    #[command(alias = "dlt")]
    Uninstall {
        /// Skip the confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Download and install the latest release, replacing this binary.
    ///
    /// Verifies the download with SHA-256, swaps `svgfetch` in place,
    /// and removes old binaries and leftover `.old` files. Uses the GitHub
    /// releases API pointed at this project unless `SVGFETCH_UPDATE_REPO` is set.
    Update,

    /// Display detailed metadata for an SVG asset without downloading.
    Info {
        /// File name, MediaWiki title, or query (e.g. `File:GitHub_Logo.svg`, `amazon`, or `github.svg`).
        asset: String,

        /// Asset variant: default, icon, wordmark, full, mascot.
        #[arg(long, value_enum)]
        variant: Option<crate::models::AssetVariant>,

        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value_t = crate::output::OutputFormat::Table)]
        format: crate::output::OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
pub enum CacheCommand {
    /// Show cache location and disk usage.
    Status,
    /// Delete all cached responses.
    Clear,
}

impl Cli {
    /// Parse from `std::env::args_os` (clap's standard entry point).
    pub fn parse_args() -> Cli {
        <Cli as Parser>::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_search() {
        let cli = Cli::parse_from([
            "svgfetch", "search", "github", "--limit", "10", "--format", "json",
        ]);
        match cli.command {
            Some(Command::Search {
                query,
                limit,
                format,
                ..
            }) => {
                assert_eq!(query, "github");
                assert_eq!(limit, 10);
                assert_eq!(format, crate::output::OutputFormat::Json);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_download_with_output() {
        let cli = Cli::parse_from(["svgfetch", "download", "File:GitHub_Logo.svg", "-o", "/tmp"]);
        match cli.command {
            Some(Command::Download { file, output, .. }) => {
                assert_eq!(file, "File:GitHub_Logo.svg");
                assert_eq!(output.unwrap(), std::path::PathBuf::from("/tmp"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_cache_clear() {
        let cli = Cli::parse_from(["svgfetch", "cache", "clear"]);
        assert!(matches!(
            cli.command,
            Some(Command::Cache(CacheCommand::Clear))
        ));
    }

    #[test]
    fn no_subcommand_means_interactive() {
        let cli = Cli::parse_from(["svgfetch"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn global_debug_flag() {
        let cli = Cli::parse_from(["svgfetch", "--debug", "version"]);
        assert!(cli.debug);
    }

    #[test]
    fn parses_dlt_with_yes() {
        let cli = Cli::parse_from(["svgfetch", "dlt", "--yes"]);
        assert!(matches!(
            cli.command,
            Some(Command::Uninstall { yes: true })
        ));
    }

    #[test]
    fn parses_uninstall_with_yes() {
        let cli = Cli::parse_from(["svgfetch", "uninstall", "--yes"]);
        assert!(matches!(
            cli.command,
            Some(Command::Uninstall { yes: true })
        ));
    }

    #[test]
    fn parses_direct_brand_argument() {
        let cli = Cli::parse_from(["svgfetch", "amazon"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.brand, vec!["amazon"]);
    }

    #[test]
    fn parses_multi_word_brand_arguments() {
        let cli = Cli::parse_from(["svgfetch", "amazon", "prime"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.brand, vec!["amazon", "prime"]);
    }

    #[test]
    fn parses_dry_run_and_cache_flags() {
        let cli = Cli::parse_from([
            "svgfetch",
            "--dry-run",
            "--refresh",
            "--no-project",
            "amazon",
        ]);
        assert!(cli.dry_run);
        assert!(cli.refresh);
        assert!(cli.no_project);
        assert!(!cli.project);
        assert_eq!(cli.brand, vec!["amazon"]);
    }

    #[test]
    fn parses_info_command() {
        let cli = Cli::parse_from(["svgfetch", "info", "File:GitHub_Logo.svg", "-f", "json"]);
        match cli.command {
            Some(Command::Info {
                asset,
                format,
                variant,
            }) => {
                assert_eq!(asset, "File:GitHub_Logo.svg");
                assert_eq!(format, crate::output::OutputFormat::Json);
                assert!(variant.is_none());
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_info_command_with_variant() {
        let cli = Cli::parse_from(["svgfetch", "info", "instagram", "--variant", "wordmark"]);
        match cli.command {
            Some(Command::Info { asset, variant, .. }) => {
                assert_eq!(asset, "instagram");
                assert_eq!(variant, Some(crate::models::AssetVariant::Wordmark));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_variant_flags_and_aliases() {
        let cli = Cli::parse_from(["svgfetch", "instagram", "--variant", "icon"]);
        assert_eq!(cli.variant, Some(crate::models::AssetVariant::Icon));

        let cli = Cli::parse_from(["svgfetch", "instagram", "--variant", "symbol"]);
        assert_eq!(cli.variant, Some(crate::models::AssetVariant::Icon));

        let cli = Cli::parse_from(["svgfetch", "instagram", "--variant", "wordmark"]);
        assert_eq!(cli.variant, Some(crate::models::AssetVariant::Wordmark));

        let cli = Cli::parse_from(["svgfetch", "instagram", "--variant", "text"]);
        assert_eq!(cli.variant, Some(crate::models::AssetVariant::Wordmark));

        let cli = Cli::parse_from(["svgfetch", "docker", "--variant", "full"]);
        assert_eq!(cli.variant, Some(crate::models::AssetVariant::Full));

        let cli = Cli::parse_from(["svgfetch", "docker", "--variant", "lockup"]);
        assert_eq!(cli.variant, Some(crate::models::AssetVariant::Full));

        let cli = Cli::parse_from(["svgfetch", "linux", "--variant", "mascot"]);
        assert_eq!(cli.variant, Some(crate::models::AssetVariant::Mascot));

        let cli = Cli::parse_from(["svgfetch", "linux", "--variant", "character"]);
        assert_eq!(cli.variant, Some(crate::models::AssetVariant::Mascot));
    }

    #[test]
    fn rejects_invalid_variant_cleanly() {
        let parsed = Cli::try_parse_from(["svgfetch", "instagram", "--variant", "banana"]);
        assert!(parsed.is_err());
        let err = parsed.unwrap_err().to_string();
        assert!(err.contains("invalid value 'banana' for '--variant"));
    }
}
