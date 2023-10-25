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
//     reviewers: ["cohenarthur", "p-e-", "philberty"],
//     labels: ["upstream"]
// ).create();

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string;
use std::time::Instant;

use chrono::Local;
use git2::{
    BranchType, Commit, Cred, Oid, PushOptions, RemoteCallbacks, Repository, Revwalk, Sort,
};
use log::{error, info, warn};
use octocrab::OctocrabBuilder;
use thiserror::Error;

pub struct UpstreamOpt {
    pub token: Option<String>,
    pub branch: String,
    pub gccrs: PathBuf,
    pub ssh: PathBuf,
}

#[derive(Debug, Error)]
pub enum Error {
    Io(#[from] io::Error),
    Utf8(#[from] string::FromUtf8Error),
    Git(#[from] git2::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self:#?}")
    }
}

fn init_workspace(gccrs: &Path) -> Result<Repository, Error> {
    // do we assume there's already a valid clone of gccrs here?

    Command::new("git").args(["remote", "add", "gcc", "git://gcc.gnu.org/git/gcc.git"]);

    info!("workspace: {}", gccrs.display());

    let repo = Repository::open(gccrs)?;

    let get_remote = |remote, url| {
        info!("fetching remote {remote} at {url}");
        repo.find_remote(remote)
            .or_else(|_| repo.remote(remote, url))
    };

    {
        let mut github = get_remote("github", "https://github.com/rust-gcc/gccrs")?;
        let mut gcc = get_remote("gcc", "git://gcc.gnu.org/git/gcc.git")?;

        github.fetch(&["master", "gcc-patch-dev"], None, None)?;
        gcc.fetch(&["master"], None, None)?;
    }

    Ok(repo)
}

fn last_prepared_commit(repo: &Repository, walker: &mut Revwalk) -> Result<String, Error> {
    let gcc_patch_dev = repo.find_branch("gcc-patch-dev", BranchType::Local)?;

    walker.push(gcc_patch_dev.get().target().unwrap())?;

    // FIXME: Remove all unwraps
    let last_prepared_commit = walker
        .find(|commit| {
            let commit = repo.find_commit(*commit.as_ref().unwrap()).unwrap();
            commit.message().unwrap().starts_with("gccrs: ")
        })
        .unwrap()
        .unwrap();

    let last_prepared_commit = repo.find_commit(last_prepared_commit).unwrap();

    info!("last commit prepared: {:?}", &last_prepared_commit);

    let last_commit_msg = last_prepared_commit.message().unwrap();
    // .strip_prefix("gccrs: ")
    // .unwrap();

    walker.reset()?;

    info!(
        "last prepared commit: {:?}",
        last_prepared_commit
            .message()
            .and_then(|msg| msg.lines().next())
    );

    Ok(String::from(last_commit_msg))
}

fn equivalent_github_commit(
    repo: &Repository,
    walker: &mut Revwalk,
    to_find: &str,
) -> Result<Oid, Error> {
    let github = repo
        .references()?
        .find(|reference| {
            reference.as_ref().unwrap().name().unwrap() == "refs/remotes/github/master"
        })
        .unwrap()
        .unwrap();

    let to_find = to_find.lines().next().unwrap();

    walker.push(github.target().unwrap())?;

    let ours = walker
        .find(|commit| {
            let msg = repo.find_commit(*commit.as_ref().unwrap()).unwrap();
            let msg = msg.message();

            msg.unwrap().lines().next() == Some(to_find)
        })
        .unwrap()?;

    Ok(ours)
}

fn prepare_branch(gccrs: &Path, ssh: &Path) -> Result<String, Error> {
    let repo = init_workspace(gccrs)?;
    let mut walker = repo.revwalk()?;
    let last_commit = last_prepared_commit(&repo, &mut walker)?;
    let ours = equivalent_github_commit(&repo, &mut walker, &last_commit)?;

    let gcc = repo
        .references()?
        .find(|reference| reference.as_ref().unwrap().name().unwrap() == "refs/remotes/gcc/master")
        .unwrap()
        .unwrap();

    info!("found equivalent commit: {ours}");

    walker.reset()?;
    // FIXME: Need to ignore merge commits
    walker.set_sorting(Sort::REVERSE)?;
    walker.hide(gcc.target().unwrap())?;
    walker.push_range(&format!("{ours}..refs/remotes/github/master"))?;
    // walker.push(ours)?;
    // walker.push(github.target().unwrap())?;

    // now we have the entire list of commits between our github remote and the latest pushed one
    // we need to figure out how to split them into two lists of commits - those which might need to be upstreamed later on and those which need to be upstreamed now
    // we can specify that behavior with a flag to gerris directly, and changing it in CI

    let start = Instant::now();
    info!("starting commit collection");

    let all_commits = walker
        .map(|commit| repo.find_commit(commit.unwrap()).unwrap())
        .collect::<Vec<Commit>>();

    let (rust_commits, maybe_to_stage) = all_commits
        .iter()
        // filter merge commits out - a merge commit is a commit
        // with more than one parent
        .filter(|commit| commit.parents().len() == 1)
        // we map each commit to the list of files it has touched
        .fold(
            (Vec::new(), Vec::new()),
            |(mut rust_commits, mut maybe_to_stage), commit| {
                // we can iter only on `commit.parent()` right?
                let parent = commit.parents().next().unwrap();
                let diff = repo
                    .diff_tree_to_tree(
                        Some(parent.tree().as_ref().unwrap()),
                        Some(commit.tree().as_ref().unwrap()),
                        None,
                    )
                    .unwrap();

                let touches_common_parts = diff
                    .deltas()
                    // Is that okay? What  about deleting common files
                    .filter_map(|delta| delta.new_file().path())
                    .any(|path| {
                        !(path.starts_with("gcc/rust/") || path.starts_with("gcc/testsuite/rust"))
                    });

                // FIXME: Should probably check if the commit's parent is in `maybe_to_stage`, in which case it
                // should be added there as well? Or is that invalid?

                if touches_common_parts {
                    maybe_to_stage.push(commit);
                } else {
                    rust_commits.push(commit);
                }

                (rust_commits, maybe_to_stage)
            },
        );

    let end = Instant::now();
    info!(
        "commit collection took {} seconds",
        (end - start).as_secs_f32()
    );

    warn!("bringing over {} commits", rust_commits.len());
    warn!("might need to stage {} commits", maybe_to_stage.len());

    let now = Local::now();
    let branch_name = format!("prepare-{}-{}", now.date_naive(), now.timestamp_micros());
    let gcc_patch_dev = repo
        .find_branch("gcc-patch-dev", BranchType::Local)
        .unwrap()
        .into_reference()
        .peel_to_commit()
        .unwrap();

    info!("creating branch {branch_name}");

    let branch = repo.branch(&branch_name, &gcc_patch_dev, true)?;
    repo.set_head(branch.into_reference().name().unwrap())?;

    rust_commits.iter().for_each(|commit| {
        println!("{}", commit.id());
    });

    rust_commits.into_iter().for_each(|commit| {
        // FIMXE: We need to edit the commit's message
        info!("cherry-picking {commit:?}");

        repo.cherrypick(commit, None).unwrap();

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let mut parents = vec![head];
        parents.append(&mut commit.parents().collect::<Vec<Commit>>());

        let parents = parents.iter().collect::<Vec<&Commit>>();

        let tree = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree).unwrap();

        repo.commit(
            Some("HEAD"),
            &commit.author(),
            &commit.committer(), // FIXME: Should this be gerris? me?
            &format!("gccrs: {}", commit.message().unwrap()),
            &tree,
            parents.as_slice(),
        )
        .unwrap();
    });

    let mut callbacks = RemoteCallbacks::new();
    callbacks
        .credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key(username_from_url.unwrap(), None, ssh, None)
        })
        .push_update_reference(|name, status| {
            if let Some(status) = status {
                error!("push from `{name}`` was rejected: `{status}`");
            } else {
                info!("push was ok! `{name}`");
            }

            Ok(())
        });

    let mut options = PushOptions::new();
    options.remote_callbacks(callbacks);
    let mut origin = repo.find_remote("origin")?;
    origin.push(&[&format!("refs/heads/{branch_name}")], Some(&mut options))?;

    Ok(branch_name)
}

pub async fn prepare_commits(
    UpstreamOpt {
        token,
        branch,
        gccrs,
        ssh,
    }: UpstreamOpt,
) -> Result<(), Error> {
    let new_branch = prepare_branch(&gccrs, &ssh)?;

    if let Some(token) = token {
        let instance = OctocrabBuilder::new()
            .personal_token(token)
            .build()
            .unwrap();

        instance
            .pulls("cohenarthur", "gccrs")
            .create(
                format!("Commits to upstream: {}", Local::now().date_naive()),
                new_branch,
                branch,
            )
            .body("Hey there! I'm gerris :)")
            .maintainer_can_modify(true)
            .send()
            .await
            .unwrap();
    } else {
        error!("no github token provided - skipping pull-request creation!")
    }

    todo!()
}
