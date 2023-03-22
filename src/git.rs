//! This module aims at abstracting some common git operations such as cloning, updating, and checking out particular commits

use std::io::Error;
use std::process::{Child, Command, Stdio};

use log::info;

pub struct Checkout;

#[derive(Default, Debug)]
#[must_use]
pub struct Log {
    branch: Option<String>,
    grep: Option<String>,
    amount: Option<u32>,
    // TODO: Use strong typing here with a `Format` enum
    format: Option<String>,
    not_on: Vec<String>,
}

pub fn checkout() -> Checkout {
    Checkout
}

pub fn log() -> Log {
    Log::default()
}

impl Log {
    pub fn branch(self, branch: impl Into<String>) -> Log {
        Log {
            branch: Some(branch.into()),
            ..self
        }
    }

    pub fn format(self, format: impl Into<String>) -> Log {
        Log {
            format: Some(format.into()),
            ..self
        }
    }

    pub fn grep(self, regex: impl Into<String>) -> Log {
        Log {
            grep: Some(regex.into()),
            ..self
        }
    }

    pub fn amount(self, amount: u32) -> Log {
        Log {
            amount: Some(amount),
            ..self
        }
    }

    pub fn not_on(mut self, not_on: impl Into<String>) -> Log {
        Log {
            not_on: {
                self.not_on.push(not_on.into());
                self.not_on
            },
            ..self
        }
    }

    pub fn cmd(self) -> Result<Child, Error> {
        let mut cmd = Command::new("git");

        info!("starting git log: {:?}", &self);

        cmd.arg("log").stdout(Stdio::piped()).stderr(Stdio::piped());

        self.branch.map(|branch| cmd.arg(&branch));
        self.grep.map(|regex| cmd.arg("--grep").arg(&regex));
        self.amount.map(|amount| cmd.arg(&format!("-{amount}")));
        self.format.map(|fmt| cmd.arg(&format!("--format={fmt}")));
        let mut cmd = self.not_on.into_iter().fold(cmd, |mut cmd, not_on| {
            cmd.arg(&format!("^{not_on}"));
            cmd
        });

        info!("{:?}", &cmd.get_args());

        cmd.spawn()
    }
}

#[derive(Default)]
#[must_use]
pub struct Remote {
    name: String,
    url: Option<String>,
}

pub fn remote(name: impl Into<String>) -> Remote {
    Remote {
        name: name.into(),
        url: None,
    }
}

impl Remote {
    pub fn add(self, url: &str) -> Result<Child, Error> {
        info!("adding remote `{}` at `{url}`", &self.name);

        Command::new("git")
            .arg("remote")
            .arg("add")
            .arg(self.name)
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }

    pub fn fetch(self) -> Result<Child, Error> {
        info!("fetching remote `{}`", &self.name);

        Command::new("git")
            .arg("fetch")
            .arg(self.name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }
}
