# Changelog

## 0.2.2-alpha.1

- Prepare crates.io distribution using exact tsrs 0.24.0 registry dependencies. The 21 tsrs packages' 494 packaged Rust, TypeScript, and JSON source files match the original pinned checker source.
- Test the packaged source archive against registry dependencies in release mode on Linux, alongside the macOS and ARM64 Linux release tests.
- Add tag-checked crates.io publishing after the complete build matrix, with a manual first-publish token path and OIDC trusted publishing for subsequent releases.
- Include user documentation, tests, and attribution in the source package. Keep the existing v0.2.1 GitHub release unchanged.

## 0.2.1

- Preserve JSDoc tags in standard SCIP documentation, including parameter and return descriptions, templates, deprecation, examples, and custom tags.
- Keep fenced and indented examples, Unicode text, and comments containing only tags. Add a checked JavaScript fixture and verify occurrence tuples remain unchanged.

## 0.2.0

- Follow TypeScript project references automatically, including empty solution configs and repeated references. Emit one index with unique documents and cross-project symbol identities.
- Reclaim compiler programs between projects; keep the checker in-process and avoid invoking the build or writing generated files.
- Infer a configuration in memory with `--infer-tsconfig` for projects without `tsconfig.json`.
- Resolve inherited base members through the checker so implementations link to the declaration on the original interface or class.
- Keep the standard SCIP schema, typed ranges, symbol naming, and atomic output. Add end-to-end tests for project graphs, cross-project references, inherited members, and inferred TypeScript/JavaScript projects.

## 0.1.0

Initial native indexer with pinned tsrs, SCIP 0.10 output, TypeScript-compatible symbol names, local symbols, signatures, documentation, and direct implementation relationships. Native release archives cover macOS and Linux on ARM64 and x86-64.
