use std::path::PathBuf;

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

        #[arg(short, long, help = "GitHub project owner")]
        github_project_owner: Option<String>,

        #[arg(long, help = "GCC upstream branch", default_value = "gnu/trunk")]
        gcc_upstream_branch: String,

        #[arg(long, help = "Do not update remotes")]
        no_fetch: bool,

        #[arg(
            long,
            help = "Only linearize commit sequence, do not rebase over updated upstream"
        )]
        linearize: bool,

        #[arg(
            long,
            help = "gccrs development branch",
            default_value = "gccrs/master"
        )]
        gccrs_dev_branch: String,

        #[arg(long, help = "autosquash fixup commits")]
        autosquash: bool,

        #[arg(long, help = "New branch gerris will create for the new pull-request")]
        to: String,

        #[arg(short, long, help = "Push the branch to the specified remote")]
        push: Option<String>,

        #[arg(
            short,
            long,
            help = "work directory which contains a copy of the gccrs respository"
        )]
        #[arg(short, long, help = "ssh key to use when pushing created branches")]
        ssh: PathBuf,

        work: PathBuf,
    },
    /// Create a PR on `gccrs`'s repository containing the commits from master which haven't yet
    /// been formatted properly for upstreaming.
    Upstream {
        #[arg(short, long, help = "GitHub token to perform actions as gerris")]
        token: Option<String>,

        #[arg(short, long, help = "GitHub project owner")]
        github_project_owner: Option<String>,

        #[arg(
            long,
            help = "Branch on which to base the pull-request gerris will create"
        )]
        to: String,

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
        push: Option<String>,

        #[arg(short, long, help = "ssh key to use when pushing created branches")]
        ssh: PathBuf,
    },
}

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: SubCmd,
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
            linearize,
            no_fetch,
            gccrs_dev_branch,
            autosquash,
            to: new_branch,
            work,
            push,
            ssh,
        } => {
            rebaseupstream::rebase_and_update(rebaseupstream::RebaseUpstreamOpt {
                token,
                github_project_owner,
                autosquash,
                linearize,
                no_fetch,
                gcc_upstream_branch,
                gccrs_dev_branch,
                new_branch,
                gccrs: work,
                push_to: push,
                ssh,
            })
            .await?
        }

        SubCmd::Upstream {
            token,
            github_project_owner,
            no_fetch,
            no_rebase,
            to,
            gcc_upstream_branch,
            gccrs_dev_branch,
            work,
            push,
            ssh,
        } => {
            upstream::prepare_commits_bis(upstream::UpstreamOpt {
                token,
                github_project_owner,
                no_fetch,
                no_rebase,
                new_branch: to,
                gcc_upstream_branch,
                gccrs_dev_branch,
                gccrs: work,
                push_to: push,
                ssh,
            })
            .await?
        }
    }

    Ok(())
}
