use std::path::PathBuf;

use chrono::Local;
use clap::{Parser, Subcommand};

// FIXME: Add env_logger, would fit quite nicely here
// FIXME: Or should we? Is the goal to compile it asap using gccrs?
// FIXME: If not, use nom instead of the hand-written combinator

mod clog;
pub mod git;
mod parser;
mod rebaseupstream;
mod upstream;

#[derive(Clone, Subcommand)]
enum SubCmd {
    /// Check the output of GCC's changelog checker (`contrib/gcc-changelog/git_check_commit.py`)
    /// on a range of commit and post a message on GitHub indicating the necessary changes. This
    /// subcommand takes the output of the above mentioned script as input on `stdin`.
    ChangeLogs,
    /// Updates current branch by rebasing over upstream branch and moving all
    /// commits following the latest merge within the existing sequence located
    /// before the merge.
    Rebase {
        #[arg(short, long, help = "GitHub token to perform actions as gerris")]
        token: Option<String>,

        #[arg(long, help = "GitHub project owner", default_value = "Rust-GCC")]
        github_project_owner: Option<String>,

        #[arg(long, help = "GCC upstream branch", default_value = "gnu/trunk")]
        gcc_upstream_branch: String,

        #[arg(long, help = "Do not update remotes")]
        no_fetch: bool,

        #[arg(
            long,
            help = "Only linearize commit sequence, do not rebase over updated upstream"
        )]
        linearize_only: bool,

        #[arg(
            long,
            help = "gccrs development branch",
            default_value = "gccrs/master"
        )]
        gccrs_dev_branch: String,

        #[arg(long, help = "autosquash fixup commits")]
        autosquash: bool,

        #[arg(
            long,
            help = "Force branch name that gerris will create for the new pull-request (uses today's date by default)"
        )]
        to_branch: Option<String>,

        #[arg(short, long, help = "Push the branch to the specified remote")]
        remote: Option<String>,

        #[arg(
            short,
            long,
            help = "work directory which contains a copy of the gccrs respository"
        )]
        work: PathBuf,

        #[arg(short, long, help = "ssh key to use when pushing created branches")]
        ssh: PathBuf,
    },
    /// Create a PR on `gccrs`'s repository containing the commits from master which haven't yet
    /// been formatted properly for upstreaming.
    Upstream {
        #[arg(short, long, help = "GitHub token to perform actions as gerris")]
        token: Option<String>,

        #[arg(long, help = "GitHub project owner", default_value = "Rust-GCC")]
        github_project_owner: Option<String>,

        #[arg(
            long,
            help = "Name for the base branch to be used for the pull-request. This should be an upstream branch.",
            default_value = "gcc-patch-dev"
        )]
        github_upstream_base: Option<String>,

        #[arg(
            long,
            help = "Force branch name that gerris will create for the new pull-request (uses today's date by default)"
        )]
        to_branch: Option<String>,

        #[arg(long, help = "Do not update remotes")]
        no_fetch: bool,

        #[arg(long, help = "Do not rebase onto latest upstream GCC")]
        no_rebase: bool,

        #[arg(long, help = "GCC upstream branch", default_value = "gnu/trunk")]
        gcc_upstream_branch: String,

        #[arg(
            long,
            help = "gccrs development branch.",
            default_value = "gccrs/master"
        )]
        gccrs_dev_branch: String,

        #[arg(
            short,
            long,
            help = "Work directory which contains a copy of the gccrs respository"
        )]
        work: PathBuf,

        #[arg(short, long, help = "Push the branch to the specified remote")]
        remote: Option<String>,

        #[arg(short, long, help = "ssh key to use when pushing created branches")]
        ssh: PathBuf,
    },
}

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: SubCmd,
}

fn create_new_branch_name(topic: &str) -> String {
    format!("gerris/{topic}/{}", Local::now())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    env_logger::init();

    match args.cmd {
        SubCmd::ChangeLogs => clog::check_clog_checker_output()?,
        SubCmd::Rebase {
            token,
            github_project_owner,
            gcc_upstream_branch,
            linearize_only,
            no_fetch,
            gccrs_dev_branch,
            autosquash,
            to_branch: new_branch,
            work,
            remote,
            ssh,
        } => {
            rebaseupstream::rebase_and_update(rebaseupstream::RebaseUpstreamOpt {
                token,
                github_project_owner,
                autosquash,
                linearize_only,
                no_fetch,
                gcc_upstream_branch,
                gccrs_dev_branch,
                new_branch: new_branch.map_or(create_new_branch_name("rebase"), |s| s),
                gccrs: work,
                remote,
                ssh,
            })
            .await?
        }

        SubCmd::Upstream {
            token,
            github_project_owner,
            github_upstream_base,
            no_fetch,
            no_rebase,
            to_branch: new_branch,
            gcc_upstream_branch,
            gccrs_dev_branch,
            work,
            remote,
            ssh,
        } => {
            upstream::prepare_commits_bis(upstream::UpstreamOpt {
                token,
                github_project_owner,
                github_upstream_base,
                no_fetch,
                no_rebase,
                new_branch: new_branch.map_or(create_new_branch_name("upstream"), |s| s),
                gcc_upstream_branch,
                gccrs_dev_branch,
                gccrs: work,
                remote,
                ssh,
            })
            .await?
        }
    }

    Ok(())
}
