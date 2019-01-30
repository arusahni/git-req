use crate::git;
use log::{debug, info, trace};
use regex::Regex;
use reqwest;
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::io::{stdin, stdout, Write};

pub trait Remote {
    /// Get the ID of the project associated with the repository
    fn get_project_id(&mut self) -> Result<&str, &str>;

    /// Get the branch associated with the merge request having the given ID
    fn get_req_branch(&mut self, mr_id: i64) -> Result<String, &str>;
}

/// Print a pretty remote
impl fmt::Display for Remote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Remote")
    }
}

/// Debug a remote
impl fmt::Debug for Remote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
struct GitHub {
    id: String,
    name: String,
    origin: String,
    api_root: String,
}

impl Remote for GitHub {
    fn get_project_id(&mut self) -> Result<&str, &str> {
        Ok(&self.id)
    }

    fn get_req_branch(&mut self, mr_id: i64) -> Result<String, &str> {
        Ok(format!("pr/{}", mr_id))
    }
}

#[derive(Debug)]
struct GitLab {
    id: String,
    domain: String,
    name: String,
    namespace: String,
    origin: String,
    api_root: String,
    api_key: String,
}

impl Remote for GitLab {
    fn get_project_id(&mut self) -> Result<&str, &str> {
        if self.id.is_empty() {
            self.id = format!("{}", query_gitlab_project_id(self)?);
        }
        Ok(&self.id)
    }

