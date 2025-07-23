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

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;
use std::path::PathBuf;
use std::string;

use chrono::Local;
use log::{error, info, warn};
use octocrab::OctocrabBuilder;
use thiserror::Error;

use crate::git::{self, split_remote_branch, GitCmd};

pub struct UpstreamOpt {
    pub token: Option<String>,
    pub github_project_owner: Option<String>,
    pub no_fetch: bool,
    pub no_rebase: bool,
    pub new_branch: String,
    pub gcc_upstream_branch: String,
    pub gccrs_dev_branch: String,
    pub gccrs: PathBuf,
    pub push_to: Option<String>,
    pub ssh: PathBuf,
}

#[derive(Debug, Error)]
pub enum Error {
    Io(#[from] io::Error),
    Utf8(#[from] string::FromUtf8Error),
    Git(#[from] git2::Error),
    Gitv2(#[from] git::Error),
    NotAMerge,
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

pub async fn prepare_commits(
    UpstreamOpt {
        token,
        github_project_owner: _gh,
        no_fetch,
        no_rebase,
        new_branch,
        gcc_upstream_branch,
        gccrs_dev_branch,
        gccrs,
        push_to: _push_to,
        ssh: _ssh, // FIXME: Use ssh key for pushing
    }: UpstreamOpt,
) -> Result<(), Error> {
    // let _ = CdRaii::change_path(gccrs);
    std::env::set_current_dir(gccrs)?;

    if !no_fetch {
        info!("fetching `upstream`...");
        git::fetch().remote("upstream").spawn()?;

        info!("fetching `gcc`...");
        git::fetch().remote("gcc").spawn()?;
    } else {
        info!("Not fetching from remotes");
    }

    let last_upstreamed_commit = git::log()
        .amount(1)
        .grep("gccrs: ")
        .branch(git::Branch(gcc_upstream_branch))
        .format(git::Format::Title)
        .spawn()?;
    let last_upstreamed_commit = String::from_utf8(last_upstreamed_commit.stdout)?;

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
        .spawn()?;
    let last_commit_us = String::from_utf8(last_commit_us.stdout)?;
    let last_commit_us = last_commit_us.trim_end();

    info!("found equivalent commit: {}", last_commit_us);

    let rev_list = git::rev_list(last_commit_us, "upstream/master")
        .no_merges()
        .reverse()
        .exclude(git::Branch("gcc/trunk"))
        .dir("gcc/rust")
        .dir("gcc/testsuite/rust")
        .dir("libgrust")
        .spawn()?;
    let rev_list = String::from_utf8(rev_list.stdout)?;

    warn!("found {} commits to upstream", rev_list.lines().count());

    let now = Local::now();
    let new_branch = format!("prepare-{}-{}", now.date_naive(), now.timestamp_micros());
    git::branch()
        .name(&new_branch)
        .starting_point(git::StartingPoint::Branch("gcc/trunk"))
        .spawn()?;
    git::switch(&new_branch).spawn()?;

    info!("created branch `{new_branch}`");

    rev_list.lines().try_for_each(|commit| {
        info!("cherry-picking {commit}...");
        git::cherry_pick(git::Commit(commit)).spawn().map(|_| ())
    })?;

    info!("pushing branch...");
    std::process::Command::new("git")
        .args(["push", "-u", "origin", "HEAD"])
        .spawn()?
        .wait_with_output()?;

    if let Some(token) = token {
        info!("creating pull-request...");

        let instance = OctocrabBuilder::new()
            .personal_token(token)
            .build()
            .unwrap();

        instance
            .pulls("cohenarthur", "gccrs")
            .create(
                format!("Commits to upstream: {}", Local::now().date_naive()),
                new_branch,
                &gccrs_dev_branch,
            )
            .body("Hey there! I'm gerris :)")
            .maintainer_can_modify(true)
            .send()
            .await
            .unwrap();
    } else {
        error!("no github token provided - skipping pull-request creation!")
    }

    Ok(())
}

pub async fn prepare_commits_bis(
    UpstreamOpt {
        token,
        github_project_owner,
        no_fetch,
        no_rebase,
        new_branch,
        gcc_upstream_branch,
        gccrs_dev_branch,
        gccrs,
        push_to,
        ssh: _ssh, // FIXME: Use ssh key for pushing
    }: UpstreamOpt,
) -> Result<(), Error> {
    std::env::set_current_dir(gccrs)?;

    if !no_fetch {
        info!("fetching `gccrs`...");
        git::maybe_fetch_from_branch(&gccrs_dev_branch)?;

        info!("fetching `upstream gcc`...");
        git::maybe_fetch_from_branch(&gcc_upstream_branch)?;
    } else {
        info!("Not fetching from remotes");
    }

    let last_commit_parents = String::from_utf8(
        git::show(&gccrs_dev_branch)
            .no_patch()
            .format("%P")
            .spawn()?
            .stdout,
    )?;

    let is_merge = last_commit_parents.split(" ").collect::<Vec<_>>().len() > 1;
    if !is_merge {
        info!("Last commit is not a merge, can't do anything.");
        return Err(Error::NotAMerge);
    } else {
        info!("Last commit is a merge, continuing.");
    }

    info!("Switch to new branch, starting from revision {gccrs_dev_branch}^");
    git::switch(&new_branch)
        .create()
        .start_point(git::Revision(format!("{gccrs_dev_branch}^")))
        .force()
        .spawn()?;

    let merge_base = String::from_utf8(
        git::merge_base(&gcc_upstream_branch, &new_branch)
            .spawn()?
            .stdout,
    )?;
    let merge_base = merge_base.trim().to_string();
    info!("Merge base is {merge_base}");

    let rev_list_all = git::rev_list(&merge_base, &new_branch).reverse();
    let all_commits = String::from_utf8(rev_list_all.spawn()?.stdout)?;
    let mut all_commits: Vec<_> = all_commits.split('\n').collect();
    let all_commits_count = all_commits.len();

    let do_not_upstream = vec![
        ".github/",
        "CODE_OF_CONDUCT.md",
        "CONTRIBUTING.md",
        "Dockerfile",
        "README.md",
        "logo.png",
        "gcc/rust/gource-gccrs.sh",
        "gcc/rust/monthly-diff.py",
    ];
    let rev_list_to_drop = git::rev_list(&merge_base, &new_branch).dirs(do_not_upstream);

    let commits_to_drop = String::from_utf8(rev_list_to_drop.spawn()?.stdout)?;
    let commits_to_drop: BTreeSet<_> = commits_to_drop.split('\n').collect();

    all_commits.retain(|rev| !commits_to_drop.contains(rev));

    info!("All commits: {all_commits_count}");
    info!(
        "Commits to drop {}: {:?}",
        commits_to_drop.len(),
        commits_to_drop
    );
    info!(
        "Commits to apply: {}/{all_commits_count} [-{}]{:?}",
        all_commits.len(),
        commits_to_drop.len(),
        all_commits
    );

    if no_rebase {
        info!("Not rebasing, reseting branch to merge-base {merge_base}");
        git::switch(&new_branch)
            .create()
            .start_point(git::Revision(merge_base))
            .force()
            .spawn()?;
    } else {
        info!("Rebasing onto new gcc upstream, reseting branch to {gcc_upstream_branch}");
        git::switch(&new_branch)
            .create()
            .start_point(git::Revision(gcc_upstream_branch))
            .force()
            .spawn()?;
    }

    info!("Applying commits over upstream base");
    for commit in all_commits {
        info!("Applying {commit}...");
        git::cherry_pick(git::Commit(commit)).spawn()?;
    }
    info!("Done applying");

    if let Some(remote_for_push) = push_to {
        info!("Pushing branch to {remote_for_push} {new_branch}");
        git::push()
            .remote(remote_for_push)
            .force()
            .refspec(format!("HEAD:{new_branch}"))
            .spawn()?;
    }
    if let Some(token) = token {
        info!("creating pull-request...");
        let gh_owner =
            github_project_owner.expect("Missing github project owner for pull-request creation");

        let instance = OctocrabBuilder::new()
            .personal_token(token)
            .build()
            .unwrap();

        let (remote, rem_branch) = if let Some((rem, br)) = split_remote_branch(&gccrs_dev_branch) {
            info!("Remote: {rem}, branch: {br}");
            (Some(rem), br)
        } else {
            info!("branch spec has no remote: {gccrs_dev_branch}");
            (None, gccrs_dev_branch.as_str())
        };

        info!("head: {new_branch}, base: {rem_branch}");

        instance
            .pulls(gh_owner, "gccrs")
            .create(
                format!("Commits to upstream: {}", Local::now().date_naive()),
                &new_branch,
                rem_branch,
            )
            .body("Hey there! I'm gerris 🦀")
            .maintainer_can_modify(true)
            .send()
            .await
            .unwrap();
    }

    Ok(())
}
