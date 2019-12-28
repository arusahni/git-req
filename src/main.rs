///! GIT REQ!
mod git;
mod remotes;

use clap::{crate_authors, crate_version, load_yaml, App};
use colored::*;
use git2::ErrorCode;
use log::{debug, error, info, trace};
use std::io::{self, Write};
use std::{env, process};
use tabwriter::TabWriter;

/// Get the remote url
fn get_remote_url(remote_name: &str) -> String {
    git::get_remote_url(remote_name)
}

/// Get the remote for the current project
fn get_remote(remote_name: &str, fetch_api_key: bool) -> Result<Box<dyn remotes::Remote>, String> {
    let remote_url = get_remote_url(remote_name);
    remotes::get_remote(remote_name, &remote_url, !fetch_api_key)
}

/// Get the remote, fail hard otherwise
fn get_remote_hard(remote_name: &str, fetch_api_key: bool) -> Box<dyn remotes::Remote> {
    match get_remote(remote_name, fetch_api_key) {
        Ok(x) => x,
        Err(error) => {
            eprintln!(
                "{}",
                format!(
                    "There was a problem finding the remote Git repo: {}",
                    &error
                )
                .red()
            );
            process::exit(1);
        }
    }
}

/// Check out the branch corresponding to the MR ID and the remote's name
fn checkout_mr(remote_name: &str, mr_id: i64) {
    info!("Getting MR: {}", mr_id);
    let mut remote = get_remote_hard(remote_name, true);
    debug!("Found remote: {}", remote);
    let remote_branch_name = match remote.get_remote_req_branch(mr_id) {
        Ok(name) => name,
        Err(error) => {
            eprintln!(
                "{}",
                format!(
                    "There was a problem ascertaining the branch name: {}",
                    &error
                )
                .red()
            );
            process::exit(1);
        }
    };
    debug!("Got remote branch name: {}", remote_branch_name);
    match git::checkout_branch(
        remote_name,
        &remote_branch_name,
        &remote.get_local_req_branch(mr_id).unwrap(),
    ) {
        Ok(_) => {
            info!("Done!");
        }
        Err(error) => {
            eprintln!(
                "{}",
                format!("There was an error checking out the branch: {}", &error).red()
            );
            process::exit(1)
        }
    };
}

/// Clear the API key for the current domain
fn clear_domain_key(remote_name: &str) {
    trace!("Deleting domain key");
    let mut remote = get_remote_hard(remote_name, false);
    let deleted = match git::delete_req_config(&remote.get_domain(), "apikey") {
        Ok(_) => Ok(true),
        Err(e) => match e.code() {
            ErrorCode::NotFound => Ok(false),
            _ => Err(e),
        },
    };
    match deleted {
        Ok(_) => eprintln!("{}", "Domain key deleted!".green()),
        Err(e) => {
            error!("Git Config error: {}", e);
            eprintln!(
                "{}",
                format!(
                    "There was an error deleting the domain key: {}",
                    e.message()
                )
                .red()
            );
            process::exit(1)
        }
    }
}

/// Set the API key for the current domain
fn set_domain_key(remote_name: &str, new_key: &str) {
    trace!("Setting domain key: {}", new_key);
    let mut remote = get_remote_hard(remote_name, false);
    git::set_req_config(&remote.get_domain(), "apikey", new_key);
    eprintln!("{}", "Domain key changed!".green());
}

/// Delete the project ID entry
fn clear_project_id(remote_name: &str) {
    trace!("Deleting project ID for {}", remote_name);
    git::delete_config("projectid", remote_name);
    eprintln!("{}", "Project ID cleared!".green());
}

/// Set the project ID
fn set_project_id(remote_name: &str, new_id: &str) {
    trace!("Setting project ID: {} for remote: {}", new_id, remote_name);
    git::set_config("projectid", remote_name, new_id);
    eprintln!("{}", "New project ID set!".green());
}

/// Print the open requests
fn list_open_requests(remote_name: &str) {
    info!("Getting open requests");
    let mut remote = get_remote_hard(remote_name, true);
    debug!("Found remote: {}", remote);
    let mrs = remote.get_req_names().unwrap();
    let mut tw = TabWriter::new(io::stdout()).padding(4);
    for mr in &mrs {
        if remote.has_useful_branch_names() {
            writeln!(
                &mut tw,
                "{}\t{}\t{}",
                mr.id.to_string().green(),
                mr.source_branch.green().dimmed(),
                mr.title
            )
            .unwrap();
        } else {
            writeln!(&mut tw, "{}\t{}", mr.id.to_string().green(), mr.title).unwrap();
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

    let cfg = load_yaml!("../cli-flags.yml");
    let app = App::from_yaml(&cfg);

    let matches = app
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .get_matches();

    // Not using Clap's default_value because of https://github.com/clap-rs/clap/issues/1140
    let remote_name = matches.value_of("REMOTE_NAME").unwrap_or("origin");

    if let Some(project_id) = matches.value_of("NEW_PROJECT_ID") {
        set_project_id(remote_name, project_id);
    } else if matches.is_present("CLEAR_PROJECT_ID") {
        clear_project_id(remote_name);
    } else if matches.is_present("LIST_MR") {
        list_open_requests(remote_name);
    } else if matches.is_present("CLEAR_DOMAIN_KEY") {
        clear_domain_key(remote_name);
    } else if let Some(domain_key) = matches.value_of("NEW_DOMAIN_KEY") {
        set_domain_key(remote_name, domain_key);
    } else {
        checkout_mr(
            remote_name,
            matches.value_of("REQUEST_ID").unwrap().parse().unwrap(),
        );
    }
}
