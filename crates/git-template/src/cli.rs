//! CLI argument types for `git template`.

use std::path::PathBuf;

use clap::Parser;

/// Merge strategy option, mirroring `git merge -X`.
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum StrategyOption {
    /// Record conflict markers in the working tree (default).
    #[default]
    Normal,
    /// Resolve conflicts by favoring the local side.
    Ours,
    /// Resolve conflicts by favoring the upstream/template side.
    Theirs,
    /// Resolve conflicts by combining both sides.
    Union,
}

impl StrategyOption {
    /// Convert to the corresponding [`git2::FileFavor`].
    pub fn to_file_favor(self) -> git2::FileFavor {
        match self {
            StrategyOption::Normal => git2::FileFavor::Normal,
            StrategyOption::Ours => git2::FileFavor::Ours,
            StrategyOption::Theirs => git2::FileFavor::Theirs,
            StrategyOption::Union => git2::FileFavor::Union,
        }
    }
}

/// How the template repository's commit history appears in the local repo.
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum HistoryArg {
    /// Merge commit whose second parent is the template tip (default).
    #[default]
    Squash,
    /// Single linear commit with no upstream parent.
    Linear,
    /// Replay each upstream commit individually (not supported for `init`).
    Replay,
}

/// `git template` — create repositories from template repositories.
#[derive(Parser)]
#[command(name = "git template", bin_name = "git template")]
#[command(
    author,
    version,
    about = "Create new Git repositories from template repositories.",
    long_about = None
)]
pub struct Cli {
    /// Path to the git repository (defaults to the current directory).
    #[arg(short = 'C', long, global = true)]
    pub repo: Option<PathBuf>,

    #[command(subcommand)]
    /// The subcommand to run.
    pub command: Command,
}

/// Subcommands for `git template`.
#[derive(clap::Subcommand)]
pub enum Command {
    /// Initialize the current repository from a template.
    Init {
        /// URL of the template repository.
        url: String,

        /// Name for this template entry (defaults to the basename of the URL).
        #[arg(short, long)]
        name: Option<String>,

        /// Upstream branch or tag to use (defaults to HEAD).
        #[arg(short, long)]
        branch: Option<String>,

        /// How the template's commit history appears in the local repository.
        #[arg(long, value_enum, default_value_t)]
        history: HistoryArg,

        /// Keep `.gitvendors` and `.gitattributes` for future template syncing.
        #[arg(long)]
        keep_vendor: bool,

        /// Strategy option for merge conflict resolution.
        #[arg(short = 'X', long = "strategy-option", value_enum, default_value_t)]
        strategy_option: StrategyOption,
    },
}
