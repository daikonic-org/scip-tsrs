# Releases and crates.io publishing

The [Build and release workflow](../.github/workflows/build.yml) tests Apple silicon macOS, Linux ARM64, and Linux x86-64. A version tag must match `Cargo.toml`, including any prerelease suffix. GitHub binary archives and crates.io source publication both depend on the complete test matrix.

The current release is [scip-tsrs 0.2.2](https://crates.io/crates/scip-tsrs/0.2.2), also available as [GitHub binaries](https://github.com/daikonic-org/scip-tsrs/releases/tag/v0.2.2). Install it with `cargo install scip-tsrs --version 0.2.2 --locked`. The first registry publication was `0.2.2-alpha.1`.

## Package verification

Linux x86-64 CI creates the actual `.crate` archive, unpacks it, and runs its tests in release mode against registry dependencies. This checks the normalized manifest and the sources a crates.io installation will use. macOS and Linux ARM64 run the release suite from the checkout.

The publish job repeats the tag check, then uses `cargo publish --locked --no-verify --registry crates-io`. Verification has already compiled and tested the source archive in release mode; `--no-verify` avoids a second Cargo verification build using the default development profile.

The source package contains the application, tests, Python release helpers, user documentation, lockfile, and license/attribution files. Scratch outputs, compiled artifacts, and local indexes are excluded by the manifest's include list.

## First publication

[crates.io requires an API token for a new crate's first publication](https://crates.io/docs/trusted-publishing). The workflow supports this explicitly; it does not fall back to a long-lived token during automatic releases.

1. Sign in to the intended crates.io owner account and verify its email.
2. Create a short-lived API token that allows the initial `scip-tsrs` publication. Add it directly to the repository's **`crates-io` GitHub environment** as the secret **`CARGO_REGISTRY_TOKEN`**. Keep credentials out of issues, chat, and committed files.
3. Commit the release version, lockfile, and changelog. Wait for that commit's CI to pass, then push the matching version tag. For the first registry release this is `v0.2.2-alpha.1`.
4. The tag run creates the GitHub binary release. Automatic crates.io publishing stays off until its repository variable is enabled below.
5. Run the workflow on that tag with the explicit bootstrap input:

   ```sh
   gh workflow run build.yml --repo daikonic-org/scip-tsrs \
     --ref v0.2.2-alpha.1 -f bootstrap=true
   ```

The full test matrix runs before the bootstrap publish job. A manual run does not recreate the GitHub binary release. Keep the temporary token until the first upload is confirmed, then complete the trusted-publishing setup.

## Trusted publishing

After the crate exists, open its crates.io **Settings → Trusted Publishing** and add:

| Setting | Value |
| --- | --- |
| Provider | GitHub |
| Repository owner | `daikonic-org` |
| Repository | `scip-tsrs` |
| Workflow filename | `build.yml` |
| Environment | `crates-io` |

The GitHub environment permits version tags matching `v*`. The publish job alone has `id-token: write`; the auth action is pinned to a full commit and revokes its temporary token when the job ends.

Validate OIDC without uploading another version:

```sh
gh workflow run build.yml --repo daikonic-org/scip-tsrs \
  --ref v0.2.2-alpha.1 -f dry_run=true
```

After the dry run succeeds, revoke the bootstrap API token and remove the environment secret. Enable automatic publishing for future version tags:

```sh
gh variable set CRATES_IO_PUBLISH_ENABLED --repo daikonic-org/scip-tsrs --body true
```

The variable prevents automatic publication before the one-time setup is complete. Manual tag runs remain available for bootstrap and validation. Branches and pull requests do not publish crates. Concurrent publication jobs are serialized without cancelling an in-flight upload.

## Future releases

Update the version and changelog, refresh the workspace lockfile, and wait for CI before pushing the matching tag. With trusted publishing configured and `CRATES_IO_PUBLISH_ENABLED=true`, the tag run publishes both GitHub archives and the crate after all three platform jobs pass. Release numbers use plain SemVer, such as `0.2.2`; the README retains the alpha maturity disclaimer. Explicit prerelease versions remain supported. The workflow marks a GitHub release as a prerelease only when its version has a prerelease suffix.

## Compiler dependencies

The current source pins eight direct tsrs dependencies to `=0.24.0`; `Cargo.lock` records checksums for all 24 required tsrs crates. The 0.2.2 documentation update adds `tsrs_ls` and its `tsrs_lsproto` and `tsrs_projectutil` dependencies. It calls a public documentation helper in-process and does not run an LSP server or change the checker version. The three added registry packages identify publication commit `383b683154ecfc0e17914b21363d0879411d23e9` in their VCS metadata.

The published `0.2.2-alpha.1` source had seven direct dependencies and 21 tsrs crates. Before switching that release from the original git dependency, all 494 packaged Rust, TypeScript, and JSON source files were checked against commit `86de03ae428a956686af29177b50f0fdf4aee059` and matched. Those registry packages identify the same publication commit above.

The upstream [0.25.0 CLI/npm release](https://github.com/oxc-project/tsrs/releases/tag/v0.25.0) states that the type checker is unchanged from 0.24.0. At setup time, the Rust libraries on crates.io were still 0.24.0. Publishing scip-tsrs therefore uses those registry libraries without requiring a compiler fork or a Git-only dependency.

The [published performance comparison](benchmarks.md) remains attributed to scip-tsrs 0.2.1 and the checker revision actually measured.
