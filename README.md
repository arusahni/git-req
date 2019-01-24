[![Build Status](https://travis-ci.com/arusahni/git-req.svg?branch=master)](https://travis-ci.com/arusahni/git-req)
[![Build status](https://ci.appveyor.com/api/projects/status/qs5cwdpsx1pdt4dg?svg=true)](https://ci.appveyor.com/project/arusahni/git-req)

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
checkout hotfix/jrdevs_new_branch` (no typos!), and starts jamming on some
code.*

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

Simply place the `git-req` executable somewhere in your `$PATH`. The first time
you run `git req <#>` it will prompt you for API credentials; use a Personal
Access Token (see Profile Settings > Personal Access Token in GitLab or
GitHub).

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

Contributing
------------

Contributions are welcome! I'm down for supporting other services (e.g.
BitBucket). Just file a PR!
