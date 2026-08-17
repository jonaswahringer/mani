# Rebase

Replay your branch's commits on top of a new base. This keeps the history linear
without adding a merge commit.

## Use it when

- Your feature branch has fallen behind `main`.
- You want to reorder, squash, rename, or edit local commits.
- You want a clean story before opening a pull request.

## Common flow

```sh
git fetch origin
git rebase origin/main
```

For an interactive rewrite:

```sh
git rebase -i origin/main
```

## If there is a conflict

1. Fix the conflicted files.
2. Stage each resolved file with `git add <file>`.
3. Continue with `git rebase --continue`.

Use `git rebase --abort` to return to the state before the rebase.

## Watch out

Avoid rebasing commits that other people already depend on. Rebasing rewrites
commit identities, even when the file contents stay the same.
