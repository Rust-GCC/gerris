use super::{Branch, Format, GitCmd};
use std::process::Command;

// FIXME: Add a derive(Builder)
pub struct Mergebase {
    commit1: String,
    commit2: String,
}

pub fn merge_base<T1: Into<String>, T2: Into<String>>(commit1: T1, commit2: T2) -> Mergebase {
    Mergebase {
        commit1: commit1.into(),
        commit2: commit2.into(),
    }
}

impl GitCmd for Mergebase {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("merge-base");
        cmd.arg(self.commit1);
        cmd.arg(self.commit2);
    }
}
