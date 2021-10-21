use crate::git;
use crate::remotes::{MergeRequest, Remote};
use anyhow::{anyhow, Result};
use git_url_parse::GitUrl;
use log::{debug, error, trace};
use logchop::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug)]
pub struct GitLab {
    pub id: String,
    pub domain: String,
    pub name: String,
    pub namespace: String,
    pub full_path: String,
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

    fn get_project_id(&mut self) -> Result<&str> {
        if self.id.is_empty() {
            self.id = format!("{}", query_gitlab_project_id(self)?);
        }
        Ok(&self.id)
    }

    fn get_local_req_branch(&mut self, mr_id: i64) -> Result<String> {
        self.get_remote_req_branch(mr_id)
    }

    fn get_remote_req_branch(&mut self, mr_id: i64) -> Result<String> {
        query_gitlab_branch_name(self, mr_id)
    }

    fn get_req_names(&mut self) -> Result<Vec<MergeRequest>> {
        retrieve_gitlab_project_merge_requests(self)
    }

    fn has_useful_branch_names(&mut self) -> bool {
        true
    }

    fn has_virtual_remote_branch_names(&mut self) -> bool {
        false
    }
}

/// Query the GitLab API
fn query_gitlab_api(url: &str, token: &str) -> Result<ureq::Response, ureq::Response> {
    let response = ureq::get(url).set("PRIVATE-TOKEN", token).call();
    if response.error() {
        return Err(response);
    }
    Ok(response)
}

