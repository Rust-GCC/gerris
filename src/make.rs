use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::shell::{self, Error, Output};

#[derive(Default, Debug)]
pub struct Make {
    directory: Option<String>,
    jobs: Option<usize>,
    load: Option<usize>,
    recipes: Vec<String>,
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
}

impl shell::Command for Make {
    fn spawn(self) -> Result<Output, Error> {
        let mut cmd = Command::new("make");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        self.directory.map(|d| cmd.arg("-C").arg(d));
        self.jobs.map(|jobs| cmd.arg(format!("-j{jobs}")));
        self.load.map(|load| cmd.arg(format!("-l{load}")));
        self.recipes.iter().for_each(|recipe| {
            cmd.arg(recipe);
        });

        Make::spawn_with_output(cmd)
    }
}
