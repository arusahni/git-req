///! GIT REQ!
mod git;
mod remotes;

use clap::{crate_authors, crate_version, App, Arg};
use log::{debug, info, trace};
use std::io::{self, Write};
use std::{env, process};
use tabwriter::TabWriter;

/// Get the `origin` remote
fn get_origin() -> String {
    git::get_remote_url("origin")
}

/// Get the remote for the current project
fn get_remote() -> Result<Box<remotes::Remote>, String> {
    let origin = get_origin();
    remotes::get_remote(&origin)
}

/// Get the remote, fail hard otherwise
fn get_remote_hard() -> Box<remotes::Remote> {
    match get_remote() {
        Ok(x) => x,
        Err(error) => {
            eprintln!(
                "There was a problem finding the remote Git repo: {}",
                &error
            );
            process::exit(1);
        }
    }
}

/// Check out the branch corresponding to the MR ID
fn checkout_mr(mr_id: i64) {
    info!("Getting MR: {}", mr_id);
    let mut remote = get_remote_hard();
    debug!("Found remote: {}", remote);
    let branch_name = match remote.get_req_branch(mr_id) {
        Ok(name) => name,
        Err(error) => {
            eprintln!(
                "There was a problem ascertaining the branch name: {}",
                &error
            );
            process::exit(1);
        }
    };
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

/// Delete the project ID entry
fn clear_project_id() {
    trace!("Deleting project ID");
    git::delete_config("projectid");
    eprintln!("Project ID cleared!");
}

/// Set the project ID
fn set_project_id(new_id: &str) {
    trace!("Setting project ID: {}", new_id);
    git::set_config("projectid", new_id);
    eprintln!("New project ID set!");
}

/// Print the open requests
fn list_open_requests() {
    info!("Getting open requests");
    let mut remote = get_remote_hard();
    debug!("Found remote: {}", remote);
    let mrs = remote.get_req_names().unwrap();
    let mut tw = TabWriter::new(io::stdout()).padding(4);
    for mr in &mrs {
        if remote.has_useful_branch_names() {
            writeln!(&mut tw, "{}\t{}\t{}",  mr.id, mr.source_branch, mr.title).unwrap();
        } else {
            writeln!(&mut tw, "{}\t{}",  mr.id, mr.title).unwrap();
        }
    }
    tw.flush().unwrap();
}

/// Do the thing
fn main() {
    color_backtrace::install();
    let _ = env_logger::Builder::new()
        .parse_filters(&env::var("REQ_LOG").unwrap_or_default())
        .try_init();
    let matches = App::new("git-req")
        .bin_name("git req")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .about(
            "Switch between merge/pull requests in your GitLab and GitHub repositories with just the request ID.",
        )
        .arg(Arg::with_name("LIST_MR")
             .long("list")
             .short("l")
             .help("List all open requests against the repository")
             .takes_value(false)
             .required(false)
             .conflicts_with_all(&["CLEAR_PROJECT_ID", "NEW_PROJECT_ID", "REQUEST_ID"]))
        .arg(Arg::with_name("NEW_PROJECT_ID")
             .long("set-project-id")
             .value_name("PROJECT_ID")
             .help("A project ID for the current repository")
             .required(false)
             .takes_value(true)
             .conflicts_with_all(&["CLEAR_PROJECT_ID", "LIST_MR", "REQUEST_ID"]))
        .arg(Arg::with_name("CLEAR_PROJECT_ID")
             .long("clear-project-id")
             .help("A project ID for the current repository")
             .required(false)
             .takes_value(false)
             .conflicts_with_all(&["REQUEST_ID", "LIST_MR", "NEW_PROJECT_ID"]))
        .arg(Arg::with_name("REQUEST_ID")
             .required(true)
             .conflicts_with_all(&["NEW_PROJECT_ID", "LIST_MR", "CLEAR_PROJECT_ID"])
             .index(1))
        .get_matches();
    if let Some(project_id) = matches.value_of("NEW_PROJECT_ID") {
        set_project_id(project_id);
    } else if matches.is_present("CLEAR_PROJECT_ID") {
        clear_project_id();
    } else if matches.is_present("LIST_MR") {
        list_open_requests();
    } else {
        checkout_mr(matches.value_of("REQUEST_ID").unwrap().parse().unwrap());
    }
}
