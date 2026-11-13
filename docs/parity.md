# Parity audit

This audit covers the **inherited-documentation change released in 0.2.2** on 13 November 2026. It separates measured gaps against a named producer from features merely available in the SCIP protocol. It does not claim complete parity across languages.

## Comparators

| Area | Comparator and evidence | Result or remaining work |
| --- | --- | --- |
| TypeScript symbol names, definitions and references | Actual scip-typescript 0.4.0 output, using TypeScript 5.9.3 | Matched fixture: 52/52 tuples. Package differences remain between the TypeScript 5 and TypeScript 7 checkers. |
| Inherited JSDoc and parameter comments | Actual scip-typescript 0.4.0 output | Confirmed gap, fixed in this change. All 266 package prose comments match exactly, including the 239 previously missing comments. |
| Kinds, structured signatures, containment, named locals and external metadata | [rust-analyzer's emitter at `2a362a41`](https://github.com/rust-lang/rust-analyzer/blob/2a362a41a8eb6e090f1a12d9b712df85af6f0c33/crates/rust-analyzer/src/cli/scip.rs) | Already emitted through the corresponding standard fields. This is a representation comparison, not a cross-language golden test. |
| Type and overridden-member relationships | [SCIP-Java's visitor at `3793579c`](https://github.com/scip-code/scip-java/blob/3793579c53edb5c0a51c69315c07faca7376dc3e/scip-javac/src/main/java/org/scip_code/scip_java/javac/ScipVisitor.java) | Existing direction and flags follow the same model: type implementation links; member implementation/reference links. Java uses compiler override checks; scip-tsrs still matches checker-resolved base members by name and kind. Full override correctness needs targeted validation. |
| Read/write occurrence roles | [SCIP protocol](https://github.com/scip-code/scip/blob/main/scip.proto) and inspection of the two emitters above | Optional protocol improvement, not a demonstrated gap against these producers. The inspected rust-analyzer and SCIP-Java occurrence emitters set definition/reference roles without read/write classification; neither did the measured scip-typescript output. |
| JSDoc link navigation, complex mixins and JSX completeness | No matched failure established in this audit | Coverage questions, not quantified parity deficits. Add comparator fixtures before claiming parity or scheduling a specific fix. |

SCIP has no separate `extends` and `implements` relationship flags. Its `is_implementation` and `is_reference` fields express navigation behavior; declaration kinds distinguish classes from interfaces. See [relationship semantics](scip.md#interfaces-inheritance-and-implementation-relationships).

## Documentation implementation

Existing direct descriptions and tags retain their renderer and precedence. If that produces no documentation, the indexer resolves the declaration symbol with `Checker::get_symbol_of_declaration_exported` and calls the public `tsrs_ls::get_symbol_documentation_comment` helper. The helper uses checker-aware inheritance and parameter lookup; the indexer does not duplicate those algorithms or start an LSP server.

All tsrs dependencies remain exactly `0.24.0`. The added helper introduces three registry crates, bringing the tsrs dependency closure from 21 to 24. External-symbol documentation is resolved before the checker and program are dropped. There are no CLI, SCIP-schema, symbol-name or occurrence-role changes.

Regression tests cover class methods, properties and static methods; generic and transitive interface properties; interface methods; explicit comment precedence; `@param`/`@template` descriptions; and inherited external-symbol documentation. Rich and `--minimal` outputs retain identical occurrence tuples. Existing direct-JSDoc and JavaScript tests continue to pass.

## Measured documentation and occurrences

The follow-up uses the captured `@sittir/typescript` package source from the earlier benchmark, with its **currently resolved dependencies**. Those dependencies have changed since 11 November. Both binaries in this control read the same stable inputs; these results do not replace the [published release benchmark](benchmarks.md).

| Check | Before | After |
| --- | ---: | ---: |
| Package prose-bearing symbols retained from scip-typescript | 27 / 266 | 266 / 266 |
| Exact prose text matches | 27 / 266 | 266 / 266 |
| Previously reported 239 missing comments recovered | 0 / 239 | 239 / 239 |
| Package occurrence tuples | 247,164 | 247,164 |
| Changed occurrence tuples between native binaries | — | **0** |

The matched documentation fixture has 52 exact shared tuples out of 52 for each producer and retains prose for all 16 symbols documented by scip-typescript. Fifteen prose records match verbatim; the generic function also retains the existing direct tag rendering, which adds its `@param` and `@template` text. The package's 266 prose records match verbatim after signatures are removed from scip-typescript's Markdown documentation.

A fresh common-library control shares 246,177 tuples out of 247,074 scip-typescript tuples (**99.637% recall**) and 246,987 native tuples (**99.672% precision**). Every unmatched tuple is grouped below. These differences are present independently of the documentation fix; both native binaries emit the same tuple set.

| Difference in this follow-up | scip-typescript only | scip-tsrs only |
| --- | ---: | ---: |
| Local identity/order | 714 | 714 |
| Legacy ESNext library mapping | 152 | 0 |
| Node imported/ambient declaration resolution at the same location | 23 | 23 |
| Type-literal descriptor ordinal | 5 | 5 |
| Contextual object-literal definition versus base property reference | 3 | 3 |
| Additional TypeScript 7 resolutions: 58 Node symbols, 4 library members and 3 index-signature locals | 0 | 65 |
| **Total** | **897** | **810** |

These groups describe the observed declaration/identity differences, not independently proven root causes for every checker decision. The additional Node resolutions also expose inferred member references where the older checker supplied no matching tuple. The comparison does not infer complete compiler conformance from the high match percentage.

## Before/after performance control

Three fresh-process pairs alternate order on ARM64 macOS with warm filesystem caches, using normal metadata-enabled output and bundled libraries. `/usr/bin/time -l` measures process wall time, CPU time and peak RSS. The host is shared with other development work; this small sample is suitable for finding a material regression, not claiming a speedup from the documentation change.

| Median | Before (`0045ed9`) | After |
| --- | ---: | ---: |
| Wall time | 1.49 s | 1.45 s |
| CPU time | 1.67 s | 1.68 s |
| Peak RSS | 399.6 MiB | 399.2 MiB |
| SCIP output size | 35.49 MiB | 35.56 MiB |
| Release executable size | 19.20 MiB | 19.33 MiB |

The native release target, including test and Clippy artifacts, occupied 501 MiB after local verification. No debug build was made. Input checks covered 135 roots, 1,077 TypeScript 5 program source files and 1,135 read inputs; their fingerprint stayed unchanged before and after every timed sample and the common-library comparison. Bundled-library native loading reported 1,213 program files. This input set differs from the original performance report's 1,175-file program.

The [raw samples and counts](benchmarks/2026-11-13-documentation.json) record the input fingerprint and binary hashes. These timings exclude builds, consumer imports, embeddings and daemon memory. They show no material producer regression in this sample.
