# Release Process
- [ ] Pull latest from `main` branch.
      `git checkout main && git pull origin main`
- [ ] Determine release version string dependending on whether changes included
      in changelog are API breaking, it's a release candidate, etc.
      The release string should have the format "vX.Y.Z", with a possible
      trailing "-rcN" and follow [semantic versioning](https://semver.org/).
- [ ] Checkout branch named `release-N` where N is the version string.
      `git checkout -b release-N`
- [ ] Update the version field in the package entry of `Cargo.toml` files.
  - The versions must be the same.
  - There are 4 relevant `Cargo.toml` files in the repo:
    - `Cargo.toml`: update the version string.
    - `cairo-vm-cli/Cargo.toml`: update the version string and also the `cairo-vm` dependency version to match the above.
    - `felt/Cargo.toml`: update the version string.
    - `deps/parse-hyperlinks/Cargo.toml`: this vendored dependency needs its version bumped, but does not need to match the other crate versions.
  - [Here](https://github.com/lambdaclass/cairo-rs/pull/948/files) is an example pull request with these changes.
- [ ] Run `cargo update` and `git add Cargo.lock`
- [ ] Update `CHANGELOG.md`:
  - Verify that the changelog is up to date.
  - Add a title with the release version string just below the `Upcoming Changes` section.
- [ ] Commit your changes, push your branch, and create a pull request.
- [ ] Merge after CI and review passes.
- [ ] Pull latest from `main` again.
- [ ] Tag commit with version string and push tag.
      `git tag -a <version string> -m "Release..."`
- [ ] Watch the `publish` workflow run in Github Actions.
- [ ] Verify all the crates are available on crates.io with the correct versions.
  - [cairo-vm](https://crates.io/crates/cairo-vm)
  - [cairo-felt](https://crates.io/crates/cairo-felt)
  - [cairo-take-until-unbalanced](https://crates.io/crates/cairo-take_until_unbalanced)
- [ ] Create a release in Github.
  - Select the recently created tag.
  - Set the title to the version string.
  - If it is a release candidate, mark it as a draft release.
- [ ] Announce release through corresponding channels.

