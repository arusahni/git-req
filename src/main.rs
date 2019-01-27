///! GIT REQ!
mod git;
mod remotes;

use clap::{App, Arg};
use log::{debug, info};
use std::process;

/// Get the `origin` remote
fn get_origin() -> String {
    git::get_repo_info("remote.origin.url")
}

/// Check out the branch corresponding to the MR ID
fn checkout_mr(mr_id: i64) {
    info!("Getting MR: {}", mr_id);
    let origin = get_origin();
    let mut remote = match remotes::get_remote(&origin) {
        Ok(x) => x,
        Err(error) => {
            eprintln!("There was a problem finding the remote Git repo: {}", &error);
            process::exit(1);
        }
    };
    debug!("Found remote: {}", remote);
    let branch_name = remote.get_req_branch(mr_id);
    debug!("Got branch name: {}", branch_name);
    match git::checkout_branch(&branch_name) {
        Ok(_) => {
            info!("Done!");
        }
        Err(error) => {
            eprintln!("There was an error checking out the branch: {}", &error);
            process::exit(1)
        }
    };
}

/// Do the thing
fn main() {
    let _ = env_logger::try_init();
    let matches = App::new("git-req")
        .version("0.1")
        .author("Aru Sahni <arusahni@gmail.com>")
        .about(
            "Switch between merge/pull requests in your GitLab/GitHub repositories with just the request ID.",
        )
        .arg(Arg::with_name("REQUEST_ID").required(true).index(1))
        .get_matches();
    checkout_mr(matches.value_of("REQUEST_ID").unwrap().parse().unwrap());
}
