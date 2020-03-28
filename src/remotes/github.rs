use crate::remotes::{MergeRequest, Remote};
use log::{debug, trace};
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use serde_json;
use ureq;

#[derive(Debug)]
pub struct GitHub {
    pub id: String,
    pub domain: String,
    pub name: String,
    pub origin: String,
    pub api_root: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GitHubPullRequest {
    id: i64,
    number: i64,
    title: String,
    body: Option<String>,
    html_url: String,
}

impl Remote for GitHub {
    fn get_domain(&mut self) -> &str {
        &self.domain
    }

    fn get_project_id(&mut self) -> Result<&str, &str> {
        Ok(&self.id)
    }

    fn get_local_req_branch(&mut self, mr_id: i64) -> Result<String, &str> {
        Ok(format!("pr/{mr_id}", mr_id = mr_id))
    }

    fn get_remote_req_branch(&mut self, mr_id: i64) -> Result<String, &str> {
        Ok(format!("pull/{mr_id}/head", mr_id = mr_id))
    }

    fn get_req_names(&mut self) -> Result<Vec<MergeRequest>, &str> {
        retrieve_github_project_pull_requests(self)
    }

    fn has_useful_branch_names(&mut self) -> bool {
        false
    }

    fn has_virtual_remote_branch_names(&mut self) -> bool {
        true
    }
}

/// Convert a GitHub PR to a git-req MergeRequest
fn github_to_mr(req: GitHubPullRequest) -> MergeRequest {
    MergeRequest {
        id: req.number,
        title: req.title,
        description: req.body,
        source_branch: format!("pr/{}", req.number),
    }
}

/// Query the GitHub API
fn query_github_api(url: &str, token: &str) -> Result<ureq::Response, ureq::Response> {
    trace!("Querying {}", url);
    let response = ureq::get(url)
        .set("Authorization", &format!("token {}", token))
        .call();
    if response.error() {
        return Err(response);
    }
    Ok(response)
}

/// Get the pull requests for the current project
fn retrieve_github_project_pull_requests(
    remote: &GitHub,
) -> Result<Vec<MergeRequest>, &'static str> {
    trace!("Querying for GitHub PR for {:?}", remote);
    let url = &format!("{}/{}/pulls", remote.api_root, remote.id);
    let gprs: Vec<GitHubPullRequest> = match query_github_api(url, &remote.api_key) {
        Ok(response) => {
            debug!("Successful PR list query response: {:?}", response);
            let buf = response.into_json().expect("malformed API response");
            serde_json::from_value(buf).expect("failed to decode API response")
        }
        Err(response) => {
            debug!("Failed PR list query response: {:?}", response);
            if response.status() == 404 {
                return Err("remote project not found");
            }
            return Err("failed to read API response");
        }
    };
    Ok(gprs.into_iter().map(github_to_mr).collect())
}

/// Extract the project name from a Github origin URL
pub fn get_github_project_name(origin: &str) -> Option<String> {
    trace!("Getting project name for: {}", origin);
    let project_regex =
        Regex::new(r"((http[s]?|ssh)://)?(\S+@)?[^:/]+[:/](?P<project>\S+?)(\.git)?$").unwrap();
    let captures = project_regex.captures(origin)?.name("project")?;
    Some(String::from(captures.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_github_project_name_ssh() {
        let name = get_github_project_name("git@github.com:my_org/my_project.git");
        assert!(name.is_some());
        assert_eq!("my_org/my_project", name.unwrap());
    }

    #[test]
    fn test_get_github_project_name_ssh_no_git() {
        let name = get_github_project_name("git@github.com:my_org/my_project");
        assert!(name.is_some());
        assert_eq!("my_org/my_project", name.unwrap());
    }

    #[test]
    fn test_get_github_project_name_http() {
        let name = get_github_project_name("http://github.com/my_org/my_project.git");
        assert!(name.is_some());
        assert_eq!("my_org/my_project", name.unwrap());
    }

    #[test]
    fn test_get_github_project_name_http_no_git() {
        let name = get_github_project_name("http://github.com/my_org/my_project");
        assert!(name.is_some());
        assert_eq!("my_org/my_project", name.unwrap());
    }
}
