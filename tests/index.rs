use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use protobuf::Message;
use scip::types::occurrence::Typed_range;
use scip::types::symbol_information::Kind;
use scip::types::{Index, PositionEncoding, SymbolInformation, SymbolRole};

const API: &str = "\
export interface Root { root(): string }
/** Contract documentation. */
export interface Contract extends Root { run(): number }
export class Parent { run(): number { return 1; } static same() { return 3; } }
";
const CLIENT: &str = "\
import { Parent } from './api.js';
import * as api from './api.js';
export class Widget extends Parent implements api.Contract {
  run() { return 2; }
  root() { return 'root'; }
  same() { return 4; }
}
/** Computes a value. */
export function one() { const label = 'é😀'; const count = new Map().size; return count; }
export function two() { const count = 9; return count; }
";

fn project(path: &Path) {
    fs::write(
        path.join("package.json"),
        r#"{"name":"scip-test","version":"1.0.0","type":"module"}"#,
    )
    .unwrap();
    fs::write(path.join("tsconfig.json"), r#"{"compilerOptions":{"target":"ES2022","module":"NodeNext","strict":true,"types":[]},"files":["api.ts","client.ts"]}"#).unwrap();
    fs::write(path.join("api.ts"), API).unwrap();
    fs::write(path.join("client.ts"), CLIENT).unwrap();
}

fn run(path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scip-tsrs"))
        .current_dir(path)
        .args(args)
        .output()
        .unwrap()
}

fn generate(path: &Path, name: &str, more: &[&str]) -> Index {
    let mut args = vec!["index", ".", "--output", name];
    args.extend_from_slice(more);
    let output = run(path, &args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Index::parse_from_bytes(&fs::read(path.join(name)).unwrap()).unwrap()
}

fn tuples(index: &Index) -> Vec<(&str, &str, &Option<Typed_range>, i32)> {
    index
        .documents
        .iter()
        .flat_map(|doc| {
            doc.occurrences.iter().map(|occurrence| {
                (
                    doc.relative_path.as_str(),
                    occurrence.symbol.as_str(),
                    &occurrence.typed_range,
                    occurrence.symbol_roles,
                )
            })
        })
        .collect()
}

fn symbol<'a>(index: &'a Index, suffix: &str) -> &'a SymbolInformation {
    index
        .documents
        .iter()
        .flat_map(|doc| &doc.symbols)
        .find(|info| info.symbol.ends_with(suffix))
        .unwrap_or_else(|| panic!("missing symbol {suffix}"))
}

fn implements(info: &SymbolInformation, suffix: &str, reference: bool) -> bool {
    info.relationships.iter().any(|rel| {
        rel.symbol.ends_with(suffix) && rel.is_implementation && rel.is_reference == reference
    })
}

#[test]
fn enriches_types_members_and_scoped_locals_without_changing_tuples() {
    let directory = tempfile::tempdir().unwrap();
    project(directory.path());
    let rich = generate(directory.path(), "rich.scip", &[]);
    let minimal = generate(directory.path(), "minimal.scip", &["--minimal"]);
    assert_eq!(tuples(&rich), tuples(&minimal));
    assert!(
        rich.documents.iter().all(|doc| doc.position_encoding
            == PositionEncoding::UTF16CodeUnitOffsetFromLineStart.into())
    );
    assert!(
        rich.documents
            .iter()
            .flat_map(|doc| &doc.occurrences)
            .all(|occ| occ.typed_range.is_some() && occ.range.is_empty())
    );

    let widget = symbol(&rich, "Widget#");
    let contract = symbol(&rich, "Contract#");
    let parent = symbol(&rich, "Parent#");
    assert_eq!(widget.kind, Kind::Class.into());
    assert_eq!(parent.kind, Kind::Class.into());
    assert_eq!(contract.kind, Kind::Interface.into());
    assert_eq!(contract.documentation, ["Contract documentation."]);
    assert!(
        contract
            .signature_documentation
            .text
            .contains("interface Contract extends Root")
    );
    assert!(implements(widget, "Parent#", false));
    assert!(implements(widget, "Contract#", false));
    assert!(implements(contract, "Root#", false));
    let method = symbol(&rich, "Widget#run().");
    assert!(implements(method, "Contract#run().", true));
    assert!(implements(method, "Parent#run().", true));
    assert!(symbol(&rich, "Widget#same().").relationships.is_empty());
    assert!(
        rich.external_symbols
            .iter()
            .any(|info| info.symbol.contains("/Map"))
    );
    assert!(
        rich.external_symbols
            .iter()
            .all(|info| !info.symbol.starts_with("local "))
    );

    let doc = rich
        .documents
        .iter()
        .find(|doc| doc.relative_path == "client.ts")
        .unwrap();
    let locals: Vec<_> = doc
        .symbols
        .iter()
        .filter(|info| info.display_name == "count")
        .collect();
    assert_eq!(locals.len(), 2);
    assert_ne!(locals[0].symbol, locals[1].symbol);
    let lines: Vec<_> = CLIENT.lines().collect();
    for (local, (line_no, line)) in locals.into_iter().zip(
        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains("const count")),
    ) {
        assert!(local.symbol.starts_with("local "));
        assert_eq!(local.kind, Kind::Variable.into());
        assert!(
            local.enclosing_symbol.ends_with("one().")
                || local.enclosing_symbol.ends_with("two().")
        );
        let occurrences: Vec<_> = doc
            .occurrences
            .iter()
            .filter(|occ| occ.symbol == local.symbol)
            .collect();
        assert_eq!(occurrences.len(), 2);
        for (occ, byte_offset) in occurrences
            .into_iter()
            .zip(line.match_indices("count").map(|(offset, _)| offset))
        {
            let Some(Typed_range::SingleLineRange(range)) = &occ.typed_range else {
                panic!("expected single-line local")
            };
            assert_eq!(range.line, line_no as i32);
            assert_eq!(
                range.start_character,
                line[..byte_offset].encode_utf16().count() as i32
            );
        }
    }
    assert!(doc.occurrences.iter().any(|occ| occ.symbol == widget.symbol
        && occ.symbol_roles == SymbolRole::Definition as i32
        && occ.typed_enclosing_range.is_some()));
}

#[test]
fn preserves_existing_output_when_indexing_fails() {
    let directory = tempfile::tempdir().unwrap();
    project(directory.path());
    let out = directory.path().join("index.scip");
    fs::write(&out, b"existing index").unwrap();
    let result = run(directory.path(), &["index", "--max-file-byte-size", "1"]);
    assert!(!result.status.success());
    assert_eq!(fs::read(&out).unwrap(), b"existing index");
    let result = run(directory.path(), &["index", "--output", "client.ts"]);
    assert!(!result.status.success());
    assert_eq!(
        fs::read_to_string(directory.path().join("client.ts")).unwrap(),
        CLIENT
    );
}
