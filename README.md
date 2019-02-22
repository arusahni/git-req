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

1. [Download the binary for your operating system from the release page](https://github.com/arusahni/git-req/releases/latest).
2. Decompress the archive.
3. Place the `git-req` executable somewhere in your `$PATH`.

The first time you run `git req <#>` it will prompt you for API credentials;
use a Personal Access Token (see Profile Settings > Personal Access Token in
GitLab or GitHub).

Configuration
-------------

I plan on introducting a better command line API in the future to manage the
assorted configuration settings.  Currently they can only be managed by editing
these two ini-formatted files.

##### $HOME/.gitreqconfig

This contains global settings. At the moment, it only domain API keys are
stored here.  Edit this if you have to use a new key or remove a bad one.

##### /path/to/project/.git/config

Internal GitHub/GitLab project IDs are cached here under the `[req]` block.
If you change your upstream remote, you may have to edit this property.

The project ID stored here can be edited with `git req --set-project-id PROJECT_ID`.

Contributing
------------

Contributions are welcome! I'm especially looking for:

* Supporting other services (e.g.  BitBucket).
* Rust code reviews. This is my first non-trivial Rust project, so I'd love to be corrected on best practices and patterns.

Non-binary Version
------------------

The last non-binary version of this was v1.0.0. If you don't wish to run (or
compile) the Rust executable, [feel free to use
it](https://github.com/arusahni/git-req/releases/tag/1.0.0).
