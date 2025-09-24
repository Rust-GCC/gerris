# Gerris

Gerris is a :crab: trying to help with gccrs.

## Configuration
The easiest way to use `gerris` is to create a config file similar to:

``` toml
# common part
work = "/path/to/some/gccrs/clone/gccrs"      ## existing gccrs clone that gerris can take over
github_project_owner = "github-project-owner" ## Rust-GCC for example
gcc_upstream_branch = "gnu/master"            ## remote/branch for the upstream GCC project
gccrs_dev_branch = "upstream-gccrs/master"    ## remote/branch for the gccrs dev branch
remote = "dkm"                                ## git remote where branches are to be pushed
token_file = "dkm_repo.txt"                   ## github token

# to-branch =

# for rebase only
no_fetch = true             ## if true, gerris won't `git fetch` and only use current state
linearize_only = false      ## only "bubble up" the merge commit
autosquash = false          ## when rebasing, should gerris also squash fixup commits?

# for upstream only
github_upstream_base = "dkm/upstream-base"  ## branch name used to store current gcc upstream revision 
no_rebase = true                            ## if true, don't rebase

```

## Sync with upstream gcc

``` sh
 $ gerris --config config.toml rebase
```

## Create branch for upstreaming

``` sh
 $ gerris --config config.toml upstream
```
