# CLI usage

scip-tsrs is alpha software. Pin a release and validate its output on your projects. The [README](../README.md) covers installation; [SCIP output](scip.md) covers consumer semantics and limitations.

## Index a project

The project argument accepts a directory or a `tsconfig.json` path. Without it, the current directory is used. The default output is `index.scip`.

```sh
scip-tsrs index ./packages/service --output ./index.scip
scip-tsrs index ./tsconfig.json --project-root . --output ./index.scip
```

`scip` is an alias for `index`. Indexing loads the compiler program in-process and does not invoke `tsc`, emit JavaScript or declaration files, or modify the indexed sources. JavaScript inclusion follows compiler settings such as `allowJs`.

## Project references and monorepos

Run from the solution root to combine referenced projects into one standard SCIP index, including solutions with an empty `files` list. References are followed automatically and repeated projects are indexed once. Compiler programs are released between projects to bound their lifetimes.

```sh
scip-tsrs index ./tsconfig.json --project-root . --output ./index.scip
```

Use `--project-root` to choose the base for document paths, normally the repository or solution root. Its default is the config directory. Select a root that contains the projects you want represented in the index. Use `--no-project-references` when you intentionally want just one config's program.

This is producer behavior: SCIP carries the combined documents and symbol references. Consumers read an ordinary SCIP index and do not need a scip-tsrs-specific project-reference API.

## Options

| Option | Behavior |
| --- | --- |
| `--output PATH` | Write the SCIP index to this path. Output replacement is atomic; a failed indexing run preserves an existing output file. |
| `--project-root PATH` | Set the base directory for relative document paths. |
| `--no-project-references` | Restrict indexing to the selected config instead of traversing its project-reference graph. |
| `--infer-tsconfig` | Infer an in-memory configuration for a TypeScript or JavaScript project with no config file. No config is written. |
| `--max-file-byte-size N` | Limit emitted documents by source size. The default is 1,000,000 bytes; `0` disables the limit. Excluded sources remain available to the checker. |
| `--minimal` | Emit occurrences and bare symbol data for comparison. The default also emits metadata and relationships; enabling enrichment preserves occurrence identities, positions, and roles. |
| `--lib DIRECTORY` | Select an alternate TypeScript standard-library directory. By default the executable uses tsrs's bundled TypeScript 7.1 libraries. |

Use `scip-tsrs index --help` for the complete command-line reference.

## Inspect and diagnose

```sh
scip-tsrs inspect ./tsconfig.json --file src/example.ts
scip-tsrs diagnose ./tsconfig.json
scip-tsrs dump ./index.scip
```

`inspect` prints each identifier's checker symbol, declarations, and declaration parent chains for a selected file. `diagnose` runs compiler diagnostics separately from indexing. A successfully emitted index does not mean the project is free of compiler errors.

`dump` decodes a SCIP file and prints JSON lines for document paths and normalized occurrence tuples. It also validates native symbol syntax, range ordering, and position encoding. It is an occurrence-comparison aid; it does not dump all symbol metadata or relationships.
