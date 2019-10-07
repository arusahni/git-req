use crate::git;
use crate::remotes::{MergeRequest, Remote};
use log::{debug, error, trace};
use regex::Regex;
use reqwest;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug)]
pub struct GitLab {
    pub id: String,
    pub domain: String,
    pub name: String,
    pub namespace: String,
    pub origin: String,
    pub api_root: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GitLabMergeRequest {
    id: i64,
    iid: i64,
    title: String,
    description: Option<String>,
    target_branch: String,
    source_branch: String,
    sha: String,
    web_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GitLabProject {
    id: i64,
    description: Option<String>,
    name: String,
    path: String,
    path_with_namespace: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GitLabNamespace {
    id: i64,
    name: String,
    path: String,
    kind: String,
    full_path: String,
}

impl Remote for GitLab {
    fn get_domain(&mut self) -> &str {
        &self.domain
    }

    fn get_project_id(&mut self) -> Result<&str, &str> {
        if self.id.is_empty() {
            self.id = format!("{}", query_gitlab_project_id(self)?);
        }
        Ok(&self.id)
    }

    fn get_local_req_branch(&mut self, mr_id: i64) -> Result<String, &str> {
        self.get_remote_req_branch(mr_id)
    }

    fn get_remote_req_branch(&mut self, mr_id: i64) -> Result<String, &str> {
        query_gitlab_branch_name(self, mr_id)
    }

    fn get_req_names(&mut self) -> Result<Vec<MergeRequest>, &str> {
        retrieve_gitlab_project_merge_requests(self)
    }

    fn has_useful_branch_names(&mut self) -> bool {
        true
    }
}

/// Query the GitLab API
fn query_gitlab_api(url: reqwest::Url, token: String) -> reqwest::Response {
    let client = reqwest::Client::new();
    client
        .get(url)
        .header("PRIVATE-TOKEN", token)
        .send()
        .expect("failed to send request")
}

/// Query the GitLab API for remote's project
fn query_gitlab_project_id(remote: &GitLab) -> Result<i64, &'static str> {
    trace!("Querying GitLab Project API for {:?}", remote);
    let url = reqwest::Url::parse(&format!(
        "{}/projects/{}%2F{}",
        remote.api_root, remote.namespace, remote.name
    ))
    .unwrap();
    let mut resp = query_gitlab_api(url, remote.api_key.to_string());
    debug!("Project ID query response: {:?}", resp);
    if !resp.status().is_success() {
        match search_gitlab_project_id(remote) {
            Ok(id) => {
                return Ok(id);
            }
            Err(_) => {
                return Err(
                    "Unable to get the project ID from the GitLab API.\nFind and configure \
                     your project ID using the instructions at: \
                     https://github.com/arusahni/git-req/wiki/Finding-Project-IDs",
                );
            }
        }
    }
    let buf: GitLabProject = resp.json().expect("failed to read response");
    debug!("{:?}", buf);
    Ok(buf.id)
}

/// Convert a GitLab MR to a git-req MR
fn gitlab_to_mr(req: GitLabMergeRequest) -> MergeRequest {
    MergeRequest {
        id: req.iid,
        title: req.title,
        description: req.description,
        source_branch: req.source_branch,
    }
}

/// Get the list of merge requests for the current project
fn retrieve_gitlab_project_merge_requests(
    remote: &GitLab,
) -> Result<Vec<MergeRequest>, &'static str> {
    trace!("Querying GitLab MR for {:?}", remote);
    let url = reqwest::Url::parse(&format!(
        "{}/projects/{}/merge_requests?state=opened",
        remote.api_root, remote.id
    ))
    .unwrap();
    let mut resp = query_gitlab_api(url, remote.api_key.to_string());
    debug!("MR list query response: {:?}", resp);
    let buf: Vec<GitLabMergeRequest> = match resp.json() {
        Ok(buf) => buf,
        Err(_) => {
            return Err("failed to read response");
        }
    };
    Ok(buf.into_iter().map(gitlab_to_mr).collect())
}

/// Search GitLab for the project ID (if the direct lookup didn't work)
fn search_gitlab_project_id(remote: &GitLab) -> Result<i64, &'static str> {
    trace!(
        "Searching GitLab API for namespace {:?} by project name",
        remote.namespace
    );
    let url = reqwest::Url::parse(&format!(
        "{}/namespaces/{}",
        remote.api_root, remote.namespace
    ))
    .unwrap();
    let mut resp = query_gitlab_api(url, remote.api_key.to_string());
    debug!("Namespace ID query response: {:?}", resp);
    if !resp.status().is_success() {
        return Err("Couldn't find namespace");
    }
    let ns_buf: GitLabNamespace = resp.json().expect("failed to read response");
    debug!("Querying namespace {:?}", ns_buf);
    let url = match ns_buf.kind.as_ref() {
        "user" => reqwest::Url::parse(&format!("{}/users/{}/projects", remote.api_root, ns_buf.id))
            .unwrap(),
        "group" => reqwest::Url::parse(&format!(
            "{}/groups/{}/projects?search={}",
            remote.api_root, ns_buf.id, remote.name
        ))
        .unwrap(),
        _ => {
            error!("Unknown namespace kind {:?}", ns_buf.kind);
            return Err("Unknown namespace");
        }
    };
    let mut resp = query_gitlab_api(url, remote.api_key.to_string());
    debug!("Project ID query response: {:?}", resp);
    let projects: Vec<GitLabProject> = resp.json().expect("failed to read projects response");
    match projects.iter().find(|&prj| prj.name == remote.name) {
        Some(project) => Ok(project.id),
        None => Err("Couldn't find project"),
    }
}

