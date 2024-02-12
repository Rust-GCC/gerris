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
//
// # FIXME: how does this work if someone pushed a commit prefixed by "gccrs: " on `us`?
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
// rev_list.for_each(commit -> {
//     git.cherry_pick(commit);
//     where msg = git.log().amount(1).format(Body);
//     git.commit.amend().message("gccrs: {msg}");
// })
//
// # figure out which commits might need to be skipped due to staging
// where maybe_skip = rev_list.filter(commit -> {
//     # can this be done using git show -1 -- gcc/{,testsuite}/rust and checking the line amount?
//     !git.show(commit).amount(1).contains("gcc/rust") &&
//     !git.show(commit).amount(1).contains("gcc/testsuite/rust")
// });
// where msg = maybe_skip.fold(
//     "Careful: these commits touch on common GCC directories - they might need to be skipped due to the current GCC stage:\n",
//     (msg, commit) -> "msg\n- {commit}"
// )
//
// # push our branch and create the PR
// git.push(branch).origin("github");
// PullRequest(
//     from: branch,
//     base: "gcc-patch-dev",
//     repo: "rust-gcc/gccrs",
//     message: msg,
//     reviewers: ["cohenarthur", "p-e-p", "philberty"],
//     labels: ["upstream"]
// ).create();

// shell script equivalent:
//
// git fetch gcc
// git fetch upstream
// # FIXME: The remotes need to exist already
//
// last_pushed_commit=$(git log -1 --grep "gccrs: " gcc/trunk --format="title")
// last_msg = last_pushed_commit.strip_prefix("gccrs: ");
// last_commit_us=$(git log -1 --grep $last_msg upstream/master --format="%h")
// rev_list=$(git rev-list --no-merges $last_commit_us..upstream/master ^gcc/trunk -- gcc/rust/ libgrust/ gcc/testsuite/rust)
//
// git checkout -b $date
// for commit in rev_list.lines()
//     git cherry-pick $commit
//
// git push -u origin HEAD
// create_pr()

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;
use std::path::PathBuf;
use std::string;

use chrono::Local;
use log::{error, info, warn};
use octocrab::OctocrabBuilder;
use thiserror::Error;

use crate::git;
use crate::make;
use crate::shell::{self, Command};

pub struct UpstreamOpt {
    pub token: Option<String>,
    pub branch: String,
    pub gccrs: PathBuf,
    pub repo: String,
}

