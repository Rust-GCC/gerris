//! Github specific part of `gerris` - this module concerns the formatting of the PR's body, as well as any operations
//! related to interacting with Github.

use crate::upstream::BuildError;

const FAILURE: &str = "❌";
const SUCCESS: &str = "✅";

pub fn prepare_body(
    last_commit: String, /* FIXME: Should this be a Commit type? */
    commits: Vec<(&str, Option<BuildError>)>,
) -> String {
    let tab = String::from("|Commit|Build|Test|\n|---|:-:|:-:|");

    let tab = commits.iter().fold(tab, |tab, (commit, result)| {
        let (build_result, test_result) = match result {
            Some(BuildError::Build) => (FAILURE, FAILURE),
            Some(BuildError::Tests) => (SUCCESS, FAILURE),
            _ => (SUCCESS, SUCCESS),
        };

        format!("{tab}\n|{commit}|{build_result}|{test_result}|")
    });

    // TODO: Put this in a const somewhere. Cleanup that file overall
    format!(
        "
This pull-request aims to help upstreaming commits to the GCC repository by formatting them \
and checking that they can be cherry-picked/rebased properly.

The last commit upstreamed was:

`{}`
        
The list of commits prepared is as follows:
        
{}
        
🐙
",
        last_commit, tab
    )
}
