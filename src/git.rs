use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::path::Path;
use std::str;

use duct::cmd;
use git2::{Config, Repository};
use log::{debug, trace, warn};

/// Update old `req.key` config format to include remote name, i.e, `req.remote_name.key`
fn migrate_legacy(field_name: &str, remote_name: &str) {
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let mut cfg = repo.config().unwrap();

    if let Ok(ref value) = cfg.get_string(&format!("req.{}", field_name)) {
        cfg.set_str(&format!("req.{}.{}", remote_name, field_name), value)
            .unwrap();
        cfg.remove(&format!("req.{}", field_name)).unwrap();
    }
}

/// Convert a domain string into a configuration slug
fn slugify_domain(domain: &str) -> String {
    str::replace(domain, ".", "|")
}

/// Get the remotes for the repository
pub fn get_remotes() -> HashSet<String> {
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    match repo.remotes() {
        Ok(remotes) => remotes
            .into_iter()
            .filter_map(|rem| rem)
            .map(String::from)
            .collect(),
        Err(_) => HashSet::new(),
    }
}

/// Get the URL of the given remote
pub fn get_remote_url(remote: &str) -> String {
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let remote = repo.find_remote(remote).expect("Couldn't find the remote");
    String::from(remote.url().unwrap())
}

/// Get a value fom the repository config
pub fn get_repo_info(repo_field: &str) -> Result<String> {
    let repo = Repository::open_from_env().map_err(|_| anyhow!("Couldn't find repository"))?;
    let cfg = repo.config().unwrap();
    cfg.get_string(repo_field)
        .map_err(|err| anyhow!(err.to_string()))
}

/// Get a value for the given project-local git-req config
pub fn get_config(field_name: &str, remote_name: &str) -> Option<String> {
    migrate_legacy(field_name, "origin");
    let key = format!("req.{}.{}", remote_name, field_name);
    get_repo_info(&key).ok()
}

/// Set a value for the project remote-local git-req configuration
pub fn set_config(field_name: &str, remote_name: &str, value: &str) {
    migrate_legacy(field_name, "origin");
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let mut cfg = repo.config().unwrap();
    cfg.set_str(&format!("req.{}.{}", remote_name, field_name), value)
        .unwrap();
}

/// Delete the entry for the project-local git-req config field with the provided name
pub fn delete_config(field_name: &str, remote_name: &str) {
    migrate_legacy(field_name, "origin");
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let mut cfg = repo.config().unwrap();
    cfg.remove(&format!("req.{}.{}", remote_name, field_name))
        .unwrap();
}

/// Get a value for the given project-local git-req config
pub fn get_project_config(field_name: &str) -> Option<String> {
    let key = format!("req.{}", field_name);
    get_repo_info(&key).ok()
}

/// Set a value for the project-local git-req configuration. Consider using `set_config` unless
/// absolutely necessary.
pub fn set_project_config(field_name: &str, value: &str) {
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let mut cfg = repo.config().unwrap();
    cfg.set_str(&format!("req.{}", field_name), value).unwrap();
}

/// Get a value for the given global git-req config
pub fn get_req_config(domain: &str, field: &str) -> Option<String> {
    let slug = slugify_domain(domain);
    let cfg = Config::open(&Path::new(
        &shellexpand::tilde("~/.gitreqconfig").to_string(),
    ))
    .unwrap();
    cfg.get_string(&format!("req.{}.{}", slug, field)).ok()
}

/// Set a value for the global git-req configuration
pub fn set_req_config(domain: &str, field: &str, value: &str) {
    let slug = slugify_domain(domain);
    let mut cfg = Config::open(&Path::new(
        &shellexpand::tilde("~/.gitreqconfig").to_string(),
    ))
    .unwrap();
    cfg.set_str(&format!("req.{}.{}", slug, field), value)
        .unwrap();
}

/// Clear the value for the lobal git-req configuration
pub fn delete_req_config(domain: &str, field: &str) -> Result<(), git2::Error> {
    let slug = slugify_domain(domain);
    let mut cfg = Config::open(&Path::new(
        &shellexpand::tilde("~/.gitreqconfig").to_string(),
    ))
    .unwrap();
    cfg.remove(&format!("req.{}.{}", slug, field))
}

/// Guess the name of the default remote
pub fn guess_default_remote_name() -> Result<String> {
    let repo = Repository::open_from_env().map_err(|_| anyhow!("Couldn't find repository"))?;
    let remotes = repo
        .remotes()
        .map_err(|_| anyhow!("Couldn't fetch the list of remotes"))?;
    match remotes.len() {
        0 => Err(anyhow!("Could not find any remotes")),
        1 => Ok(String::from(remotes.get(0).unwrap())),
        _ => match repo.find_remote("origin") {
            Ok(_) => Ok(String::from("origin")),
            Err(_) => Err(anyhow!("No origin remote found")),
        },
    }
}

/// Check out a branch by name
pub fn checkout_branch(
    remote_name: &str,
    remote_branch_name: &str,
    local_branch_name: &str,
    is_virtual_remote_branch: bool,
) -> Result<()> {
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let local_branch_name = match get_project_config("defaultremote") {
        Some(default_remote_name) => {
            if remote_name != default_remote_name {
                trace!("Non-default remote name requested: {}", remote_name);
                format!("req/{}/{}", remote_name, local_branch_name)
            } else {
                trace!("Default remote name requested: {}", remote_name);
                String::from(local_branch_name)
            }
        }
        None => {
            warn!("No default remote found. Using {}", remote_name);
            format!("{}/{}", remote_name, local_branch_name)
        }
    };

    let local_branch_exists = repo.revparse_single(&local_branch_name);
    match local_branch_exists {
        Ok(_) => {
            debug!("Checking out branch: {}", local_branch_name);
            match cmd!("git", "checkout", &local_branch_name).run() {
                Ok(_) => Ok(()),
                Err(err) => Err(anyhow!("Could not check out local branch: {}", err)),
            }
        }
        Err(_) => {
            // Fetch the remote branch if there's no local branch with the correct name
            let mut fetch_args = vec!["fetch", &remote_name];
            let remote_to_local_binding = format!("{}:{}", remote_branch_name, local_branch_name);
            fetch_args.push(if is_virtual_remote_branch {
                &remote_to_local_binding
            } else {
                &remote_branch_name
            });
            if cmd("git", fetch_args).run().is_err() {
                return Err(anyhow!(
                    "Could not fetch remote branch '{}'",
                    remote_branch_name
                ));
            };
            debug!("Checking out branch: {}", local_branch_name);
            let mut checkout_args = vec!["checkout"];
            let origin_with_remote = format!("{}/{}", remote_name, remote_branch_name);
            if is_virtual_remote_branch {
                checkout_args.push(&local_branch_name);
                trace!("Checking out branch: {}", local_branch_name);
            } else {
                checkout_args.push("-b");
                checkout_args.push(&local_branch_name);
                checkout_args.push(&origin_with_remote);
                trace!(
                    "Checking '{}' as '{}'",
                    origin_with_remote,
                    local_branch_name
                );
            };
            match cmd("git", checkout_args).run() {
                Ok(_) => Ok(()),
                Err(err) => Err(anyhow!("Could not check out local branch: {}", err)),
            }
        }
    }
}
