use std::fmt;
use std::io::{stdin, stdout, Write};
use log::{debug,info};
use regex::Regex;
use reqwest;
use reqwest::header::Headers;
use serde_derive::{Deserialize, Serialize};
use serde_json;
use crate::git;

pub trait Remote {
    fn get_project_id(&mut self) -> Result<&str, &str>;
    fn get_req_branch(&mut self, mr_id: &i64) -> String;
}

impl fmt::Display for Remote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Remote")
    }
}

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

    fn get_req_branch(&mut self, mr_id: &i64) -> String {
        format!("pr/{}", mr_id)
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

    fn get_req_branch(&mut self, mr_id: &i64) -> String {
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

fn query_gitlab_project_id(remote: &GitLab) -> Result<i64, &'static str> {
    let client = reqwest::Client::new();
    let url = reqwest::Url::parse(&format!("{}/projects/{}%2F{}", remote.api_root, remote.namespace, remote.name)).unwrap();
    let mut headers = Headers::new();
    headers.set_raw("PRIVATE-TOKEN", remote.api_key.to_string());
    debug!("{:?}", headers);
    let mut resp = client.get(url)
        .headers(headers)
        .send()
        .expect("failed to send request");
    debug!("Response: {:?}", resp);
    if !resp.status().is_success() {
        return Err("bad server response");
    }
    let buf: GitLabProject = resp.json().expect("failed to read response");
    debug!("{:?}", buf);
    return Ok(buf.id);
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

fn load_project_id() -> Option<String> {
    match git::get_config("projectid") {
        Some(project_id) => Some(project_id),
        None => {
            debug!("No project ID found");
            None
        }
    }
}

fn query_gitlab_branch_name(remote: &GitLab, mr_id: &i64) -> String {
    let client = reqwest::Client::new();
    let url = reqwest::Url::parse(&format!("{}/projects/{}/merge_requests/{}", remote.api_root, remote.id, mr_id)).unwrap();
    let mut headers = Headers::new();
    headers.set_raw("PRIVATE-TOKEN", remote.api_key.to_string());
    // debug!("{:?}", headers);
    let mut resp = client.get(url)
        .headers(headers)
        .send()
        .expect("failed to send request");
    debug!("Response: {:?}", resp);
    let buf: GitLabMergeRequest = resp.json().expect("failed to read response");
    buf.source_branch
}

fn get_github_project_name(origin: &str) -> String {
    let project_regex = Regex::new(r".*:(.*/\S+)\.git\w*$").unwrap();
    let captures = project_regex.captures(origin).unwrap();
    String::from(&captures[1])
}

fn get_gitlab_project_name(origin: &str) -> String {
    let project_regex = Regex::new(r".*/(\S+)\.git$").unwrap();
    let captures = project_regex.captures(origin).unwrap();
    String::from(&captures[1])
}

fn get_gitlab_project_namespace(origin: &str) -> String {
    let project_regex = Regex::new(r".*/(\S+)/\S+\.git$").unwrap();
    let captures = project_regex.captures(origin).unwrap();
    String::from(&captures[1])
}

pub fn get_domain(origin: &str) -> Result<&str, String> {
    let domain_regex = Regex::new(r"((http[s]?|ssh)://)?(\S+@)?(?P<domain>([^:/])+)").unwrap();
    let captures = domain_regex.captures(origin);
    if captures.is_none() {
        return Err(String::from("invalid remote set"))
    }
    Ok(captures.unwrap().name("domain").map_or("", |x| x.as_str()))
}

pub fn get_remote(origin: &str) -> Result<Box<Remote>, String> {
    let domain = get_domain(origin)?;
    Ok(match domain {
        "github.com" => Box::new(GitHub {
            id: get_github_project_name(origin),
            name: get_github_project_name(origin),
            origin: String::from(origin),
            api_root: String::from("https://api.github.com/repos"),
        }),
        gitlab_domain => {
            let mut remote = GitLab {
                id: String::from(""),
                domain: String::from(gitlab_domain),
                name: get_gitlab_project_name(origin),
                namespace: get_gitlab_project_namespace(origin),
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
                    stdin().read_line(&mut newkey).expect("Did not input a correct key");
                    debug!("{}", &newkey);
                    git::set_req_config(&domain, "apikey", &newkey.trim());
                    String::from(newkey.trim())
                }
            };
            info!("API Key: {}", &apikey);
            remote.api_key = apikey;
            let project_id = match load_project_id() {
                Some(x) => x,
                None    => {
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