    fn get_req_branch(&mut self, mr_id: i64) -> Result<String, &str> {
        query_gitlab_branch_name(self, mr_id)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct GitLabProject {
    id: i64,
    description: String,
    name: String,
    path: String,
    path_with_namespace: String,
}

/// Query the GitLab API for remote's project
fn query_gitlab_project_id(remote: &GitLab) -> Result<i64, &'static str> {
    let client = reqwest::Client::new();
    let url = reqwest::Url::parse(&format!(
        "{}/projects/{}%2F{}",
        remote.api_root, remote.namespace, remote.name
    ))
    .unwrap();
    let mut resp = client
        .get(url)
        .header("PRIVATE-TOKEN", remote.api_key.to_string())
        .send()
        .expect("failed to send request");
    debug!("Response: {:?}", resp);
    if !resp.status().is_success() {
        return Err("bad server response");
    }
    let buf: GitLabProject = resp.json().expect("failed to read response");
    debug!("{:?}", buf);
    Ok(buf.id)
}

#[derive(Serialize, Deserialize, Debug)]
struct GitLabMergeRequest {
    id: i64,
    iid: i64,
    title: String,
    target_branch: String,
    source_branch: String,
    sha: String,
    web_url: String,
}

/// Get the project ID from config
fn load_project_id() -> Option<String> {
    match git::get_config("projectid") {
        Some(project_id) => Some(project_id),
        None => {
            debug!("No project ID found");
            None
        }
    }
}

/// Query the GitLab API for the branch corresponding to the MR
fn query_gitlab_branch_name(remote: &GitLab, mr_id: i64) -> Result<String, &str> {
    let client = reqwest::Client::new();
    let url = reqwest::Url::parse(&format!(
        "{}/projects/{}/merge_requests/{}",
        remote.api_root, remote.id, mr_id
    ))
    .unwrap();
    let mut resp = client
        .get(url)
        .header("PRIVATE-TOKEN", remote.api_key.to_string())
        .send()
        .expect("failed to send request");
    debug!("Response: {:?}", resp);
    let buf: GitLabMergeRequest = match resp.json() {
        Ok(buf) => buf,
        Err(_) => {
            return Err("failed to read response");
        }
    };
    Ok(buf.source_branch)
}

/// Extract the project name from a Github origin URL
fn get_github_project_name(origin: &str) -> String {
    trace!("Getting project name for: {}", origin);
    let project_regex = Regex::new(r".*:(.*/\S+)\.git\w*$").unwrap();
    let captures = project_regex.captures(origin).unwrap();
    String::from(&captures[1])
}

/// Extract the project name from a GitLab origin URL
fn get_gitlab_project_name(origin: &str) -> String {
    trace!("Getting project name for: {}", origin);
    let project_regex = Regex::new(r".*/(\S+)\.git$").unwrap();
    let captures = project_regex.captures(origin).unwrap();
    String::from(&captures[1])
}

/// Extract the project namespace from a GitLab origin URL
fn get_gitlab_project_namespace(origin: &str) -> Option<String> {
    trace!("Getting project namespace for: {}", origin);
    let project_regex = Regex::new(r".*[/:](\S+)/\S+\.git$").unwrap();
    match project_regex.captures(origin) {
        Some(captures) => Some(String::from(&captures[1])),
        None => None
    }
}

/// Get the domain from an origin URL
pub fn get_domain(origin: &str) -> Result<&str, String> {
    let domain_regex = Regex::new(r"((http[s]?|ssh)://)?(\S+@)?(?P<domain>([^:/])+)").unwrap();
    let captures = domain_regex.captures(origin);
    if captures.is_none() {
        return Err(String::from("invalid remote set"));
    }
    Ok(captures.unwrap().name("domain").map_or("", |x| x.as_str()))
}

/// Get a remote struct from an origin URL
pub fn get_remote(origin: &str) -> Result<Box<Remote>, String> {
    let domain = get_domain(origin)?;
    Ok(match domain {
        "github.com" => Box::new(GitHub {
            id: get_github_project_name(origin),
            name: get_github_project_name(origin),
            origin: String::from(origin),
            api_root: String::from("https://api.github.com/repos"),
        }),
        // For now, if not GitHub, then GitLab
        gitlab_domain => {
            let namespace = match get_gitlab_project_namespace(origin) {
                Some(ns) => ns,
                None => {
                    return Err(String::from("Could not parse the GitLab project namespace from the origin."));
                }
            };
            let mut remote = GitLab {
                id: String::from(""),
                domain: String::from(gitlab_domain),
                name: get_gitlab_project_name(origin),
                namespace: namespace,
                origin: String::from(origin),
                api_root: format!("https://{}/api/v4", gitlab_domain),
                api_key: String::from(""),
            };
            let apikey = match git::get_req_config(&domain, "apikey") {
                Some(key) => key,
                None => {
                    let mut newkey = String::new();
                    print!("Please enter the read-only API key for {}: ", gitlab_domain);
                    let _ = stdout().flush();
                    stdin()
                        .read_line(&mut newkey)
                        .expect("Did not input a correct key");
                    trace!("New Key: {}", &newkey);
                    git::set_req_config(&domain, "apikey", &newkey.trim());
                    String::from(newkey.trim())
                }
            };
            info!("API Key: {}", &apikey);
            remote.api_key = apikey;
            let project_id = match load_project_id() {
                Some(x) => x,
                None => {
                    let project_id_str = match remote.get_project_id() {
                        Ok(id_str) => Ok(id_str),
                        Err(e) => {
                            info!("Error getting project ID: {:?}", e);
                            Err(e)
                        }
                    }?;
                    git::set_config("projectid", project_id_str);
                    String::from(project_id_str)
                }
            };
            info!("Got project ID: {}", project_id);
            remote.id = project_id;
            Box::new(remote)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_gitlab_project_namespace_http() {
        let ns = get_gitlab_project_namespace("https://gitlab.com/my_namespace/my_project.git");
        assert!(ns.is_some());
        assert_eq!("my_namespace", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_namespace_git() {
        let ns = get_gitlab_project_namespace("git@gitlab.com:my_namespace/my_project.git");
        assert!(ns.is_some());
        assert_eq!("my_namespace", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_name_http() {
        let ns = get_gitlab_project_name("https://gitlab.com/my_namespace/my_project.git");
        assert_eq!("my_project", ns);
    }

    #[test]
    fn test_get_gitlab_project_name_git() {
        let ns = get_gitlab_project_name("git@gitlab.com:my_namespace/my_project.git");
        assert_eq!("my_project", ns);
    }
}
