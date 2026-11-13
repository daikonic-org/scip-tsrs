# Benchmarks and occurrence compatibility

These measurements compare released **scip-tsrs 0.2.1** with **scip-typescript 0.4.0** on `@sittir/typescript`. They were captured on 11 November 2026 UTC. The [raw measurements](benchmarks/2026-11-11.json) accompany this report.

## Method and scope

- ARM64 macOS host, 14 cores, 48 GiB RAM.
- Three fresh-process pairs, alternating the order: scip-typescript/scip-tsrs, scip-tsrs/scip-typescript, scip-typescript/scip-tsrs.
- Normal metadata-enabled output for both tools. scip-tsrs did not use `--minimal`.
- `/usr/bin/time -l` measured elapsed time, user/system CPU time, and peak RSS. Each run wrote an actual SCIP file outside the source checkout and exited successfully.
- The selected config had 135 root files, 1,175 program source files, and 1,236 captured read inputs. Both producers emitted 132 document paths. There were no project references in this package.
- Input fingerprints were checked before and after each sample and matched. The captured-input SHA-256 was `0e26a9c7dc9e8ea8cf8456a2ae29cafce08da950bfdf3b8e5ae7d9006022fa6f`.
- The host was heavily loaded: load averages were approximately 170–317. Filesystem caches were warm. These are a small observed sample, not isolated-machine, cold-cache, or confidence-interval results.

