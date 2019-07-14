use crate::remotes::{MergeRequest, Remote};
use log::{debug, trace};
use regex::Regex;
use reqwest;
use serde_derive::{Deserialize, Serialize};

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

    fn get_req_branch(&mut self, mr_id: i64) -> Result<String, &str> {
        Ok(format!("pr/{}", mr_id))
    }

    fn get_req_names(&mut self) -> Result<Vec<MergeRequest>, &str> {
        retrieve_github_project_pull_requests(self)
    }

    fn has_useful_branch_names(&mut self) -> bool {
        false
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
fn query_github_api(url: reqwest::Url, token: String) -> reqwest::Response {
    let client = reqwest::Client::new();
    client
        .get(url)
        .header("Authorization", format!("token {}", token))
        .send()
        .expect("failed to send request")
}

/// Get the pull requests for the current project
fn retrieve_github_project_pull_requests(
    remote: &GitHub,
) -> Result<Vec<MergeRequest>, &'static str> {
    trace!("Querying for GitHub PR for {:?}", remote);
    let url = reqwest::Url::parse(&format!("{}/{}/pulls", remote.api_root, remote.id)).unwrap();
    let mut resp = query_github_api(url, remote.api_key.to_string());
    debug!("PR list query response: {:?}", resp);
    let buf: Vec<GitHubPullRequest> = match resp.json() {
        Ok(buf) => buf,
        Err(_) => {
            return Err("failed to read API response");
        }
    };
    Ok(buf.into_iter().map(github_to_mr).collect())
}

/// Extract the project name from a Github origin URL
pub fn get_github_project_name(origin: &str) -> String {
    trace!("Getting project name for: {}", origin);
    let project_regex = Regex::new(r".*:(.*/\S+)\.git\w*$").unwrap();
    let captures = project_regex.captures(origin).unwrap();
    String::from(&captures[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_github_project_name() {
        let name = get_github_project_name("git@github.com:my_org/my_project.git");
        assert_eq!("my_org/my_project", name);
    }
}
