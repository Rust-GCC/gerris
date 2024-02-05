use super::{Branch, GitCmd};

use std::process::Command;

#[derive(Default)]
pub struct RevList {
    start: String,
    end: String,
    prevent_merges: bool,
    reverse: bool,
    exclude: Option<String>,
    dirs: Vec<String>,
}

// FIXME: Use newtypes here
pub fn rev_list<T1: Into<String>, T2: Into<String>>(start: T1, end: T2) -> RevList {
    RevList {
        start: start.into(),
        end: end.into(),
        ..RevList::default()
    }
}

impl RevList {
    pub fn no_merges(self) -> RevList {
        RevList {
            prevent_merges: true,
            ..self
        }
    }

    pub fn reverse(self) -> RevList {
        RevList {
            reverse: true,
            ..self
        }
    }

    pub fn exclude<T: Into<String>>(self, Branch(branch): Branch<T>) -> RevList {
        RevList {
            exclude: Some(branch.into()),
            ..self
        }
    }

    pub fn dir<T: Into<String>>(self, to_add: T) -> RevList {
        let mut dirs = self.dirs;
        dirs.push(to_add.into());

        RevList { dirs, ..self }
    }

    pub fn dirs<T: Into<String>>(self, dirs: Vec<T>) -> RevList {
        RevList {
            dirs: dirs.into_iter().map(Into::into).collect(),
            ..self
        }
    }
}

impl GitCmd for RevList {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("rev-list")
            .arg(format!("{}..{}", self.start, self.end));

        if self.reverse {
            cmd.arg("--reverse");
        }
        if self.prevent_merges {
            cmd.arg("--no-merges");
        }

        self.exclude
            .map(|to_exclude| cmd.arg(format!("^{to_exclude}")));

        if !self.dirs.is_empty() {
            cmd.arg("--");
        }
        self.dirs.iter().for_each(|dir| {
            cmd.arg(dir);
        });
    }
}