#[derive(Debug, Error)]
pub enum Error {
    Io(#[from] io::Error),
    Utf8(#[from] string::FromUtf8Error),
    Shell(#[from] shell::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self:#?}")
    }
}

// shell script equivalent:
//
// git fetch gcc
// git fetch upstream
// # FIXME: The remotes need to exist already
//
// last_pushed_commit=$(git log -1 --grep "gccrs: " gcc/trunk --format="title")
// last_msg = last_pushed_commit.strip_prefix("gccrs: ");
// last_commit_us=$(git log -1 --grep $last_msg upstream/master --format="%h")
// rev_list=$(git rev-list --no-merges $last_commit_us..upstream/master ^gcc/trunk -- gcc/rust/ libgrust/ gcc/testsuite/rust)
//
// git checkout -b $date
// for commit in rev_list.lines()
//     git cherry-pick $commit
//
// git push -u origin HEAD
// create_pr()

fn maybe_prefix_cherry_picked_commit() -> Result<(), Error> {
    let msg = git::log()
        .amount(1)
        .format(git::Format::Body)
        .spawn()?
        .stdout;

    let commit = gccrs_tools::Commit::new(msg);

    if let gccrs_tools::Commit::NeedsPrefixing(_) = commit {
        let new_msg = commit.maybe_prefix();

        info!("commit needs prefixing... adding `gccrs: ` prefix");
        git::commit().amend().message(new_msg).spawn()?;
    }

    Ok(())
}

enum BuildError {
    Build,
    Tests,
}

fn build_prefixed_commit() -> Result<(), BuildError> {
    let build_dir = PathBuf::from("build-gerris");

    info!("building last applied commit...");
    make::new()
        .directory(&build_dir)
        .jobs(14)
        .load(6)
        .spawn()
        .map_err(|_| BuildError::Build)?;

    info!("testing last applied commit...");
    let test_output = make::new()
        .directory(&build_dir)
        .recipe("check-rust")
        .jobs(14)
        .load(6)
        .spawn()
        .map_err(|_| BuildError::Tests)?
        .stdout;

    if test_output.contains("unexpected") || test_output.contains("unresolved") {
        warn!("unexpected test failure in last commit");
        Err(BuildError::Tests)
    } else {
        Ok(())
    }
}

fn prepare_body(last_commit: String, result_tab: String) -> String {
    format!(
        "
This pull-request aims to help upstreaming commits to the GCC repository by formatting them \
and checking that they can be cherry-picked/rebased properly.

The last commit upstreamed was:

`{last_commit}`
        
The list of commits prepared is as follows:
        
{result_tab}
        
🐙
        "
    )
}

pub async fn prepare_commits(
    UpstreamOpt {
        token,
        branch,
        gccrs,
        repo,
    }: UpstreamOpt,
) -> Result<(), Error /* FIXME: Should this return another Error type then */> {
    // let _ = CdRaii::change_path(gccrs);
    std::env::set_current_dir(&gccrs)?;

    info!("fetching `upstream`...");
    git::fetch().remote("upstream").spawn()?;

    info!("fetching `gcc`...");
    git::fetch().remote("gcc").spawn()?;

    let last_upstreamed_commit = git::log()
        .amount(1)
        .grep("gccrs: ")
        .branch(git::Branch("gcc/trunk"))
        .format(git::Format::Title)
        .spawn()?
        .stdout;

    info!("found last upstreamed commit: {}", last_upstreamed_commit);

    let last_msg = last_upstreamed_commit
        .strip_prefix("gccrs: ")
        .unwrap()
        .trim_end();

    let last_commit_us = git::log()
        .amount(1)
        .grep(last_msg)
        .branch(git::Branch("upstream/master"))
        .grep(last_msg)
        .format(git::Format::Hash)
        .spawn()?
        .stdout;

    info!("found equivalent commit: {}", last_commit_us);

    let rev_list = git::rev_list(last_commit_us, "upstream/master")
        .no_merges()
        .reverse()
        .exclude(git::Branch("gcc/trunk"))
        .dir("gcc/rust")
        .dir("gcc/testsuite/rust")
        .dir("libgrust")
        .spawn()?
        .stdout;

    warn!("found {} commits to upstream", rev_list.lines().count());

    let now = Local::now();
    let new_branch = format!("prepare-{}-{}", now.date_naive(), now.timestamp_micros());
    git::branch()
        .name(&new_branch)
        .starting_point(git::StartingPoint::Branch("gcc/trunk"))
        .spawn()?;
    git::switch(&new_branch).spawn()?;

    info!("created branch `{new_branch}`");

    let tab = String::from("|Commit|Build|Test|\n|---|:-:|:-:|");

    let tab = rev_list.lines().fold(tab, |tab, commit| {
        info!("cherry-picking {commit}...");

        // FIXME: Can we unwrap here?
        git::cherry_pick(git::Commit(commit))
            .spawn()
            .expect("couldn't cherry pick commit");

        let checks = build_prefixed_commit();

        enum CheckResult {
            BuildFailure,
            TestFailure,
            AllOk,
        }

        let result = match checks {
            Ok(_) => CheckResult::AllOk,
            Err(BuildError::Build) => CheckResult::BuildFailure,
            Err(BuildError::Tests) => CheckResult::TestFailure,
        };

        // FIXME: Can we unwrap here?
        maybe_prefix_cherry_picked_commit().expect("couldn't prefix commit");

        let build_result = match result {
            CheckResult::BuildFailure => "❌",
            CheckResult::TestFailure | CheckResult::AllOk => "✅",
        };

        let test_result = match result {
            CheckResult::AllOk => "✅",
            _ => "❌",
        };

        format!("{tab}\n|{commit}|{build_result}|{test_result}|")
    });

    info!("pushing branch...");
    git::push()
        .upstream(git::Remote("origin"))
        // TODO: Rename? This should be .refspec()?
        .branch(git::Branch("HEAD"))
        .spawn()?;

    if let Some(token) = token {
        info!("creating pull-request...");

        let instance = OctocrabBuilder::new()
            .personal_token(token)
            .build()
            .unwrap();

        instance
            .pulls(repo, "gccrs")
            .create(
                format!("[upstream] [{}] Prepare commits", Local::now().date_naive()),
                // FIXME: Will branches always be created and pushed from my fork? Add CLI parameter for this maybe?
                format!("cohenarthur:{new_branch}"),
                branch,
            )
            .body(prepare_body(last_upstreamed_commit, tab))
            .maintainer_can_modify(true)
            .send()
            .await
            .unwrap();
    } else {
        error!("no github token provided - skipping pull-request creation!")
    }

    Ok(())
}
