use anyhow::{anyhow, Result};
use logchop::OptionLogger;
use std::path::Path;
use std::str;
use std::{collections::HashSet, convert::TryInto};

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
        Ok(remotes) => remotes.into_iter().flatten().map(String::from).collect(),
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
    let cfg = Config::open(Path::new(
        &shellexpand::tilde("~/.gitreqconfig").to_string(),
    ))
    .unwrap();
    cfg.get_string(&format!("req.{}.{}", slug, field)).ok()
}

/// Set a value for the global git-req configuration
pub fn set_req_config(domain: &str, field: &str, value: &str) {
    let slug = slugify_domain(domain);
    let mut cfg = Config::open(Path::new(
        &shellexpand::tilde("~/.gitreqconfig").to_string(),
    ))
    .unwrap();
    cfg.set_str(&format!("req.{}.{}", slug, field), value)
        .unwrap();
}

/// Clear the value for the lobal git-req configuration
pub fn delete_req_config(domain: &str, field: &str) -> Result<(), git2::Error> {
    let slug = slugify_domain(domain);
    let mut cfg = Config::open(Path::new(
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

/// Get the ID of the previous MR that had been checked out using git-req
pub fn get_previous_mr_id() -> Result<i64> {
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let testco = repo.find_reference("git-req/previous")?;
    let content = testco.peel_to_blob()?;
    let binary = content.content();
    let reqnum = i64::from_le_bytes(binary.try_into()?);
    debug!("Loaded previous reference MR number: {}", reqnum);
    Ok(reqnum)
}

/// Push a new `current` history ref, moving the existing `current` to `previous`
pub fn push_current_ref(new_req_number: i64) -> Result<i64> {
    trace!("Storing refs for MR {}", new_req_number);
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let old_oid = match repo.find_reference("git-req/current") {
        Ok(current_ref) => current_ref
            .target()
            .debug_none("Could not peel current ref"),
        Err(_) => None,
    };
    let data = new_req_number.to_le_bytes();
    let new_oid = repo.blob(&data).unwrap();
    if let Some(oid) = old_oid {
        repo.reference("git-req/previous", oid, true, "").unwrap();
        debug!("Wrote old OID '{}' to git_req/previous", oid);
    };
    repo.reference("git-req/current", new_oid, true, "")?;
    Ok(new_req_number)
}

#[derive(Debug)]
pub enum CheckoutResult {
    BranchChanged,
    BranchUnchanged,
}

/// Check out a branch by name
pub fn checkout_branch(
    remote_name: &str,
    remote_branch_name: &str,
    local_branch_name: &str,
    is_virtual_remote_branch: bool,
) -> Result<CheckoutResult> {
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
            let head = repo.head()?;
            trace!("On head: {:?}", head.name());
            if head.is_branch()
                && head.name().unwrap() == format!("refs/heads/{}", &local_branch_name)
            {
                // return Err(anyhow!("Already on {}", &local_branch_name));
                return Ok(CheckoutResult::BranchUnchanged);
            }
            match cmd!("git", "checkout", &local_branch_name).run() {
                Ok(_) => Ok(CheckoutResult::BranchChanged),
                Err(err) => Err(anyhow!("Could not check out local branch: {}", err)),
            }
        }
        Err(_) => {
            // Fetch the remote branch if there's no local branch with the correct name
            let mut fetch_args = vec!["fetch", remote_name];
            let remote_to_local_binding = format!("{}:{}", remote_branch_name, local_branch_name);
            fetch_args.push(if is_virtual_remote_branch {
                &remote_to_local_binding
            } else {
                remote_branch_name
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
                Ok(_) => Ok(CheckoutResult::BranchChanged),
                Err(err) => Err(anyhow!("Could not check out local branch: {}", err)),
            }
        }
    }
}
