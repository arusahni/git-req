_git_req() {
	local cur
    _init_completion || return
    COMPREPLY=( $(compgen -W '-l --list --set-project-id --clear-project-id --set-domain-key --clear-domain-key' -- "$cur") )
} &&
complete -F _git_req git-req