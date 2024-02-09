use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use std::str;

use thiserror::Error;

#[derive(Default, Debug)]
pub struct Make {
    directory: Option<String>,
    jobs: Option<usize>,
    load: Option<usize>,
    recipes: Vec<String>,
}

// FIXME: Factor these two types in a common type for ::git and this module
#[derive(Debug)]
pub struct Output {
    pub status: process::ExitStatus,
    pub stdout: String,
    pub stderr: Vec<u8>,
}
#[derive(Debug, Error)]
pub enum Error {
    IO(#[from] io::Error),
    Status(process::Output),
    Utf8(#[from] str::Utf8Error),
}
// FIXME:
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:#?}")
    }
}
// FIXME:
impl TryFrom<process::Output> for Output {
    type Error = str::Utf8Error;

    fn try_from(out: process::Output) -> Result<Self, Self::Error> {
        let stdout = str::from_utf8(out.stdout.as_slice())?;
        let stdout = stdout.trim_end().to_string();

        Ok(Output {
            status: out.status,
            stderr: out.stderr,
            stdout,
        })
    }
}

pub fn new() -> Make {
    Make::default()
}

impl Make {
    pub fn directory(self, dir: impl Into<PathBuf>) -> Make {
        Make {
            directory: Some(dir.into().display().to_string()),
            ..self
        }
    }

    pub fn jobs(self, jobs: usize) -> Make {
        Make {
            jobs: Some(jobs),
            ..self
        }
    }

    pub fn load(self, load: usize) -> Make {
        Make {
            load: Some(load),
            ..self
        }
    }

    pub fn recipe(self, recipe: impl Into<String>) -> Make {
        let mut recipes = self.recipes;
        recipes.push(recipe.into());

        Make { recipes, ..self }
    }

    pub fn spawn(self) -> Result<Output, Error> {
        let mut cmd = Command::new("make");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        self.directory.map(|d| cmd.arg("-C").arg(d));
        self.jobs.map(|jobs| cmd.arg(format!("-j{jobs}")));
        self.load.map(|load| cmd.arg(format!("-l{load}")));
        self.recipes.iter().for_each(|recipe| {
            cmd.arg(recipe);
        });

        let output = cmd.spawn()?.wait_with_output()?;

        if output.status.success() {
            Ok(output.try_into()?)
        } else {
            Err(Error::Status(output))
        }
    }
}
