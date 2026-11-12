# scip-tsrs

> **Alpha.** The CLI, checker compatibility, and index coverage are still moving.
> Pin exact versions and validate the output on your projects before adopting it
> at scale. See [current limitations](docs/scip.md#current-limitations).

`scip-tsrs` is a native TypeScript and JavaScript [SCIP](https://github.com/scip-code/scip) indexer written in Rust. It loads your `tsconfig.json` with the in-process [tsrs checker](https://github.com/oxc-project/tsrs) and emits definitions, references, documentation, and implementation relationships. The executable bundles its TypeScript standard library and runs without Node.js.

## Performance

On `@sittir/typescript`, comparing scip-typescript **0.4.0** with scip-tsrs **0.2.1**, using normal metadata-enabled output:

| Metric | scip-typescript | scip-tsrs | Difference |
| --- | ---: | ---: | --- |
| Wall time | 30.78 s | 1.77 s | **17.4× faster** |
| CPU time | 36.31 s | 1.72 s | **21.1× less CPU time** |
| Peak memory (RSS) | 2,329 MiB | 343 MiB | **85.3% lower** |
| SCIP file size | 29.68 MiB | 35.97 MiB | 21.2% larger |

Timings and memory are medians of three fresh-process runs on a busy ARM64 macOS host with warm filesystem caches. These measure the indexer process; consumer import, embeddings, and long-running daemon memory are separate costs. See the [methodology, raw samples, and occurrence comparison](docs/benchmarks.md).

## Beyond scip-typescript 0.4.0

- **Explicit symbol kinds.** Classes, interfaces, methods, properties, variables, parameters, and other declarations have typed SCIP metadata, so consumers can distinguish them without guessing from symbol spelling.
- **Structured signatures and containment.** Source signatures use `signature_documentation`; enclosing symbols and occurrence ranges describe where declarations belong.
- **Implementation and override links.** Direct class and interface heritage, plus matching instance members, use standard SCIP relationships. Member lookup includes declarations inherited by the base interface or class.
- **Named local-symbol metadata.** Document-scoped `local N` identities also carry display names and kinds, including shadowed variables. Local IDs remain scoped to their document.
- **External-symbol metadata.** Referenced global symbols defined outside the emitted documents have symbol information, with kinds, signatures, and documentation where available.
- **Standalone native distribution.** One executable with bundled standard libraries; no Node.js runtime or compiler subprocess is required.

These additions were checked against 0.4.0 output on the benchmark package. Project references, definitions and references, and JSDoc are also supported; they are shared capabilities rather than new additions. See [SCIP output and compatibility](docs/scip.md) for the exact semantics and limits.

## Install and use

Download the archive for your platform from [GitHub Releases](https://github.com/daikonic-org/scip-tsrs/releases/tag/v0.2.1). Extract it and put `scip-tsrs` on your `PATH`. Releases include SHA-256 checksums. Supported binaries cover Apple silicon macOS, Linux ARM64, and Linux x86-64.

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

Project references are followed automatically, including empty solution configs, and combined into one index without building or writing project sources. JavaScript follows your compiler configuration; config-free TypeScript and JavaScript projects can use `--infer-tsconfig`. See [CLI usage](docs/usage.md) for options, monorepos, library selection, and diagnostics.

## SCIP output and coverage

The index uses the standard [SCIP protobuf](https://github.com/scip-code/scip/blob/main/scip.proto), through the `scip` Rust crate 0.10.0, with typed ranges and explicit UTF-16 columns. Symbol names follow scip-typescript conventions, including package-qualified names and `key<N>:` object members. Direct JSDoc descriptions and tags use the standard documentation field.

Alpha limitations include missing `ReadAccess`/`WriteAccess` occurrence flags, inherited JSDoc and link-target resolution, full override compatibility checking, complex mixins, and JSX-specific parity. TypeScript 7 checker and library differences also prevent exact scip-typescript tuple equality. [Compatibility details and known gaps](docs/scip.md#current-limitations).

## Documentation

- [CLI usage and project references](docs/usage.md)
- [SCIP fields, local variables, relationships, and limitations](docs/scip.md)
- [Benchmarks, raw samples, and occurrence compatibility](docs/benchmarks.md)
- [Changelog](CHANGELOG.md)

## Development

```sh
cargo fmt --check
cargo test --release --locked
```

Tests cover typed ranges, Unicode columns, local shadowing, inherited interface members, project-reference graphs, in-memory configuration, JSDoc tags, and atomic output. The tsrs revision and Rust dependencies are pinned by the manifest and lockfile.

The repository is licensed under Apache-2.0. See [NOTICE](NOTICE) and [NOTICE-tsrs](NOTICE-tsrs) for attribution.
