use super::{Branch, Format, GitCmd};

use std::process::Command;

// FIXME: Add a derive(Builder)
#[derive(Default)]
pub struct Log {
    amount: Option<usize>,
    branch: Option<String>,
    grep: Option<String>,
    format: Option<Format>,
    merges: Option<bool>,
}

pub fn log() -> Log {
    Log::default()
}

impl Log {
    pub fn amount(self, amount: usize) -> Log {
        Log {
            amount: Some(amount),
            ..self
        }
    }

    pub fn branch<T: Into<String>>(self, Branch(branch): Branch<T>) -> Log {
        Log {
            branch: Some(branch.into()),
            ..self
        }
    }

    pub fn grep<T: Into<String>>(self, grep: T) -> Log {
        Log {
            grep: Some(grep.into()),
            ..self
        }
    }

    pub fn format(self, format: Format) -> Log {
        Log {
            format: Some(format),
            ..self
        }
    }
    pub fn merges<T: Into<bool>>(self, merges: T) -> Log {
        Log {
            merges: Some(merges.into()),
            ..self
        }
    }
}

impl GitCmd for Log {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("log");

        self.amount.map(|x| cmd.arg(format!("-{x}")));
        self.grep.map(|s| cmd.arg("--grep").arg(s));
        self.format
            .map(|f| cmd.arg(format!("--format={}", f.as_str())));
        self.branch.map(|b| cmd.arg(b));
        self.merges.map(|b| {
            if b {
                cmd.arg("--merges")
            } else {
                cmd.arg("--no-merges")
            }
        });
    }
}
