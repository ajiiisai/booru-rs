# Publish a release

Release-plz maintains a release PR with version bumps in `Cargo.toml` and
`Cargo.lock`, plus entries in `CHANGELOG.md`. Contributors do not need to edit
these files for each change.

## Configure GitHub once

1. Create a fine-grained personal access token with access to this repository.
   Grant **Contents: Read and write** and **Pull requests: Read and write**.
2. Save the token as the repository Actions secret `RELEASE_PLZ_TOKEN`.
   The bot uses this token so its PRs trigger CI and its tags trigger release notes.
   The default `GITHUB_TOKEN` does not trigger those workflows.
3. Keep the existing `CARGO_REGISTRY_TOKEN` secret with permission to publish
   `booru-rs`. The publishing job uses the existing `crates-io` environment and
   retains its required reviewer approval.
4. Require the `CI Success` check before merging PRs, including release PRs.

See the [release-plz token guide](https://release-plz.dev/docs/github/token)
for token setup and GitHub App alternatives.

## Merge changes

Squash-merge PRs with a Conventional Commit title. For example:

```text
fix(gelbooru): handle string post counts in autocomplete
feat: add a new client
fix!: change the autocomplete return type
```

Fixes normally bump the patch version. Features bump the minor version, including
before 1.0. Mark breaking changes with `!` or a `BREAKING CHANGE:` footer.
Release-plz also checks library API compatibility. Review its proposed version
before merging, especially for breaking changes.

## Release the accumulated changes

1. Merge normal PRs into `main`.
2. Review the bot's release PR and its generated changelog. The bot updates this
   PR as more changes land.
3. Wait for CI to pass, then merge the release PR.
4. Approve the publishing job in the `crates-io` environment.

The release automation validates the code, publishes to crates.io, and creates
`v<version>`. The tag triggers GitHub's generated release notes with PR links,
contributor credits, and a full comparison link. `CHANGELOG.md` remains the
versioned changelog in the repository.

Only merged release PRs trigger publishing. Keep the bot's `release-plz-` branch
prefix. Do not manually bump versions or push release tags for this workflow.

## Retry a failed release

Rerun the failed release automation job for the release commit. Check the run logs
and crates.io first, because publishing a crate cannot be undone by rerunning CI.

If publishing succeeded but release notes failed, run **Release notes** from the
Actions tab and enter the existing tag, such as `v0.3.2`. This workflow only
creates GitHub notes. It does not publish the crate again and leaves an existing
GitHub release unchanged.
