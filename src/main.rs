///! GIT REQ!
mod cli;
mod git;
mod remotes;

use anyhow::Result;
use clap::{Command, CommandFactory, Parser};
use clap_complete::{generate, Generator};
use colored::*;
use git2::ErrorCode;
use log::{debug, error, info, trace};
use std::io::{self, stdin, stdout, Write};
use std::{env, process};
use tabwriter::TabWriter;

use cli::Cli;

fn abort(message: &str) -> ! {
    eprintln!("{}", message.red());
    process::exit(1);
}

/// Get the remote for the current project
fn get_remote(remote_name: &str, fetch_api_key: bool) -> Result<Box<dyn remotes::Remote>> {
    let remote_url = git::get_remote_url(remote_name);
    remotes::get_remote(remote_name, &remote_url, !fetch_api_key)
}

/// Get the remote, fail hard otherwise
fn get_remote_hard(remote_name: &str, fetch_api_key: bool) -> Box<dyn remotes::Remote> {
    get_remote(remote_name, fetch_api_key).unwrap_or_else(|error| {
        let message = format!(
            "There was a problem finding the remote Git repo: {}",
            &error
        );
        abort(&message);
    })
}

/// Check out the branch corresponding to the MR ID and the remote's name
fn checkout_mr(remote_name: &str, mr_id: i64) {
    info!("Getting MR: {}", mr_id);
    let mut remote = get_remote_hard(remote_name, true);
    debug!("Found remote: {}", remote);
    let remote_branch_name = remote.get_remote_req_branch(mr_id).unwrap_or_else(|error| {
        let message = format!(
            "There was a problem ascertaining the branch name: {}",
            &error
        );
        abort(&message);
    });
    debug!("Got remote branch name: {}", remote_branch_name);
    match git::checkout_branch(
        remote_name,
        &remote_branch_name,
        &remote.get_local_req_branch(mr_id).unwrap(),
        remote.has_virtual_remote_branch_names(),
    )
    .unwrap_or_else(|err| {
        let message = format!("There was an error checking out the branch: {}", err);
        abort(&message);
    }) {
        git::CheckoutResult::BranchChanged => {
            if git::push_current_ref(mr_id).is_err() {
                trace!("Couldn't update the current ref");
                eprintln!("{}", "failed to update some git-req metadata".yellow());
            }
        }
        _ => {
            eprintln!("Already on branch");
        }
    };
    trace!("Done");
}

/// Clear the API key for the current domain
fn clear_domain_key(remote_name: &str) {
    trace!("Deleting domain key");
    let mut remote = get_remote_hard(remote_name, false);
    let deleted = match git::delete_req_config(remote.get_domain(), "apikey") {
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
            let message = format!(
                "There was an error deleting the domain key: {}",
                e.message()
            );
            abort(&message);
        }
    }
}

/// Set the API key for the current domain
fn set_domain_key(remote_name: &str, new_key: &str) {
    trace!("Setting domain key: {}", new_key);
    let mut remote = get_remote_hard(remote_name, false);
    git::set_req_config(remote.get_domain(), "apikey", new_key);
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

/// Set the default remote for the repository
fn set_default_remote(remote_name: &str) {
    trace!("Setting default remote {}", remote_name);
    git::set_project_config("defaultremote", remote_name);
    eprintln!("{}", "New default remote set!".green());
}

/// Print the open requests
fn list_open_requests(remote_name: &str) {
    info!("Getting open requests");
    let mut remote = get_remote_hard(remote_name, true);
    debug!("Found remote: {}", remote);
    let mrs = remote.get_req_names().unwrap_or_else(|error| {
        let message = format!("There was a problem querying the open reqs: {}", &error);
        abort(&message);
    });
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

fn print_completions<G: Generator>(generator: G, cmd: &mut Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut io::stdout(),
    );
}

/// Get the name of the remote to use for the operation
fn get_remote_name(remote_override: Option<String>) -> String {
    let default_remote_name = git::get_project_config("defaultremote").unwrap_or_else(|| {
        let new_remote_name = git::guess_default_remote_name().unwrap_or_else(|_| {
            let mut new_remote_name = String::new();
            println!("Multiple remotes detected. Enter the name of the default one.");
            for remote in git::get_remotes() {
                println!(" * {}", remote);
            }
            print!("Remote name: ");
            let _ = stdout().flush();
            stdin()
                .read_line(&mut new_remote_name)
                .expect("Did not input a name");
            new_remote_name = new_remote_name.trim().to_string();
            trace!("New remote: {}", &new_remote_name);
            if !git::get_remotes().contains(&new_remote_name) {
                abort("Invalid remote name provided")
            }
            new_remote_name
        });
        git::set_project_config("defaultremote", &new_remote_name);
        new_remote_name
    });
    // Not using Clap's default_value because of https://github.com/clap-rs/clap/issues/1140
    remote_override.unwrap_or(default_remote_name)
}

/// Do the thing
fn main() {
    color_backtrace::install();
    let _ = env_logger::Builder::new()
        .parse_filters(&env::var("REQ_LOG").unwrap_or_default())
        .try_init();

    let cli = Cli::parse();

    if let Some(project_id) = cli.new_project_id {
        set_project_id(&get_remote_name(cli.remote_name), &project_id);
    } else if cli.clear_project_id {
        clear_project_id(&get_remote_name(cli.remote_name));
    } else if cli.list {
        list_open_requests(&get_remote_name(cli.remote_name));
    } else if cli.clear_domain_key {
        clear_domain_key(&get_remote_name(cli.remote_name));
    } else if let Some(domain_key) = cli.new_domain_key {
        set_domain_key(&get_remote_name(cli.remote_name), &domain_key);
    } else if let Some(remote_name) = cli.new_default_remote {
        set_default_remote(&remote_name);
    } else if let Some(generator) = cli.generate_completions {
        let mut cmd = Cli::command();
        print_completions(generator, &mut cmd);
    } else {
        let request_id = cli.request_id.unwrap_or_else(|| {
            abort("Request ID required");
        });
        let mr_id = if request_id == "-" {
            trace!("Received request for previous MR");
            git::get_previous_mr_id().unwrap_or_else(|_| {
                abort("Could not find previous request");
            })
        } else {
            trace!("Received request for numbered MR: {}", request_id);
            request_id.parse::<i64>().unwrap_or_else(|_| {
                abort("Invalid request ID provided");
            })
        };
        checkout_mr(&get_remote_name(cli.remote_name), mr_id);
    }
}
