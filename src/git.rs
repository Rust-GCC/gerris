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

pub struct RevList {
    from: String,
    to: String,
    not_on: Vec<String>,
}

pub fn rev_list(from: impl Into<String>, to: impl Into<String>) -> RevList {
    RevList {
        from: from.into(),
        to: to.into(),
        not_on: vec![],
    }
}

impl RevList {
    pub fn not_on(mut self, not_on: impl Into<String>) -> RevList {
        RevList {
            not_on: {
                self.not_on.push(not_on.into());
                self.not_on
            },
            ..self
        }
    }

    pub fn cmd(self) -> Result<Child, Error> {
        info!("fetching revlist: {} -> {}", &self.from, &self.to);

        let mut cmd = Command::new("git");

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd.arg("rev-list").arg("--no-merges");

        let mut cmd = self.not_on.into_iter().fold(cmd, |mut cmd, not_on| {
            cmd.arg(&format!("^{not_on}"));
            cmd
        });

        cmd.arg(&format!("{}..{}", self.from, self.to));

        info!("{:?}", &cmd.get_args());

        cmd.spawn()
    }

    pub fn commits(self) -> Result<Vec<String>, Error> {
        let out = self.cmd()?.wait_with_output()?;
        // FIXME: No unwrap here
        let out = String::from_utf8(out.stdout).unwrap();

        // FIXME: This can return an iterator instead
        Ok(out.lines().map(str::to_owned).collect())
    }
}

pub struct Branch {
    name: String,
}

pub fn branch(name: impl Into<String>) -> Branch {
    Branch { name: name.into() }
}

impl Branch {
    pub fn create(self) -> Result<Child, Error> {
        let mut cmd = Command::new("git");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        cmd.arg("checkout").arg("-b").arg(self.name);

        cmd.spawn()
    }
}

pub struct Commit {
    hash: String,
}

pub fn commit(hash: impl Into<String>) -> Commit {
    Commit { hash: hash.into() }
}

impl Commit {
    pub fn cherry_pick(&self) -> Result<Child, Error> {
        info!("cherry-picking {}", &self.hash);

        let mut cmd = Command::new("git");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        cmd.arg("cherry-pick").arg(&self.hash);

        cmd.spawn()
    }

    pub fn amend(self) -> Amend {
        Amend {
            commit: self,
            msg: None,
        }
    }
}

pub struct Amend {
    commit: Commit,
    msg: Option<String>,
}

impl Amend {
    pub fn message(self, msg: impl Into<String>) -> Amend {
        Amend {
            msg: Some(msg.into()),
            ..self
        }
    }

    pub fn cmd(self) -> Result<Child, Error> {
        info!("amending with msg: {:#?}", self.msg);

        let mut cmd = Command::new("git");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        // FIXME: This should really use the commit here
        cmd.arg("amend");
        self.msg.map(|msg| cmd.arg("-m").arg(&msg));

        cmd.spawn()
    }
}
