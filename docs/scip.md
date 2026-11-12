# SCIP output and compatibility

scip-tsrs emits the standard [SCIP protobuf](https://github.com/scip-code/scip/blob/main/scip.proto). The Rust binding dependency is `scip` 0.10.0; this is the binding version, not a separate scip-tsrs protocol. Consumers use the same index, document, occurrence, symbol-information, and relationship messages as other SCIP implementations.

This document describes the alpha `v0.2.1` release. See [CLI usage](usage.md) for project loading and [benchmarks](benchmarks.md) for measured occurrence compatibility with scip-typescript 0.4.0.

## Occurrences and symbol names

- Occurrences use typed SCIP ranges with zero-based lines and explicit UTF-16 code-unit columns. Unicode positions are tested.
- Definitions set `SymbolRole.Definition`; other emitted occurrences are references. Read/write access classification is a [current limitation](#current-limitations).
- Global symbols follow scip-typescript naming conventions, including package identity, declaration-parent descriptors, and `key<N>:` object-literal members.
- The TypeScript 7 checker and bundled standard libraries can choose different declarations, library package versions, and local ordinals from the TypeScript 5 checker used by scip-typescript 0.4.0. Compatible naming conventions do not guarantee identical tuples.

## Symbol information

The default output populates standard fields where the declaration provides the information:

| SCIP field | Information |
| --- | --- |
| `SymbolInformation.kind` | Explicit declaration kinds, including distinct classes and interfaces. |
| `display_name` | A source-facing name, including names for local symbols. |
| `signature_documentation` | A source signature with its language, separate from prose documentation. This preserves source text rather than synthesizing every inferred checker type. |
| `documentation` | Direct JSDoc descriptions and rendered tags. |
| `enclosing_symbol` | The containing symbol for declarations represented in the hierarchy. |
| `Occurrence.typed_enclosing_range` | The surrounding declaration's source extent where available, using the typed enclosing-range oneof. |
| `Index.external_symbols` | Metadata for referenced global symbols whose definitions are outside the emitted documents. |

The structured-signature and metadata design follows the standard fields used by [rust-analyzer's SCIP implementation](https://github.com/rust-lang/rust-analyzer/blob/2a362a41a8eb6e090f1a12d9b712df85af6f0c33/crates/rust-analyzer/src/cli/scip.rs). scip-typescript 0.4.0 instead places signatures in Markdown documentation and did not populate explicit kinds, structured signatures, local display names, external-symbol records, or relationships in the [measured package](benchmarks.md#metadata-observed-in-the-comparison).

## Local variables

Local symbols use SCIP's `local N` identity within a document. Their definitions and references share that identity, and symbol information supplies the display name and kind. Shadowed variables have separate identities.

The same `local N` can appear in multiple files. Consumers must key a local symbol by document and symbol together. Local ordinals can change with checker or traversal order; they are not durable project-wide IDs.

This follows SCIP's document-local identity model, also used by [scip-java's local-symbol cache](https://github.com/scip-code/scip-java/blob/3793579c53edb5c0a51c69315c07faca7376dc3e/scip-shared/src/main/java/org/scip_code/scip_java/shared/LocalSymbolsCache.java). Local occurrences already exist in scip-typescript; scip-tsrs adds their explicit names and kinds to symbol information.

## Interfaces, inheritance, and implementation relationships

Relationships are emitted from the implementing or derived symbol toward its base declaration:

| Source relationship | `is_implementation` | `is_reference` |
| --- | --- | --- |
| Class `extends` class | `true` | `false` |
| Class `implements` interface | `true` | `false` |
| Interface `extends` interface | `true` | `false` |
| Matching instance method or property to a base member | `true` | `true` |

SCIP uses `is_implementation` for implementation navigation across these type relationships; it has no separate `extends` and `implements` flags. Explicit `Class` and `Interface` kinds help consumers interpret the endpoints. The heritage reference remains in the source, where consumers can inspect the exact keyword when needed.

At the type level, a derived class should appear in an interface's implementation results without treating every use of that class as a reference to the interface. At the member level, `is_reference` also connects reference navigation across the implementation or override. These are the [standard SCIP relationship semantics](https://github.com/scip-code/scip/blob/main/scip.proto), rather than custom edge fields. The approach is informed by [scip-java's visitor](https://github.com/scip-code/scip-java/blob/3793579c53edb5c0a51c69315c07faca7376dc3e/scip-javac/src/main/java/org/scip_code/scip_java/javac/ScipVisitor.java).

Base types and their members are resolved through the checker, including members inherited by a base interface or class. Instance members are matched by name and declaration kind. This is not full TypeScript override compatibility checking, and it does not infer every structural implementation without an explicit heritage clause.

## JSDoc

Direct descriptions and tags are stored in `SymbolInformation.documentation`. Supported rendering includes parameter and return descriptions, templates, deprecation, fenced and indented examples, custom tags, Unicode text, and comments containing only tags. Checked JavaScript and TypeScript fixtures cover direct documentation.

Inherited comments and resolved JSDoc link targets remain incomplete. In the benchmark package, scip-typescript supplied inherited prose for 239 symbols that scip-tsrs did not: 238 generated properties sharing one upstream comment, and one inherited interface method. This is a known difference, not a claim of equivalent documentation coverage.

## Current limitations

scip-tsrs is alpha software. Pin exact releases and validate the fields your consumer depends on.

- **Access roles:** `ReadAccess` and `WriteAccess` bits are not emitted. Assignments and increments have reference occurrences but no write classification. Consumers must not interpret role `0` as a proven read. These are existing standard SCIP flags and require no protocol extension to support later.
- **Documentation inheritance:** Direct JSDoc works, but inherited comments and link-target resolution are incomplete.
- **Relationship completeness:** Member links use checker-resolved heritage and name/kind matching. Full override compatibility, complex mixins, and all implicit structural implementations are not covered.
- **Checker parity:** TypeScript 7 declaration choices, library layout, and ordinal allocation can differ from scip-typescript's TypeScript 5 checker. JSX-specific parity remains open.
- **Coverage of validation:** Release CI builds and tests Apple silicon macOS, Linux ARM64, and Linux x86-64. The published performance comparison covers one package on ARM64 macOS, not every language feature or workload.

Use `--minimal` to omit metadata and relationships when comparing occurrence identity. It does not supply missing access roles or change the checker into TypeScript 5.
