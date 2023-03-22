// Run this by a cron job, for example every week
// This tool/subcommand only takes care of opening a PR containing the necessary commits to upstream
// Make sure that commits that touch something other than gcc/{,testsuite}/rust are marked clearly in the PR's message
//
// ## Logic/Pseudocode
//
// # there must be two different remotes: one which is GCC's upstream, and one which is ours on github
//
// where gcc = "gcc/master";
// where us = "github/master";
//
// # look at the latest commit on GCC which is ours -- has a "gccrs: " prefix
//
// # we must look for the message of the last commit so that we can find it on our branch
// # with this system sadly, shas are different between GCC's upstream and us
// where last_title = git.log().grep("gccrs: ").amount(1).on(gcc).msg;
// # how does this work if someone pushed a commit prefixed by "gccrs: " on `us`?
// where last_title = last_title.strip_leading("gccrs: ");
// where last_upstreamed_commit_us = git.log().grep(last_title).format(Hash).amount(2).on(us);
//
// # we now have the last commit on our remote which was pushed to GCC usptream
// # we can easily generate the rev-list of commits to prepare and push
// where rev_list = git.rev_list(last_upstreamed_commit, us);
//
// # let's create our branch which will contain these new prepared commits
// where branch = git.branch("prepare-{Date.today()}").create().rebase(us);
//
// # we can modify each of them to add the "gccrs: " prefix and check it
// rev_list.for_each(commit => {
//     git.cherry_pick(commit);
//     where msg = git.log().amount(1).format(Body);
//     git.commit.amend().message("gccrs: {msg}");
// })
//
// # figure out which commits might need to be skipped due to staging
// where maybe_skip = rev_list.filter(commit => {
//     # can this be done using git show -1 -- gcc/{,testsuite}/rust and checking the line amount?
//     !git.show(commit).amount(1).contains("gcc/rust") &&
//     !git.show(commit).amount(1).contains("gcc/testsuite/rust")
// });
// where msg = maybe_skip.fold(
//     "Careful: these commits touch on common GCC directories - they might need to be skipped due to the current GCC stage:\n",
//     (msg, commit) => "msg\n- {commit}"
// )
//
// # push our branch and create the PR
// git.push(branch).origin("github");
// PullRequest(
//     from: branch,
//     base: "gcc-patch-dev",
//     repo: "rust-gcc/gccrs",
//     message: msg,
//     reviewers: ["cohenarthur", "philberty"],
//     labels: ["upstream"]
// ).create();

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string;
use std::{env, error, io};

use chrono::Local;
use log::{info, warn};
use octocrab::OctocrabBuilder;

use crate::git;

pub struct UpstreamOpt {
    pub token: String,
    pub branch: String,
    pub gccrs: PathBuf,
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Utf8(string::FromUtf8Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self:#?}")
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<string::FromUtf8Error> for Error {
    fn from(err: string::FromUtf8Error) -> Self {
        Error::Utf8(err)
    }
}

fn init_workspace(gccrs: &Path) -> Result<(), Error> {
    // do we assume there's already a valid clone of gccrs here?

    // let current_dir = env::current_dir()?;
    info!("workspace: {}", gccrs.display());

    env::set_current_dir(gccrs)?;

    git::remote("gcc")
        .add("git://gcc.gnu.org/git/gcc.git")?
        .wait()?;
    git::remote("github")
        .add("https://github.com/rust-gcc/gccrs")?
        .wait()?;

    git::remote("gcc").fetch()?.wait()?;
    git::remote("github").fetch()?.wait()?;

    // env::set_current_dir(current_dir)?;

    Ok(())
}

pub async fn prepare_commits(
    UpstreamOpt {
        token,
        branch,
        gccrs,
    }: UpstreamOpt,
) -> Result<(), Error> {
    init_workspace(&gccrs)?;

    let last_commit = git::log()
        .grep("^gccrs: ")
        .format("%s")
        .amount(1)
        .branch("gcc/master")
        .cmd()?
        .wait_with_output()?
        .stdout;
    let last_commit = String::from_utf8(last_commit)?;

    info!("last commit upstreamed: {}", &last_commit);

    let last_commit = last_commit
        .strip_prefix("gccrs: ")
        .unwrap()
        .strip_suffix('\n')
        .unwrap();

    let ours = git::log()
        .grep(format!("^{last_commit}"))
        .amount(1)
        .branch("github/master")
        .not_on("gcc/master")
        .format("%h")
        .cmd()?
        .wait_with_output()?
        .stdout;
    let ours = String::from_utf8(ours)?;
    let ours = ours.strip_suffix('\n').unwrap();

    info!("found equivalent commit: {ours}");

    let instance = OctocrabBuilder::new()
        .personal_token(token)
        .build()
        .unwrap();

    let to_bring_over = git::rev_list(ours, "github/master")
        .not_on("gcc/master")
        .commits()?;

    warn!("bringing over {} commits", to_bring_over.len());

    let name = format!("prepare-{}", Local::now().date_naive());
    info!("creating branch: {name}");
    git::branch(&name).create()?.wait()?;

    to_bring_over
        .into_iter()
        .try_for_each(|commit| -> Result<(), Error> {
            let commit = git::commit(commit);

            commit.cherry_pick()?.wait()?;

            commit
                .amend()
                .message("gerris: I'm doing my very best!")
                .cmd()?
                .wait()?;

            Ok(())
        })?;

    Command::new("git")
        .arg("push")
        .arg("-u")
        .arg("origin")
        .arg("HEAD")
        .spawn()?
        .wait()?;

    instance
        .pulls("cohenarthur", "gccrs")
        .create("test", &name, branch)
        .body("Hey there! I'm gerris :)")
        .maintainer_can_modify(true)
        .send()
        .await
        .unwrap();

    todo!()
}