/// Get the project ID from config
pub fn load_project_id() -> Option<String> {
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

/// Extract the project name from a GitLab origin URL
pub fn get_gitlab_project_name(origin: &str) -> Option<String> {
    trace!("Getting project name for: {}", origin);
    let project_regex = Regex::new(r".*/(\S+?)(\.git)?$").unwrap();
    let captures = project_regex.captures(origin)?;
    Some(String::from(&captures[1]))
}

/// Extract the project namespace from a GitLab origin URL
pub fn get_gitlab_project_namespace(origin: &str) -> Option<String> {
    trace!("Getting project namespace for: {}", origin);
    let project_regex = Regex::new(r".*[/:](\S+)/\S+(\.git)?$").unwrap();
    match project_regex.captures(origin) {
        Some(captures) => Some(String::from(&captures[1])),
        None => None,
    }
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
    fn test_get_gitlab_project_namespace_http_no_git() {
        let ns = get_gitlab_project_namespace("https://gitlab.com/my_namespace/my_project");
        assert!(ns.is_some());
        assert_eq!("my_namespace", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_namespace_ssh() {
        let ns = get_gitlab_project_namespace("git@gitlab.com:my_namespace/my_project.git");
        assert!(ns.is_some());
        assert_eq!("my_namespace", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_namespace_ssh_no_git() {
        let ns = get_gitlab_project_namespace("git@gitlab.com:my_namespace/my_project");
        assert!(ns.is_some());
        assert_eq!("my_namespace", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_name_http() {
        let ns = get_gitlab_project_name("https://gitlab.com/my_namespace/my_project.git");
        assert!(ns.is_some());
        assert_eq!("my_project", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_name_http_no_git() {
        let ns = get_gitlab_project_name("https://gitlab.com/my_namespace/my_project");
        assert!(ns.is_some());
        assert_eq!("my_project", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_name_ssh() {
        let ns = get_gitlab_project_name("git@gitlab.com:my_namespace/my_project.git");
        assert!(ns.is_some());
        assert_eq!("my_project", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_name_ssh_no_git() {
        let ns = get_gitlab_project_name("git@gitlab.com:my_namespace/my_project");
        assert!(ns.is_some());
        assert_eq!("my_project", ns.unwrap());
    }
}
