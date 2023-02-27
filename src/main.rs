use clap::{Parser, Subcommand};

// FIXME: Add env_logger, would fit quite nicely here
// FIXME: Or should we? Is the goal to compile it asap using gccrs?
// FIXME: If not, use nom instead of the hand-written combinator

mod clog;
mod parser;
mod upstream;

#[derive(Clone, Copy, Subcommand)]
enum SubCmd {
    /// Check the output of GCC's changelog checker (`contrib/gcc-changelog/git_check_commit.py`)
    /// on a range of commit and post a message on GitHub indicating the necessary changes. This
    /// subcommand takes the output of the above mentioned script as input on `stdin`.
    ChangeLogs,
    /// Create a PR on `gccrs`'s repository containing the commits from master which haven't yet
    /// been formatted properly for upstreaming.
    Upstream,
}

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: SubCmd,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.cmd {
        SubCmd::ChangeLogs => clog::check_clog_checker_output()?,
        SubCmd::Upstream => upstream::prepare_commits()?,
    }

    Ok(())
}
