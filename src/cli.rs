use clap::{arg, Parser};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(
    bin_name = "git req",
    author,
    version,
    about = "Switch between merge/pull requests in your GitLab and GitHub repositories with just the request ID",
    long_about = None
)]
pub struct Cli {
    #[arg(
        short = 'u',
        long = "use-remote",
        help = "The remote to be used for this command"
    )]
    pub remote_name: Option<String>,

    #[arg(
        short,
        long,
        help = "List all open requests against the repository",
        conflicts_with_all=[
            "new_project_id",
            "clear_project_id",
            "new_domain_key",
            "clear_domain_key",
            "new_default_remote",
            "generate_completions",
        ]
    )]
    pub list: bool,

    #[arg(
        long = "set-project-id",
        help = "Set a project ID for the current repository",
        conflicts_with_all=[
            "clear_project_id",
            "new_domain_key",
            "clear_domain_key",
            "new_default_remote",
            "generate_completions",
        ]
    )]
    pub new_project_id: Option<String>,

    #[arg(
        long,
        help = "Clear the project ID for the current repository",
        conflicts_with_all=[
            "new_domain_key",
            "clear_domain_key",
            "new_default_remote",
            "generate_completions",
        ]
    )]
    pub clear_project_id: bool,

    #[arg(
        long = "set-domain-key",
        help = "Set the API key for the current repository's domain",
        conflicts_with_all=[
            "clear_domain_key",
            "new_default_remote",
            "generate_completions",
        ]
    )]
    pub new_domain_key: Option<String>,

    #[arg(
        long,
        help = "Clear the API key for the current repository's domain",
        conflicts_with_all=[
            "new_default_remote",
            "generate_completions",
        ]
    )]
    pub clear_domain_key: bool,

    #[arg(
        long,
        help = "Set the name of the default remote for the repository",
        conflicts_with = "generate_completions"
    )]
    pub new_default_remote: Option<String>,

    #[arg(
        long,
        help = "Generate a shell completion file",
        conflicts_with = "remote_name"
    )]
    pub generate_completions: Option<Shell>,

    #[arg(
        help = "The ID of the MR or PR, or '-' to reference the one previously checked out",
        required_unless_present_any=[
          "new_project_id",
          "clear_project_id",
          "new_domain_key",
          "clear_domain_key",
          "list",
          "new_default_remote",
          "generate_completions",
        ],
        conflicts_with_all=[
            "list",
            "new_project_id",
            "clear_project_id",
            "new_domain_key",
            "clear_domain_key",
            "new_default_remote",
            "generate_completions",
        ]
    )]
    pub request_id: Option<String>,
}
