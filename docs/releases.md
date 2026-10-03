# Publish a release

Release-plz maintains a release PR with version changes and a changelog.
Do not edit `Cargo.toml`, `Cargo.lock`, or `CHANGELOG.md` for each contribution.

## Configure GitHub

1. Create a fine-grained personal access token for this repository.
   Grant **Contents: Read and write** and **Pull requests: Read and write**.
2. Save the token as the Actions secret `RELEASE_PLZ_TOKEN`.
3. Set `CARGO_REGISTRY_TOKEN` to a token with permission to publish `booru-rs`.
4. Require the `CI Success` check for contribution PRs and release PRs.

The bot uses `RELEASE_PLZ_TOKEN` so its PRs and tags trigger workflows.
The default `GITHUB_TOKEN` does not trigger those workflows.
The publish job uses the `crates-io` environment and its approval rules.
See the [release-plz token guide](https://release-plz.dev/docs/github/token) for setup details and GitHub App alternatives.

## Prepare changes

Squash-merge PRs with a Conventional Commit title:

```text
fix(gelbooru): handle string post counts in autocomplete
feat: add a new client
fix!: change the autocomplete return type
```

Fixes normally increase the patch version. Features increase the minor version even before 1.0.
For a breaking change, use `!` or a `BREAKING CHANGE:` footer.
Review the bot's version proposal and API compatibility results before the release.

## Publish

1. Merge contribution PRs into `main`.
2. Review the bot's release PR, version changes, and changelog.
3. Wait for CI to pass.
4. Merge the release PR. Keep its `release-plz-` branch prefix.
5. If the `crates-io` environment requires approval, approve the publish job.

The automation validates the code, publishes the crate, and creates `v<version>`.
The tag triggers GitHub release notes. `CHANGELOG.md` remains the repository's version history.
Do not manually increase versions or create release tags for this process.

## Understand workflow triggers

[Release automation](../.github/workflows/release-plz.yml) runs on pushes to `main` and manual dispatch.
It attempts a release when either condition applies:

- The commit belongs to a merged release PR with a `release-plz-` branch prefix.
- The version in `Cargo.toml` has no `v<version>` tag.

The second condition permits another release attempt after a failure.
A documentation-only push can therefore trigger the publish job when the current version has no tag.

The bot updates its release PR after pushes that change `src/`, `tests/`, `examples/`, `Cargo.toml`, or `Cargo.lock`.
Manual dispatch also permits a release PR update.

## Retry a failed release

1. Check the run logs and crates.io for the current version's publication status.
2. Rerun the failed release job for the release commit.

If the crate is published but GitHub release notes are missing, run **Release notes** from the Actions tab.
Enter the tag, such as `v2.0.0`.
[Release notes](../.github/workflows/release.yml) creates notes for a tag that exists and skips a GitHub release that exists.
It does not publish the crate.