/// Query the GitLab API for remote's project
fn query_gitlab_project_id(remote: &GitLab) -> Result<i64> {
    trace!("Querying GitLab Project API for {:?}", remote);
    // First, attempt a lookup by the full project path
    let url = &format!(
        "{}/projects/{}",
        remote.api_root,
        remote.full_path.replace("/", "%2F")
    );
    trace!("Attempting direct project ID lookup: {}", url);
    let resp = query_gitlab_api(url, &remote.api_key);
    // If not found, attempt to search for it
    if resp.is_err() {
        trace!("Direct lookup unsuccessful. Attempting search strategy.");
        match search_gitlab_project_id(remote) {
            Ok(id) => {
                return Ok(id);
            }
            Err(_) => {
                return Err(anyhow!(
                    "Unable to get the project ID from the GitLab API.\nFind and configure \
                     your project ID using the instructions at: \
                     https://github.com/arusahni/git-req/wiki/Finding-Project-IDs",
                ));
            }
        }
    }
    trace!("Direct lookup successful.");
    let buf: GitLabProject = match resp.unwrap().into_json() {
        // "safe" unwrap since we test is_err above
        Ok(buf) => serde_json::from_value(buf).expect("failed to decode response"),
        Err(_) => {
            return Err(anyhow!("failed to read response"));
        }
    };
    debug!("Direct lookup response: {:?}", buf);
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
fn retrieve_gitlab_project_merge_requests(remote: &GitLab) -> Result<Vec<MergeRequest>> {
    trace!("Querying GitLab MR for {:?}", remote);
    let current_page = 1;
    let url = &format!(
        "{}/projects/{}/merge_requests?state=opened&per_page=50&page={}",
        remote.api_root, remote.id, current_page,
    );
    let resp = query_gitlab_api(url, &remote.api_key);
    debug!("MR list query response: {:?}", resp);
    let merge_requests: Vec<GitLabMergeRequest> = match resp {
        Ok(response) => {
            let buf = response.into_json().expect("malformed API response");
            serde_json::from_value(buf).expect("failed to decode response")
        }
        Err(response) => {
            debug!("Failed MR list query response: {:?}", response);
            if response.status() == 404 {
                return Err(anyhow!("remote project not found"));
            }
            return Err(anyhow!("failed to read response"));
        }
    };
    Ok(merge_requests.into_iter().map(gitlab_to_mr).collect())
}

/// Search GitLab for the project ID (if the direct lookup didn't work)
fn search_gitlab_project_id(remote: &GitLab) -> Result<i64> {
    trace!(
        "Searching GitLab API for namespace {:?} by project name",
        remote.namespace
    );
    let url = &format!("{}/namespaces/{}", remote.api_root, remote.namespace);
    let resp = query_gitlab_api(url, &remote.api_key);
    debug!("Namespace ID query response: {:?}", resp);
    let ns_buf: GitLabNamespace = match resp {
        Ok(response) => match response.into_json() {
            Ok(buf) => serde_json::from_value(buf).expect("failed to decode response"),
            Err(_) => {
                return Err(anyhow!("malformed response received"));
            }
        },
        Err(response) => {
            if response.status() == 404 {
                return Err(anyhow!("couldn't find namespace"));
            }
            return Err(anyhow!("failed to read response"));
        }
    };
    debug!("Querying namespace {:?}", ns_buf);
    let url = match ns_buf.kind.as_ref() {
        "user" => format!("{}/users/{}/projects", remote.api_root, ns_buf.id),
        "group" => format!(
            "{}/groups/{}/projects?search={}",
            remote.api_root, ns_buf.id, remote.name
        ),
        _ => {
            error!("Unknown namespace kind {:?}", ns_buf.kind);
            return Err(anyhow!("Unknown namespace"));
        }
    };
    let resp = query_gitlab_api(&url, &remote.api_key);
    debug!("Project ID query response: {:?}", resp);
    let projects: Vec<GitLabProject> = match resp {
        Ok(response) => match response.into_json() {
            Ok(buf) => serde_json::from_value(buf).expect("failed to decode projects response"),
            Err(_) => return Err(anyhow!("malformed projects response")),
        },
        Err(_) => {
            return Err(anyhow!("failed to read projects response"));
        }
    };
    match projects.iter().find(|&prj| prj.name == remote.name) {
        Some(project) => Ok(project.id),
        None => Err(anyhow!("Couldn't find project")),
    }
}

/// Get the project ID for the specified remote from config
pub fn load_project_id(remote_name: &str) -> Option<String> {
    git::get_config("projectid", remote_name).debug_none("No project ID found")
}

/// Query the GitLab API for the branch corresponding to the MR
fn query_gitlab_branch_name(remote: &GitLab, mr_id: i64) -> Result<String> {
    let url = &format!(
        "{}/projects/{}/merge_requests/{}",
        remote.api_root, remote.id, mr_id
    );
    let resp = ureq::get(url).set("PRIVATE-TOKEN", &remote.api_key).call();
    debug!("Response: {:?}", resp);
    let buf: GitLabMergeRequest = match resp.into_json() {
        Ok(buf) => serde_json::from_value(buf).expect("failed to decode response"),
        Err(_) => {
            return Err(anyhow!("failed to read response"));
        }
    };
    Ok(buf.source_branch)
}

/// Extract the project name from a GitLab origin URL
pub fn get_gitlab_project_name(origin: &str) -> Option<String> {
    trace!("Getting project name for: {}", origin);
    GitUrl::parse(origin).map(|parsed| parsed.name).ok()
}

/// Extract the project namespace from a GitLab origin URL
pub fn get_gitlab_project_namespace(origin: &str) -> Option<String> {
    trace!("Getting project namespace for: {}", origin);
    GitUrl::parse(origin)
        .map(|parsed| {
            parsed
                .path
                .split("/".chars().next().unwrap())
                .find(|s| !s.is_empty())
                .unwrap()
                .to_owned()
        })
        .ok()
}

pub fn get_gitlab_project_full_path(origin: &str) -> Option<String> {
    trace!("Getting full path for: {}", origin);
    GitUrl::parse(origin)
        .map(|parsed| {
            parsed
                .path
                .trim_start_matches("/".chars().next().unwrap())
                .trim_end_matches(".git")
                .to_owned()
        })
        .ok()
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
    fn test_get_gitlab_project_namespace_nested() {
        let ns = get_gitlab_project_namespace("git@gitlab.com:my_namespace/my_org/my_project.git");
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

    #[test]
    fn test_get_gitlab_project_name_ssh_nested() {
        let ns = get_gitlab_project_name("git@gitlab.com:my_namespace/my_org/my_project.git");
        assert!(ns.is_some());
        assert_eq!("my_project", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_full_path_https() {
        let ns = get_gitlab_project_full_path("https://gitlab.com/my_namespace/my_project");
        assert!(ns.is_some());
        assert_eq!("my_namespace/my_project", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_full_path_https_nested() {
        let ns = get_gitlab_project_full_path("https://gitlab.com/my_namespace/my_org/my_project");
        assert!(ns.is_some());
        assert_eq!("my_namespace/my_org/my_project", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_full_path_ssh() {
        let ns = get_gitlab_project_full_path("git@gitlab.com:my_namespace/my_project.git");
        assert!(ns.is_some());
        assert_eq!("my_namespace/my_project", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_full_path_ssh_nested() {
        let ns = get_gitlab_project_full_path("git@gitlab.com:my_namespace/my_org/my_project.git");
        assert!(ns.is_some());
        assert_eq!("my_namespace/my_org/my_project", ns.unwrap());
    }

    #[test]
    fn test_get_gitlab_project_full_path_ssh_no_git_nested() {
        let ns = get_gitlab_project_full_path("git@gitlab.com:my_namespace/my_org/my_project");
        assert!(ns.is_some());
        assert_eq!("my_namespace/my_org/my_project", ns.unwrap());
    }
}
