use serde::Deserialize;
use std::fs;
use std::io::Error;

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

#[derive(Debug, Deserialize)]
struct Config {
    work: Option<String>,
    github_project_owner: Option<String>,
    gcc_upstream_branch: Option<String>,
    gccrs_dev_branch: Option<String>,
    remote: Option<String>,
    token_file: Option<String>,
    to_branch: Option<String>,
    no_fetch: Option<bool>,
    linearize_only: Option<bool>,
    autosquash: Option<bool>,
    github_upstream_base: Option<String>,
    no_rebase: Option<bool>,
    add_missing_prefix: Option<bool>,
}

impl Config {
    fn new() -> Self {
        Config {
            work: None,
            github_project_owner: None,
            gcc_upstream_branch: None,
            gccrs_dev_branch: None,
            remote: None,
            token_file: None,
            to_branch: None,
            no_fetch: None,
            linearize_only: None,
            autosquash: None,
            github_upstream_base: None,
            no_rebase: None,
            add_missing_prefix: None,
        }
    }

    fn to_upstreamopt(
        &mut self,
        token_file: Option<String>,
        github_project_owner: Option<String>,
        github_upstream_base: Option<String>,
        no_fetch: u8,
        no_rebase: u8,
        new_branch: Option<String>,
        gcc_upstream_branch: Option<String>,
        gccrs_dev_branch: Option<String>,
        gccrs: Option<PathBuf>,
        remote: Option<String>,
        add_missing_prefix: u8,
    ) -> Result<upstream::UpstreamOpt, Error> {
        let token = if let Some(tf) = token_file {
            Some(fs::read_to_string(&tf)?.trim().to_string())
        } else if let Some(tf) = &self.token_file {
            Some(fs::read_to_string(&tf)?.trim().to_string())
        } else {
            None
        };
        Ok(upstream::UpstreamOpt {
            token,
            github_project_owner: github_project_owner.or(self.github_project_owner.take()),
            github_upstream_base: github_upstream_base.or(self.github_upstream_base.take()),
            no_fetch: no_fetch > 0 || self.no_fetch.map_or(false, |v| v),
            no_rebase: no_rebase > 0 || self.no_rebase.map_or(false, |v| v),
            new_branch: new_branch
                .or(self.to_branch.take())
                .or(Some(create_new_branch_name("rebase-upstream")))
                .unwrap(),
            gcc_upstream_branch: gcc_upstream_branch
                .or(self.gcc_upstream_branch.take())
                .or(Some("gnu/trunk".to_string()))
                .unwrap(),
            gccrs_dev_branch: gccrs_dev_branch
                .or(self.gccrs_dev_branch.take())
                .or(Some("upstream-gccrs/master".to_string()))
                .unwrap(),
            gccrs: gccrs.or(self.work.take().map(PathBuf::from)).unwrap(),
            remote: remote.or(self.remote.take()),
            add_missing_prefix: add_missing_prefix > 0
                || self.add_missing_prefix.map_or(false, |v| v),
        })
    }

