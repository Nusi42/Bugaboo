# ADR-0036: Public identity and plain commit messages

Status: **Accepted** · 2026-11-01

## Context

The history of the repository is published with it. Until now commits carried the identity Git found on the machine, the working vocabulary of the project (folder and branch names, work packages, milestone codes), session lines of the tool that wrote them, and merges in a form that named a branch and a package. None of that tells a reader of the public history what changed, and some of it is private.

## Decision

- **One public identity.** The maintainer's commits are authored and committed as the name of the GitHub account with its noreply address (`Name <id+Name@users.noreply.github.com>`). The identity is configuration, never code: the local Git configuration of the master and of every work folder, copied by `cargo xtask setup` (from the `origin` master when that is a folder, or from the variable `BUGABOO_PUBLIC_IDENTITY`). The handle itself may appear in files and messages only in its GitHub forms: the noreply address and `github.com/<login>` links.
- **What counts as public.** An identity is public when its address is a GitHub noreply address whose login is its name, or when it is the identity GitHub writes for merges made on its web site. Another contributor's public identity passes as well, because it is not the maintainer's private one; a real name or a private address in the author field never passes. A contribution that arrives with another identity is brought in by the maintainer (a squash or a new commit) rather than by relaxing the rule.
- **Where it is checked.** The commit-msg hook reads the author and committer about to be recorded (`git var GIT_AUTHOR_IDENT`, `GIT_COMMITTER_IDENT`). The push and full checks read the author and committer of every commit that is on no remote yet. Both are part of SEC-PRIVATE, with the private list (words, several words, e-mail addresses; the variable `BUGABOO_PRIVATE_WORDS` names another file) and the refusal of the session key of Claude Code in memory notes.
- **Plain messages.** GIT-COMMIT-MSG enforces: the types feat, fix, perf, refactor, test, docs, build, ci, chore, style and revert; a first line of at most 72 characters that starts in lower case with a verb in the imperative (the imperative is a heuristic: a first word ending in `ed` or `ing` is refused, with a short list of exceptions); merges as `Merge <what it brought>`; the single trailer `Co-Authored-By: Claude <model> <noreply@anthropic.com>`; no claude.ai link or session line; and none of the working vocabulary (folder and branch names, work folder, master, lead, fast-forward, wave, work package and its codes, milestone codes, the acronyms SSOT, ODR and DRY, owner, WIP). The list is in `xtask/src/checks/git_commit_msg/jargon.rs`, once.
- **Rewriting history.** Commits that are on the public remote are never rewritten. History that has never been published may be rewritten once, before the first push, to bring it into line with these rules.

## Consequences

- The published history reads the same for everyone and carries no private identity.
- Until the unpublished history is rewritten, the push and full checks report its old identities and messages.
- A word of the working vocabulary is refused even when it is used in its ordinary sense (a master key, a lead role); the message is reworded.

## Alternatives considered

- **Hard-code the identity in the check:** puts the handle into the code outside its GitHub forms. Rejected.
- **Allow any noreply address:** lets another person's identity pass as the maintainer's. It is accepted only because it is public, and documented.
- **Check the identity only at commit time:** a commit made with `--no-verify` or on another machine would reach the remote. Rejected.

## Enforced by

SEC-PRIVATE (identity, private list, memory-note key) and GIT-COMMIT-MSG (message rules).
