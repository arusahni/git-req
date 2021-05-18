///! GIT REQ!
mod git;
mod remotes;

use anyhow::Result;
use clap::{crate_authors, crate_version, App, AppSettings, ArgMatches, YamlLoader};
use colored::*;
use git2::ErrorCode;
use lazy_static::lazy_static;
use log::{debug, error, info, trace};
use logchop::*;
use std::io::{self, stdin, stdout, Cursor, Write};
use std::{env, include_str, process};
use tabwriter::TabWriter;
use yaml_rust::Yaml;

lazy_static! {
    static ref APP_CFG: Yaml = {
        YamlLoader::load_from_str(include_str!("../cli-flags.yml"))
            .expect("Failed to load CLI config")
            .pop()
            .unwrap()
    };
}

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
    git::checkout_branch(
        remote_name,
        &remote_branch_name,
        &remote.get_local_req_branch(mr_id).unwrap(),
        remote.has_virtual_remote_branch_names(),
    )
    .info_ok("Done")
    .unwrap_or_else(|err| {
        eprintln!(
            "{}",
            format!("There was an error checking out the branch: {}", err).red()
        );
        process::exit(1)
    });
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

/// Print a shell completion script
fn generate_completion(app: &mut App, shell_name: &str) {
    let mut buffer = Cursor::new(Vec::new());
    app.gen_completions_to("git-req", shell_name.parse().unwrap(), &mut buffer);
    let mut output = String::from_utf8(buffer.into_inner()).unwrap_or_else(|_| String::from(""));
    if shell_name == "zsh" {
        // Clap assigns the _files completor to the REQUEST_ID. This is undesirable.
        output = output.replace(":_files", ":");
    }
    print!("{}", &output);
}

/// Get the name of the remote to use for the operation
fn get_remote_name(matches: &ArgMatches) -> String {
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
            trace!("New remote: {}", &new_remote_name);
            if !git::get_remotes().contains(new_remote_name.trim()) {
                panic!("Invalid remote name provided")
            }
            new_remote_name
        });
        git::set_project_config("defaultremote", &new_remote_name);
        new_remote_name
    });
    // Not using Clap's default_value because of https://github.com/clap-rs/clap/issues/1140
    matches
        .value_of("REMOTE_NAME")
        .unwrap_or(&default_remote_name)
        .to_string()
}

/// Get the Clap app for CLI matching et al.
fn build_cli<'a>() -> App<'a, 'a> {
    App::from_yaml(&APP_CFG)
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .setting(AppSettings::ArgsNegateSubcommands)
        .usage("git req [OPTIONS] [REQUEST_ID]")
}

/// Do the thing
fn main() {
    color_backtrace::install();
    let _ = env_logger::Builder::new()
        .parse_filters(&env::var("REQ_LOG").unwrap_or_default())
        .try_init();

    let mut app = build_cli();

    let matches = app.get_matches();

    if let Some(project_id) = matches.value_of("NEW_PROJECT_ID") {
        set_project_id(&get_remote_name(&matches), project_id);
    } else if matches.is_present("CLEAR_PROJECT_ID") {
        clear_project_id(&get_remote_name(&matches));
    } else if matches.is_present("LIST_MR") {
        list_open_requests(&get_remote_name(&matches));
    } else if matches.is_present("CLEAR_DOMAIN_KEY") {
        clear_domain_key(&get_remote_name(&matches));
    } else if let Some(domain_key) = matches.value_of("NEW_DOMAIN_KEY") {
        set_domain_key(&get_remote_name(&matches), domain_key);
    } else if let Some(remote_name) = matches.value_of("NEW_DEFAULT_REMOTE") {
        set_default_remote(remote_name);
    } else if let Some(shell_name) = matches.value_of("GENERATE_COMPLETIONS") {
        app = build_cli();
        generate_completion(&mut app, &shell_name);
    } else {
        match matches.value_of("REQUEST_ID").unwrap().parse() {
            Ok(mr_id) => {
                checkout_mr(&get_remote_name(&matches), mr_id);
            }
            Err(_) => {
                eprintln!("{}", "Invalid request ID provided".red());
                process::exit(1);
            }
        };
    }
}
