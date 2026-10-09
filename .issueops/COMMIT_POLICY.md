---
name: COMMIT_POLICY.md
description: Commit message format and scope rules; read before committing.
---

# Commit Policy

Sibling: [CONVENTIONS.md](CONVENTIONS.md). Legend: **Enforced** = CI fails; **Observed** = practice in this repo's history, nothing fails.

## Enforced: `committed.toml` (crate-ci/committed)

- Style: Conventional Commits, `<type>(<scope>): <subject>`; scope optional.
- Allowed types: `fix`, `feat`, `chore`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`. (`revert` is not in the list but `cliff.toml` has a parser for it; whether `committed` accepts it is Unknown / not confirmed.)
- Subject: imperative verb (`imperative_subject = true`), no trailing punctuation, capitalization not required. Line-length and subject-length checks are disabled (`0`).
- Merge commits are allowed (`merge_commit = true`).
- Why it matters: `cliff.toml` sets `filter_unconventional = true`, so a non-conventional commit silently disappears from the changelog. The CI `commits` job exists to prevent that.

## Enforced: CI range rule (`.github/workflows/ci.yml`, job `commits`, PRs only)

- Runs `crate-ci/committed@v1.1.11 -vv` on `from..HEAD` with `fetch-depth: 0`.
- `from` is the PR base SHA, except when the PR head contains upstream tip `b1f31dd26bd4e9e513885e39edb78d0850a5d1fe` (the zoetrope fork point) and the base does not; then `from` is `b1f31dd`. Once `main` contains `b1f31dd` (it does, via `195dbb8`/`1c744d2`), later PRs lint `base..head`.
- Consequence: commits from before the fork point (upstream zoetrope history) are never linted. Everything after it is.
- Background: `114e4f9` introduced this rule, pinned `committed` to v1.1.11 (`@master` no longer resolved), and added `build`/`ci` types.

## Enforced/derived: changelog grouping (`cliff.toml`, git-cliff)

| Commit | Changelog group |
| --- | --- |
| `feat` | Features |
| `fix` | Bug Fixes |
| `refactor` | Refactor |
| `doc*` | Documentation |
| `perf` | Performance |
| `style` | Styling |
| `test` | Testing |
| `chore`, `ci`, `build` | Miscellaneous Tasks |
| body contains `security` | Security |
| `revert` | Revert |

- Skipped from the changelog: `chore(release): prepare for`, `chore: release`, `chore(deps…)`, `chore(pr)`, `chore(pull)`, and commits whose id matches `0000000` (release-plz).
- `(#N)` / `(scope #N)` issue suffixes are stripped from subjects by a preprocessor; PR numbers are re-linked from the remote.
- A `!`/breaking commit is rendered with `[**breaking**]`. Tags matching `v?[0-9].*`; `beta|alpha` skipped, `rc` ignored.
- Choose the type for the group you want: tooling and workflows are `build`/`ci`, repo-process files under `.issueops/` have used `chore(issueops)`.

## Observed: scopes in this repo's history

`provider`, `tailer`, `ui`, `herdr`, `release`, `identity`, `fork`, `issueops`, `research`. Scope names a module or area, not a file. (Observed from `git log`; no list is enforced.)

## Observed: Lore body (not enforced)

Of the 27 commits by `m16khb` in `b1f31dd..HEAD`, 24 carry a `Lore:` body. The three without it are the fork-import and identity commits (`195dbb8`, `ef49e61`; trailer-style `Constraint:`/`Directive:` lines) and the PR #2 merge commit `1c744d2` (body `Closes #1`).

~~~text
<type>(<scope>): <imperative subject>

Lore:
- Intent: <what this achieves>
- Why: <the failing observation or constraint behind it>
- Changes:
  - <concrete change>
- Verify: <commands actually run and their result>
- Risk: <blast radius, or "Low; test-only">
~~~

Examples by short sha:

- `114e4f9` (`ci:`): Intent / Why / Changes / Verify (`committed b1f31dd..HEAD 0 errors; typos rc=0; cargo fmt --check; cargo clippy -D warnings`) / Risk.
- `16081ca` (`fix(tailer)`): Why cites the review finding and measured sizes; Changes names the regression tests, including that one failed before the fix.
- `c0a5bf3` (`fix(herdr)`): short form, `Risk: Low.`
- `d58ae7b` (`test(herdr)`): Verify names the CI job that failed and the platform re-run.
- `2807a63` (`build(release)`): Verify records a fake-release run and a tampered-checksum run; Risk states the workflow only runs on a tag push.
- `bd7f035` (`refactor(ui)`): Why names the consumer that left; Risk states the feature was inert.
- `bc05b18` (`feat(provider)`): Changes lists the behavior pinned.

Rules drawn from them (observed, so follow them but they are not checked):

- Verify lists only commands that were run; do not claim untested platforms (say "Windows re-run in CI").
- Why states the observed failure or constraint, not the desire.
- Keep the type honest: `fix` only for a defect, `refactor` for removal without behavior change, `test` for test-only.
- Cherry-picked upstream work is given a conventional type and says so in the body (`4ddfee4`: "subject given a conventional type").
- Closing an issue: PRs are merged with a merge commit (not squash) so imported upstream history and authorship survive; the merge subject carries `(#PR)` and the body `Closes #N` (`1c744d2`).

## Safety

- Prefer small atomic commits; do not stage unrelated changes.
- Run the gates in [conventions/overview.md](conventions/overview.md) that fit the change before committing (`cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --locked --all-features`; `typos` if installed). Record what ran in `Verify:`.
- Manually inspect secret-like paths or credential changes before committing.
- Upstream (pre-`b1f31dd`) history is not rewritten.
