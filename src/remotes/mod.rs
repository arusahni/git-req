use crate::git;
use anyhow::{anyhow, Result};
use log::{info, trace};
use logchop::*;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::io::{stdin, stdout, Write};

pub mod github;
pub mod gitlab;

#[derive(Serialize, Deserialize, Debug)]
pub struct MergeRequest {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub source_branch: String,
}

pub trait Remote {
    /// Get the ID of the project associated with the repository
    fn get_project_id(&mut self) -> Result<&str>;

    /// Get the local branch associated with the merge request having the given ID
    fn get_local_req_branch(&mut self, mr_id: i64) -> Result<String>;

    /// Get the remote branch associated with the merge request having the given ID
    fn get_remote_req_branch(&mut self, mr_id: i64) -> Result<String>;

    /// Get the names of the merge/pull requests opened against the remote
    fn get_req_names(&mut self) -> Result<Vec<MergeRequest>>;

    /// Determine if the branch names are useful to display
    fn has_useful_branch_names(&mut self) -> bool;

    /// If the remote branch is a namespaced ref instead of an actual branch
    // This is useful for GitHub's `pull/mr/head` refs, where they're read-only
    fn has_virtual_remote_branch_names(&mut self) -> bool;

    /// The domain that hosts the remote
    fn get_domain(&mut self) -> &str;
}

/// Print a pretty remote
impl fmt::Display for dyn Remote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Remote")
    }
}

/// Get the domain from an origin URL
pub fn get_domain(origin: &str) -> Result<&str> {
    let domain_regex = Regex::new(r"((http[s]?|ssh)://)?(\S+@)?(?P<domain>([^:/])+)").unwrap();
    match domain_regex.captures(origin) {
        Some(captures) => Ok(captures.name("domain").map_or("", |x| x.as_str())),
        None => Err(anyhow!("invalid remote set")),
    }
}

/// Get the API key for the given domain. If absent, prompt.
fn get_api_key(domain: &str) -> String {
    git::get_req_config(domain, "apikey").unwrap_or_else(|| {
        let mut newkey = String::new();
        println!("No API token for {} found. See https://github.com/arusahni/git-req/wiki/API-Keys for instructions.", domain);
        print!("{} API token: ", domain);
        let _ = stdout().flush();
        stdin()
            .read_line(&mut newkey)
            .expect("Did not input a correct key");
        trace!("New Key: {}", &newkey);
        git::set_req_config(domain, "apikey", newkey.trim());
        newkey.trim().to_string()
    })
}

/// Get a remote struct from an origin URL
pub fn get_remote(remote_name: &str, origin: &str, skip_api_key: bool) -> Result<Box<dyn Remote>> {
    let domain = get_domain(origin)?;
    Ok(match domain {
        "github.com" => {
            let name = github::get_github_project_name(origin).ok_or_else(|| {
                anyhow!("Could not parse the GitHub project name from the origin.")
            })?;
            let mut remote = github::GitHub {
                id: String::from(&name),
                domain: String::from("github.com"),
                name,
                origin: String::from(origin),
                api_root: String::from("https://api.github.com/repos"),
                api_key: String::from(""),
            };
            if !skip_api_key {
                let apikey = get_api_key("github.com");
                info!("API Key: {}", &apikey);
                remote.api_key = apikey;
            }
            Box::new(remote)
        }
        // For now, if not GitHub, then GitLab
        gitlab_domain => {
            let namespace = gitlab::get_gitlab_project_namespace(origin).ok_or_else(|| {
                anyhow!("Could not parse the GitLab project namespace from the origin.")
            })?;
            let name = gitlab::get_gitlab_project_name(origin)
                .debug_some("Project name")
                .ok_or_else(|| {
                    anyhow!("Could not parse the GitLab project name from the origin.")
                })?;
            let full_path = gitlab::get_gitlab_project_full_path(origin)
                .ok_or_else(|| anyhow!("Could not parse the GitLab path from the origin."))?;
            let mut remote = gitlab::GitLab {
                id: String::from(""),
                domain: String::from(gitlab_domain),
                name,
                namespace,
                full_path,
                origin: String::from(origin),
                api_root: format!("https://{}/api/v4", gitlab_domain),
                api_key: String::from(""),
            };
            if !skip_api_key {
                let apikey = get_api_key(domain);
                info!("API Key: {}", &apikey);
                remote.api_key = apikey;
            }
            let project_id = match gitlab::load_project_id(remote_name) {
                Some(x) => x,
                None => {
                    if skip_api_key {
                        String::from("")
                    } else {
                        let project_id_str = remote
                            .get_project_id()
                            .info_err("Error getting project ID")?;
                        git::set_config("projectid", remote_name, project_id_str);
                        String::from(project_id_str)
                    }
                }
            };
            info!("Got project ID: {}", project_id);
            remote.id = project_id;
            Box::new(remote)
        }
    })
}