    fn to_rebaseupstreamopt(
        &mut self,
        token_file: Option<String>,
        github_project_owner: Option<String>,
        autosquash: u8,
        linearize_only: u8,
        no_fetch: u8,
        gcc_upstream_branch: Option<String>,
        gccrs_dev_branch: Option<String>,
        new_branch: Option<String>,
        gccrs: Option<PathBuf>,
        remote: Option<String>,
    ) -> Result<rebaseupstream::RebaseUpstreamOpt, Error> {
        let token = if let Some(tf) = token_file {
            Some(fs::read_to_string(&tf)?.trim().to_string())
        } else if let Some(tf) = &self.token_file {
            Some(fs::read_to_string(&tf)?.trim().to_string())
        } else {
            None
        };
        println!("{:#?}", self);

        Ok(rebaseupstream::RebaseUpstreamOpt {
            token: token,
            github_project_owner: github_project_owner.or(self.github_project_owner.take()),
            autosquash: autosquash > 0 || self.autosquash.map_or(false, |v| v),
            linearize_only: linearize_only > 0 || self.linearize_only.map_or(false, |v| v),
            no_fetch: no_fetch > 0 || self.no_fetch.map_or(false, |v| v),
            gcc_upstream_branch: gcc_upstream_branch
                .or(self.gcc_upstream_branch.take())
                .or(Some("gnu/trunk".to_string()))
                .unwrap(),
            gccrs_dev_branch: gccrs_dev_branch
                .or(self.gccrs_dev_branch.take())
                .or(Some("upstream-gccrs/master".to_string()))
                .unwrap(),
            new_branch: new_branch
                .or(self.to_branch.take())
                .or(Some(create_new_branch_name("rebase")))
                .unwrap(),
            gccrs: gccrs.or(self.work.take().map(PathBuf::from)).unwrap(),
            remote: remote.or(self.remote.take()),
        })
    }
}

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
        #[arg(
            short,
            long,
            help = "Path to file containing the GitHub token to perform actions as gerris"
        )]
        token_file: Option<String>,

        #[arg(long, help = "GitHub project owner")]
        github_project_owner: Option<String>,

        #[arg(long, help = "GCC upstream branch")]
        gcc_upstream_branch: Option<String>,

        #[arg(long, help = "Do not update remotes", action = clap::ArgAction::Count)]
        no_fetch: u8,

        #[arg(
            long,
            help = "Only linearize commit sequence, do not rebase over updated upstream",
            action = clap::ArgAction::Count
        )]
        linearize_only: u8,

        #[arg(long, help = "gccrs development branch")]
        gccrs_dev_branch: Option<String>,

        #[arg(long, help = "autosquash fixup commits", action = clap::ArgAction::Count)]
        autosquash: u8,

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
        work: Option<PathBuf>,
    },
    /// Create a PR on `gccrs`'s repository containing the commits from master which haven't yet
    /// been formatted properly for upstreaming.
    Upstream {
        #[arg(
            short,
            long,
            help = "Path to file containing the GitHub token to perform actions as gerris"
        )]
        token_file: Option<String>,

        #[arg(long, help = "GitHub project owner")]
        github_project_owner: Option<String>,

        #[arg(
            long,
            help = "Name for the base branch to be used for the pull-request. This should be an upstream branch."
        )]
        github_upstream_base: Option<String>,

        #[arg(
            long,
            help = "Force branch name that gerris will create for the new pull-request (uses today's date by default)"
        )]
        to_branch: Option<String>,

        #[arg(long, help = "Do not update remotes", action = clap::ArgAction::Count)]
        no_fetch: u8,

        #[arg(long, help = "Do not rebase onto latest upstream GCC", action = clap::ArgAction::Count)]
        no_rebase: u8,

        #[arg(long, help = "GCC upstream branch")]
        gcc_upstream_branch: Option<String>,

        #[arg(long, help = "gccrs development branch.")]
        gccrs_dev_branch: Option<String>,

        #[arg(
            short,
            long,
            help = "Work directory which contains a copy of the gccrs respository"
        )]
        work: Option<PathBuf>,

        #[arg(short, long, help = "Push the branch to the specified remote")]
        remote: Option<String>,

        #[arg(long, help = "Add missing 'gccrs: ' prefix to commit when missing", action = clap::ArgAction::Count)]
        add_missing_prefix: u8,
    },
}

#[derive(Parser)]
struct Args {
    #[arg(long, help = "TOML config file")]
    config: Option<String>,

    #[command(subcommand)]
    cmd: SubCmd,
}

fn create_new_branch_name(topic: &str) -> String {
    format!("gerris/{topic}/{}", Local::now().date_naive())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    env_logger::init();

    let mut conf = if let Some(toml_config) = args.config {
        let content = std::fs::read_to_string(toml_config)?;
        toml::from_str(&content)?
    } else {
        Config::new()
    };

    match args.cmd {
        SubCmd::ChangeLogs => clog::check_clog_checker_output()?,
        SubCmd::Rebase {
            token_file,
            github_project_owner,
            gcc_upstream_branch,
            linearize_only,
            no_fetch,
            gccrs_dev_branch,
            autosquash,
            to_branch: new_branch,
            work,
            remote,
        } => {
            let sconf = conf.to_rebaseupstreamopt(
                token_file,
                github_project_owner,
                autosquash,
                linearize_only,
                no_fetch,
                gcc_upstream_branch,
                gccrs_dev_branch,
                new_branch,
                work,
                remote,
            )?;

            rebaseupstream::rebase_and_update(sconf).await?
        }

        SubCmd::Upstream {
            token_file,
            github_project_owner,
            github_upstream_base,
            no_fetch,
            no_rebase,
            to_branch: new_branch,
            gcc_upstream_branch,
            gccrs_dev_branch,
            work,
            remote,
            add_missing_prefix,
        } => {
            let token = if let Some(tf) = token_file {
                Some(fs::read_to_string(&tf)?.trim().to_string())
            } else {
                None
            };

            let sconf = conf.to_upstreamopt(
                token,
                github_project_owner,
                github_upstream_base,
                no_fetch,
                no_rebase,
                new_branch,
                gcc_upstream_branch,
                gccrs_dev_branch,
                work,
                remote,
                add_missing_prefix,
            )?;

            upstream::prepare_commits_bis(sconf).await?
        }
    }

    Ok(())
}
