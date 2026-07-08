# Releasing Backburner

## First crates.io Publish

1. Confirm the crate metadata in `Cargo.toml`, especially `license`.
2. Create or sign in to a crates.io account.
3. Run a local package check:

   ```sh
   cargo publish --dry-run
   ```

4. Publish the first version manually:

   ```sh
   cargo login
   cargo publish
   ```

crates.io package versions are permanent. A published version cannot be
overwritten.

## Enable Trusted Publishing

After the first manual publish, configure Trusted Publishing for future CI
publishes in the crates.io settings for the `backburner` crate:

- Owner: `rollingstorms`
- Repository: `backburner`
- Workflow: `publish.yml`
- Environment: `release`

This lets GitHub Actions publish with a short-lived OIDC token instead of a
long-lived crates.io token stored in GitHub secrets.

## Publish a New Version

1. Update `version` in `Cargo.toml`.
2. Update release notes if present.
3. Run the local checks:

   ```sh
   cargo fmt --all --check
   cargo clippy --all-targets --locked -- -D warnings
   cargo test --locked
   cargo publish --dry-run --locked
   ```

4. Commit the version bump.
5. Tag and push:

   ```sh
   git tag "v$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "backburner") | .version')"
   git push origin main --tags
   ```

The `Publish` GitHub Actions workflow verifies that the tag matches
`Cargo.toml`, reruns the checks, and publishes to crates.io.
