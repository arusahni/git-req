use std::str;
use std::path::Path;

use duct::cmd;
use log::debug;
use git2::{Config, Repository};
use shellexpand;

fn slugify_domain(domain: &str) -> String {
    str::replace(domain, ".", "|")
}

pub fn get_repo_info(repo_field: &str) -> String {
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let cfg = repo.config().unwrap();
    cfg.get_string(repo_field).unwrap()
}

pub fn get_config(field_name: &str) -> Option<String> {
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let cfg = repo.config().unwrap();
    match cfg.get_string(&format!("req.{}", field_name)) {
        Ok(val) => Some(val),
        Err(_) => None
    }
}

pub fn set_config(field_name: &str, value: &str) {
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    let mut cfg = repo.config().unwrap();
    cfg.set_str(&format!("req.{}", field_name), value).unwrap();
}

pub fn get_req_config(domain: &str, field: &str) -> Option<String> {
    let slug = slugify_domain(domain);
    let cfg = Config::open(&Path::new(&shellexpand::tilde("~/.gitreqconfig").to_string())).unwrap();
    match cfg.get_string(&format!("req.{}.{}", slug, field)) {
        Ok(val) => Some(val),
        Err(_) => None
    }
}

/// Set a value for the global git-req configuration
pub fn set_req_config(domain: &str, field: &str, value: &str) {
    let slug = slugify_domain(domain);
    let mut cfg = Config::open(&Path::new(&shellexpand::tilde("~/.gitreqconfig").to_string())).unwrap();
    cfg.set_str(&format!("req.{}.{}", slug, field), value).unwrap();
}

pub fn checkout_branch(branch_name: &str) -> Result<bool, String> {
    let repo = Repository::open_from_env().expect("Couldn't find repository");
    // let full_branch_name = format!("refs/heads/{}", branch_name);
    if repo.revparse_single(branch_name).is_err() {
        cmd!("git", "fetch").run().unwrap();
        if repo.revparse_single(&format!("origin/{}", branch_name)).is_err() {
            return Err(format!("Could not find remote branch: {}", branch_name));
        }
    }
    debug!("Checking out branch!");
    cmd!("git", "checkout", branch_name).run().unwrap();
    Ok(true)
}
