use std::path::PathBuf;

use clap::{Parser, Subcommand};

// FIXME: Add env_logger, would fit quite nicely here
// FIXME: Or should we? Is the goal to compile it asap using gccrs?
// FIXME: If not, use nom instead of the hand-written combinator

mod clog;
mod git;
mod github;
mod make;
mod parser;
mod shell;
mod upstream;

#[derive(Clone, Subcommand)]
enum SubCmd {
    /// Check the output of GCC's changelog checker (`contrib/gcc-changelog/git_check_commit.py`)
    /// on a range of commit and post a message on GitHub indicating the necessary changes. This
    /// subcommand takes the output of the above mentioned script as input on `stdin`.
    ChangeLogs,
    /// Create a PR on `gccrs`'s repository containing the commits from master which haven't yet
    /// been formatted properly for upstreaming.
    Upstream {
        #[arg(short, long, help = "GitHub token to perform actions as gerris")]
        token: Option<String>,

        #[arg(
            long,
            help = "branch on which to base the pull-request gerris will create"
        )]
        to: String,

        #[arg(
            short,
            long,
            help = "work directory which contains a copy of the gccrs respository"
        )]
        work: PathBuf,

        #[arg(
            short,
            long,
            help = "repository on which to submit the GitHub pull-request"
        )]
        repo: String,
    },
    PullRequest {
        #[arg(short, long, help = "branch from which to create the pull-request")]
        branch: String,

        #[arg(short, long, help = "GitHub token to perform actions as gerris")]
        token: String,

        #[arg(
            long,
            help = "branch on which to base the pull-request gerris will create"
        )]
        to: String,

        #[arg(
            short,
            long,
            help = "work directory which contains a copy of the gccrs respository and the branch you would like to create a pull-request from (--branch)"
        )]
        work: PathBuf,

        #[arg(
            short,
            long,
            help = "repository on which to submit the GitHub pull-request"
        )]
        repo: String,
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
        SubCmd::Upstream {
            token,
            to,
            work,
            repo,
        } => {
            upstream::prepare_commits(upstream::UpstreamOpt {
                token,
                branch: to,
                gccrs: work,
                repo,
            })
            .await?
        }
        SubCmd::PullRequest {
            branch,
            token,
            to,
            work,
            repo,
        } => upstream::create_pull_request(token, repo, branch, to, work).await?,
    }

    Ok(())
}
