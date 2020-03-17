[![Travis build status](https://travis-ci.com/arusahni/git-req.svg?branch=master)](https://travis-ci.com/arusahni/git-req)
[![Appveyor build status](https://ci.appveyor.com/api/projects/status/qs5cwdpsx1pdt4dg?svg=true)](https://ci.appveyor.com/project/arusahni/git-req)
[![Latest version](https://img.shields.io/crates/v/git-req.svg?style=flat)](https://crates.io/crates/git-req)

git-req
=======

*Check out merge requests from your GitLab/GitHub hosted repos with ease!*

Why?
----

**jrdev**: Hey @aru, can you verify the issue you reported is fixed by mr 17?  
**aru**: Oh? OK.  
*aru switches to a browser, navigates to his org's GitLab instance, finds the
project, clicks to the merge requests view, finds the MR, reads the branch name
(`hotfix/jrdevs_new_branch`), switches back to the terminal, inputs `git
checkout hotfix/jrdevs_new_branch` (no typos!), and starts reviewing.*

---

That sucks. Too much context switching, too many clicks.  You know what's
easier?

```shell
$ git req 17
Switched to branch 'hotfix/jrdevs_new_branch'
```

That's exactly what `git-req` does.

Installation
------------

1. Install it:
    * Homebrew (MacOS)
        1. `brew tap install arusahni/git-req`
        2. `brew install git-req`
    * AUR (Arch Linux)
        1. `yay -Sy git-req`
    * DEB (Debian / Ubuntu Linux)
        1. [Download the `*.deb` file from the release page](https://github.com/arusahni/git-req/releases/latest).
        2. `dpkg -i /path/to/file.deb`
    * Everyone else
        1. [Download the binary for your operating system from the release page](https://github.com/arusahni/git-req/releases/latest)
        2. Decompress the archive
        3. Place the `git-req` executable somewhere in your `$PATH`

The first time you run `git req <#>` it will prompt you for API credentials;
use a Personal Access Token.
[This wiki page](https://github.com/arusahni/git-req/wiki/API-Keys) has
instructions on locating these on both GitLab and GitHub.

Configuration
-------------

`git-req` maintains two levels of configuration: Global and Project.

#### Global

Per-domain API keys are stored in the global scope, so your API keys can be
used across projects.

To clear the API key: `git req --clear-domain-key`
To change the API key: `git req --set-domain-key NEW_KEY`

#### Project

Project IDs are stored in the project scope. This ID is tied to the git host
being used.  If you change your upstream remote, you may have to edit this
property.

To clear the project ID: `git req --clear-project-id`
To change the project ID: `git req --set-project-id PROJECT_ID`


Completions
-----------

Completions are available for ZShell, Bash, and Fish shells.

**ZShell**

```bash
git req --completions zsh > /path/to/zfunc/location/_git-req
rm ~/.zcompdump
exec zsh
```

**Bash**

```bash
git req --completions bash > git-req-completions.sh
source git-req-completions.sh  # add this to your .bashrc!
```

**Fish**

```bash
git req --completions fish > git-req-completions.fish
source git-req-completions.fish
```

Contributing
------------

Contributions are welcome! I'm especially looking for:

* Supporting other services (e.g.  BitBucket).
* Rust code reviews. This is my first non-trivial Rust project, so I'd love to
  be corrected on best practices and patterns.

Non-binary Version
------------------

The last non-binary version of this was v1.0.0. If you don't wish to run (or
compile) the Rust executable, [feel free to use
it](https://github.com/arusahni/git-req/releases/tag/1.0.0).
