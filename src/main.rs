#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate duct;
extern crate git2;
extern crate regex;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate clap;
extern crate shellexpand;

mod remotes;
mod git;

use clap::{App, Arg};

fn get_origin() -> String {
    git::get_repo_info("remote.origin.url")
}

fn checkout_mr(mr_id: i64) {
    info!("Getting MR: {}", mr_id);
    let origin = get_origin();
    let mut remote = match remotes::get_remote(&origin) {
        Ok(x) => x,
        Err(error) => {
            panic!("There was a problem finding the remote Git repo: {:?}", error)
        }
    };
    debug!("Found remote: {}", remote);
    let branch_name = remote.get_req_branch(&mr_id);
    debug!("Got branch name: {}", branch_name);
    match git::checkout_branch(&branch_name) {
        Ok(_) => {
            info!("Done!")
        },
        Err(error) => {
            panic!("There was an error checking out the branch: {:?}", &error)
        }
    }
}

fn main() {
    let _ = env_logger::try_init();
    let matches = App::new("git-req")
                        .version("0.1")
                        .author("Aru Sahni <arusahni@gmail.com>")
                        .about("Switch between merge/pull requests in your GitLab/GitHub repositories with just the request ID.")
                        .arg(Arg::with_name("REQUEST_ID")
                                .required(true)
                                .index(1))
                        .get_matches();
    checkout_mr(matches.value_of("REQUEST_ID").unwrap().parse().unwrap());
}
