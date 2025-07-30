use crate::git::{self, split_remote_branch, GitCmd};
use chrono::Local;
use log::{error, info, warn};
use octocrab::OctocrabBuilder;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io;
use std::path::PathBuf;
use std::string;
use thiserror::Error;

pub struct RebaseUpstreamOpt {
    pub token: Option<String>,
    pub github_project_owner: Option<String>,
    pub autosquash: bool,
    pub linearize_only: bool,
    pub no_fetch: bool,
    pub gcc_upstream_branch: String,
    pub gccrs_dev_branch: String,
    pub new_branch: String,
    pub gccrs: PathBuf,
    pub remote: Option<String>,
    pub ssh: PathBuf,
}

#[derive(Debug, Error)]
pub enum Error {
    Io(#[from] io::Error),
    Utf8(#[from] string::FromUtf8Error),
    Git(#[from] git::Error),
    InvalidInput,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self:#?}")
    }
}

pub async fn rebase_and_update(
    RebaseUpstreamOpt {
        token,
        github_project_owner,
        autosquash,
        linearize_only,
        no_fetch,
        gcc_upstream_branch,
        gccrs_dev_branch,
        new_branch,
        gccrs,
        remote,
        ssh: _ssh, // FIXME: Use ssh key for pushing
    }: RebaseUpstreamOpt,
) -> Result<(), Error> {
    std::env::set_current_dir(gccrs)?;

    if !no_fetch {
        git::maybe_fetch_from_branch(&gccrs_dev_branch)?;
        git::maybe_fetch_from_branch(&gcc_upstream_branch)?;
    } else {
        info!("Not fetching from remote.");
    }

    info!("Switch to new branch, starting from revision {gccrs_dev_branch}");
    git::switch(&new_branch)
        .create()
        .start_point(git::Revision(&gccrs_dev_branch))
        .force()
        .spawn()?;

    // find last merge commit
    let last_merge_commit = String::from_utf8(
        git::log()
            .amount(1)
            .merges(true)
            .format(git::Format::Hash)
            .spawn()?
            .stdout,
    )?;
    let last_merge_commit = last_merge_commit.trim().to_string();
    info!("Last merge commit is {last_merge_commit}");

    let head_rev = String::from_utf8(
        git::revparse()
            .revision(git::Revision("HEAD"))
            .spawn()?
            .stdout,
    )?;
    let head_rev = head_rev.trim().to_string();
    info!("Current revision of {gccrs_dev_branch}: {head_rev}");

    info!("Switching back to parent of {last_merge_commit}");
    git::switch(&new_branch)
        .create()
        .start_point(git::Revision(format!("{last_merge_commit}^")))
        .force()
        .spawn()?;

    // There are some commits after the merge, we need to apply them
    if head_rev != last_merge_commit {
        info!("Cherry picking commit in range {last_merge_commit}..{head_rev}");
        git::cherry_pick(git::Commit(format!("{last_merge_commit}..{head_rev}"))).spawn()?;
    }

    if linearize_only {
        info!("Not rebasing.");
    } else {
        let mut rebase_cmd = git::rebase(&gcc_upstream_branch);

        if autosquash {
            info!("Autosquash requested for upcoming rebase");
            rebase_cmd = rebase_cmd.interactive().autosquash();
        }

        info!("Rebasing the sequence onto {gcc_upstream_branch}");
        rebase_cmd.spawn()?;
    }

    info!("Creating new merge commit");

    let merge_commit_message = format!(
        "Merge remote-tracking branch '{gccrs_dev_branch}' into {new_branch}

This branch has a no-op merge as the last commit:
 - one arm is the \"current\" development branch from github
 - the other arm is a rebased version of the \"current\" master branch onto a recent GCC's master

The merge is obtained with \"git merge --strategy=ours\" to only keep the changes from second arm."
    );

    git::merge(&gccrs_dev_branch)
        .strategy("ours")
        .message(merge_commit_message)
        .spawn()?;

    // Maybe push the branch.
    // ... and if pushing the branch, maybe create a Pull Request
    if let Some(remote_for_push) = remote {
        info!("Pushing branch to {remote_for_push} {new_branch}");
        git::push()
            .remote(remote_for_push)
            .force()
            .refspec(format!("HEAD:{new_branch}"))
            .spawn()?;

        if let Some(token) = token {
            info!("creating pull-request...");
            let gh_owner = github_project_owner
                .expect("Missing github project owner for pull-request creation");

            let instance = OctocrabBuilder::new()
                .personal_token(token)
                .build()
                .unwrap();

            let (_, rem_branch) = if let Some((rem, br)) = split_remote_branch(&gccrs_dev_branch) {
                info!("Remote: {rem}, branch: {br}");
                (Some(rem), br)
            } else {
                info!("branch spec has no remote: {gccrs_dev_branch}");
                (None, gccrs_dev_branch.as_str())
            };

            info!("head: {new_branch}, base: {rem_branch}");

            let upstream_rev = String::from_utf8(
                git::revparse()
                    .revision(git::Revision(gcc_upstream_branch))
                    .spawn()?
                    .stdout,
            )?;

            let github_rev = String::from_utf8(
                git::revparse()
                    .revision(git::Revision(&gccrs_dev_branch))
                    .spawn()?
                    .stdout,
            )?;

            let pr_descr = format!("This is a sync with upstream GCC:\n - upstream GCC revision: {upstream_rev}\n - gccrs github: {github_rev}\n-- [gerris](https://github.com/Rust-GCC/gerris) 🦀\n");

            instance
                .pulls(gh_owner, "gccrs")
                .create(
                    format!(
                        "Sync with upstream ({}): {upstream_rev}",
                        Local::now().date_naive()
                    ),
                    &new_branch,
                    rem_branch,
                )
                .body(pr_descr)
                .maintainer_can_modify(true)
                .send()
                .await
                .unwrap();
        }
    }
    Ok(())
}
