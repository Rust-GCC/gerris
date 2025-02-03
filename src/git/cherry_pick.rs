use std::process::Command;

use super::{Commit, GitCmd};

#[derive(Default)]
pub struct CherryPick {
    commit: String,
}

pub fn cherry_pick<T: Into<String>>(Commit(commit): Commit<T>) -> CherryPick {
    CherryPick {
        commit: commit.into(),
    }
}

impl GitCmd for CherryPick {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("cherry-pick").arg(self.commit);
    }
}

#[derive(Default)]
pub struct CherryPickAbort;

pub fn cherry_pick_abort() -> CherryPickAbort {
    CherryPickAbort
}

impl GitCmd for CherryPickAbort {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("cherry-pick").arg("--abort");
    }
}