scip-typescript used TypeScript 5.9.3. scip-tsrs used its bundled TypeScript 7.1 libraries and the unpatched tsrs revision [`86de03ae428a956686af29177b50f0fdf4aee059`](https://github.com/oxc-project/tsrs/tree/86de03ae428a956686af29177b50f0fdf4aee059). The scip-tsrs executable source was [`0b054fd41b2e4a96b017db7fccc491eaaeaa10e2`](https://github.com/daikonic-org/scip-tsrs/commit/0b054fd41b2e4a96b017db7fccc491eaaeaa10e2).

## Producer results

| Metric | scip-typescript 0.4.0 | scip-tsrs 0.2.1 | Comparison |
| --- | ---: | ---: | --- |
| Median wall time | 30.78 s | 1.77 s | 17.4× faster |
| Median CPU time, user + system | 36.31 s | 1.72 s | 21.1× less CPU time |
| Median peak RSS | 2,328.9 MiB | 342.8 MiB | 85.3% lower |
| Peak RSS range | 1,642.1–2,386.4 MiB | 342.5–345.5 MiB | Observed range of three samples |
| SCIP file size | 29.68 MiB | 35.97 MiB | 21.2% larger |

| Pair | Order | scip-typescript wall | scip-tsrs wall | scip-typescript peak RSS | scip-tsrs peak RSS |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | scip-typescript, then scip-tsrs | 32.13 s | 1.77 s | 2,328.9 MiB | 342.8 MiB |
| 2 | scip-tsrs, then scip-typescript | 30.78 s | 1.95 s | 2,386.4 MiB | 345.5 MiB |
| 3 | scip-typescript, then scip-tsrs | 29.07 s | 1.49 s | 1,642.1 MiB | 342.5 MiB |

A separate scip-tsrs control using scip-typescript's TypeScript 5.9.3 library directory took **1.91 s**, with **338.9 MiB** peak RSS. This control is not included in the three-run medians. It shows the observed speed difference persisted when the library directory was shared.

These are producer-process measurements. They exclude building the indexer, importing SCIP into another database, embeddings, and long-running MCP or daemon memory. The implementations also emit different metadata and have different documentation coverage, so the outputs are not equivalent feature-for-feature. No paired Linux, cold-cache, or full-solution performance benchmark was conducted.

## Occurrence compatibility

Comparison uses sets of `(symbol, file, normalized range, role)` tuples, decoded through the standard SCIP reader. Recall is the fraction of scip-typescript tuples found exactly in scip-tsrs; precision is the fraction of scip-tsrs tuples found exactly in scip-typescript. These metrics do not compare documentation or relationships and do not establish complete compiler conformance.

| Library selection | Exact shared tuples / scip-typescript tuples | Recall | Precision | Definition recall |
| --- | ---: | ---: | ---: | ---: |
| Shared TypeScript 5.9.3 libraries | 250,371 / 250,990 | 99.753% | 99.813% | 99.925% |
| Default bundled TypeScript 7.1 libraries | 241,133 / 250,990 | 96.073% | 96.062% | 99.925% |

Every unmatched tuple was assigned a cause. Each cell below is **scip-typescript-only / scip-tsrs-only**. Two differing tuple identities can describe the same source occurrence; this table is not a count of missing source tokens.

| Cause | Shared libraries | Bundled libraries |
| --- | ---: | ---: |
| TypeScript package version in otherwise matching library symbols | 0 / 0 | 9,238 / 9,238 |
| Legacy ESNext library files absent from TS7 library mapping | 152 / 0 | 0 / 0 |
| Library declarations moved to newer standard files | 0 / 0 | 152 / 152 |
| Additional bundled library declarations | 0 / 0 | 0 / 25 |
| Local ordinals assigned in a different union-declaration order | 357 / 357 | 357 / 357 |
| Local ordinal shift after an index-signature lookup | 108 / 108 | 108 / 108 |
| Extra `process.env` index-signature resolution | 0 / 3 | 0 / 3 |
| Type-literal descriptor ordinal shift | 2 / 2 | 2 / 2 |
| **Total** | **619 / 470** | **9,857 / 9,885** |

Using the same libraries reduces version and library-layout differences; it does not make the two checkers identical. The [SCIP compatibility guide](scip.md) explains local identity and access-role limitations.

## Metadata observed in the comparison

| Output metadata | scip-typescript 0.4.0 | scip-tsrs 0.2.1 |
| --- | ---: | ---: |
| Explicit symbol-kind records | 0 | 56,356 |
| Structured signature records | 0 | 56,156 |
| Named local-symbol records | 0 | 9,731 |
| External-symbol records | 0 | 3,941 |
| Implementation relationships | 0 | 1,566 |

scip-typescript includes signatures in Markdown documentation, so zero structured signature records does not mean users receive no signature text. Both producers emit local occurrences; the named-local row counts added display-name metadata.

Direct JSDoc descriptions and tags worked in the control fixture, but inherited prose was absent for 239 symbols where scip-typescript supplied it. Most were generated properties sharing a single inherited comment. Neither producer emitted read/write access-role bits in this comparison. See [current limitations](scip.md#current-limitations).

An [unreleased documentation fix](parity.md) now recovers all 239 comments. Its separate before/after control retains 266/266 prose-bearing symbols and changes zero occurrence tuples. The follow-up resolved different dependency inputs, so its timings and tuple counts do not replace the release measurements above.

## Run a comparison on your project

Use pinned executables and place outputs outside the indexed checkout. The following example uses macOS `/usr/bin/time -l`; Linux uses `/usr/bin/time -v` instead.

```sh
benchmark_project=/absolute/path/to/project
benchmark_output=$(mktemp -d)

/usr/bin/time -l scip-typescript index \
  --cwd "$benchmark_project" --no-progress-bar \
  --output "$benchmark_output/scip-typescript.scip" "$benchmark_project"

/usr/bin/time -l scip-tsrs index "$benchmark_project" \
  --output "$benchmark_output/scip-tsrs.scip"

scip-tsrs dump "$benchmark_output/scip-typescript.scip" > "$benchmark_output/scip-typescript.jsonl"
scip-tsrs dump "$benchmark_output/scip-tsrs.scip" > "$benchmark_output/scip-tsrs.jsonl"
```

Repeat with fresh processes in alternating order and verify the source inputs did not change. For a common-library control, pass `--lib /path/to/typescript/lib` to scip-tsrs, using the same directory that scip-typescript resolves. Compare decoded occurrence tuples as sets; the tools may emit them in different orders.

These commands reproduce the method on a chosen checkout. The published numbers describe the captured working-tree snapshot identified above, not an arbitrary later checkout or npm package version.
