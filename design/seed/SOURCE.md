# SEED Design tokens — vendored source

Files in this directory are byte-for-byte copies from
[daangn/seed-design](https://github.com/daangn/seed-design), Apache-2.0.
`LICENSE` and `NOTICE` are that repository's root files at the same commit.

- Source: https://github.com/daangn/seed-design/tree/22b68ce014c894edf286a2328a96250a36ab33a2/packages/rootage
- Raw base: https://raw.githubusercontent.com/daangn/seed-design/22b68ce014c894edf286a2328a96250a36ab33a2/packages/rootage/
- Commit: `22b68ce014c894edf286a2328a96250a36ab33a2`
- npm package: `@seed-design/rootage-artifacts` 3.0.2
- Fetched: 2026-10-09

`shasum -a 256 design/seed/*.yaml`:

```
6581a27b7e489a67d93e0f4111e597436a422b93d112d5f884921236e862687e  design/seed/color.yaml
3a5638b94857a8e2b3db0d65a70ddda30f5b518815cec8de13b8a1a249c2845b  design/seed/dimension.yaml
efe4324d42fbe233a46c71c47390822be23a1144b56bc576c89c548407c6f684  design/seed/duration.yaml
6eedcbae70018f27cd3a83039e9b780a60429924ba6f29b86feedb0cd83ef6b9  design/seed/font-weight.yaml
cacfc20a4dbba946d801e36a11f5db2e815014a3ba2af3ddb0d2ada8f2eb703c  design/seed/radius.yaml
166fab513fbe2c201c5a2fc1b0c08f60ae7c627600a3e6d1539e7e2110dab31b  design/seed/timing-function.yaml
```

## Updating

```
scripts/sync-seed.sh <full-git-sha>
UPDATE_SEED=1 cargo test --locked seed_tokens
```

Then update the commit, date, and checksums above, and the commit in the header
generated into `src/ui/seed/tokens.rs` (the generator reads it from this file's
`Commit:` line). `src/ui/seed/tokens.rs` is generated from these files: the
brand role's `carrot` references are replaced with the same step of `purple`,
and alpha colors are composited onto each theme's `bg.layer-default`.
