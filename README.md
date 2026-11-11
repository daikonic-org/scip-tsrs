# scip-tsrs

`scip-tsrs` is a native TypeScript indexer. It loads a `tsconfig.json` with the in-process [tsrs checker](https://github.com/oxc-project/tsrs) and writes a [SCIP](https://github.com/scip-code/scip) index. The executable bundles its TypeScript standard library and runs without a Node process.

## Install and use

Download the archive for your platform from [GitHub Releases](https://github.com/daikonic-org/scip-tsrs/releases). Extract it and put `scip-tsrs` on your `PATH`. Releases include SHA-256 checksums.

To build from source with Rust 1.99 or newer:

```sh
cargo install --git https://github.com/daikonic-org/scip-tsrs --tag v0.2.1 --locked
```

Run the indexer with a project directory or a path to its TypeScript config. Without a project argument, it uses the current directory.

```sh
scip-tsrs index ./packages/service --output ./index.scip
scip-tsrs scip ./tsconfig.json --project-root .
scip-tsrs dump ./index.scip
```

- Project references are followed automatically, including empty solution configs. Sources are never built or written; one `index.scip` contains the combined documents. `--no-project-references` restricts indexing to one config.
- `--infer-tsconfig` supports projects without a config using an in-memory compiler configuration.
- `--project-root` sets the base directory for SCIP document paths. The default is the config directory.
- `--max-file-byte-size` defaults to 1,000,000 bytes. It limits emitted documents while keeping their sources available to the checker. Use `0` to emit every root file.
- `--minimal` emits occurrence and bare symbol data for comparisons. Enrichment preserves occurrence identities, positions, and roles.
- `--lib` selects an alternate TypeScript standard-library directory. The default is tsrs’s bundled TypeScript 7.1 library.
- `scip-tsrs inspect PROJECT --file src/example.ts` prints checker resolution for one file. `scip-tsrs diagnose PROJECT` runs the compiler diagnostics separately.

## Output

The index uses the SCIP 0.10 schema with typed ranges and explicit UTF-16 columns. Symbol names follow `scip-typescript` conventions, including package-qualified names, `key<N>:` object members, and document-local `local N` IDs. The same local ID can occur in different files. Consumers should key it by document as well as symbol. scip-java uses the same per-file scoping model with compiler symbols as cache keys. Checker and standard-library differences between TypeScript 7 and TypeScript 5 can still change exact tuples. [scip-java local symbol cache](https://github.com/scip-code/scip-java/blob/3793579c53edb5c0a51c69315c07faca7376dc3e/scip-shared/src/main/java/org/scip_code/scip_java/shared/LocalSymbolsCache.java)

The default output includes symbol kinds, display names, source signatures, JSDoc descriptions and tags, enclosing ranges and symbols, and metadata for referenced global symbols defined outside the index. JSDoc parameters, returns, templates, deprecation, fenced and indented examples, and custom tags use the standard SCIP documentation field. The design follows the metadata structure used by [rust-analyzer](https://github.com/rust-lang/rust-analyzer/blob/2a362a41a8eb6e090f1a12d9b712df85af6f0c33/crates/rust-analyzer/src/cli/scip.rs).

Class and interface relationships follow compiler-resolved direct `extends` and `implements` clauses. Both use SCIP’s `is_implementation` bit. Instance methods and properties that match a compiler-resolved base member by name and kind (including members inherited by that base) also set `is_reference`. The producer emits explicit `Class` and `Interface` kinds and retains the heritage occurrence in the source. Consumers that need to distinguish the exact keyword can read that syntax. [SCIP relationship semantics](https://github.com/scip-code/scip/blob/main/scip.proto) · [scip-java visitor](https://github.com/scip-code/scip-java/blob/3793579c53edb5c0a51c69315c07faca7376dc3e/scip-javac/src/main/java/org/scip_code/scip_java/javac/ScipVisitor.java)

This is a prerelease. Tests cover typed ranges, Unicode columns, local shadowing, inherited interface members, project-reference graphs, in-memory configuration, and atomic output. Full override compatibility checking, complex mixins, and JSX-specific parity remain open areas.

## Development

```sh
cargo fmt --check
cargo test --release --locked
```

The tsrs revision and Rust dependencies are pinned by the manifest and lockfile. Binary releases are built and tested on Apple silicon macOS, Linux ARM64, and Linux x86-64. The repository is licensed under Apache-2.0. See `NOTICE` and `NOTICE-tsrs` for attribution.
