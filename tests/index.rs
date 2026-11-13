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
    assert!(implements(
        symbol(&rich, "Widget#root()."),
        "Root#root().",
        true
    ));
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

#[test]
fn inherits_documentation_without_changing_occurrences() {
    let directory = tempfile::tempdir().unwrap();
    project(directory.path());
    fs::write(
        directory.path().join("api.ts"),
        include_str!("fixtures/documentation.ts"),
    )
    .unwrap();
    fs::write(directory.path().join("client.ts"), "export {};\n").unwrap();
    let rich = generate(directory.path(), "rich.scip", &[]);
    let minimal = generate(directory.path(), "minimal.scip", &["--minimal"]);
    assert_eq!(tuples(&rich), tuples(&minimal));
    for (suffix, expected) in [
        (
            "DerivedInterface#work().",
            "Interface method documentation.",
        ),
        (
            "GenericDerived#args.",
            "Generic interface property documentation.",
        ),
        (
            "Transitive#args.",
            "Generic interface property documentation.",
        ),
        ("OwnProperty#args.", "Own property documentation."),
        ("Child#work().", "Parent method documentation."),
        ("Child#value.", "Parent property documentation."),
        ("Child#make().", "Parent static method documentation."),
        ("Own#work().", "Own method documentation."),
        ("generic().[T]", "Documented type parameter."),
        ("generic().(value)", "Documented parameter."),
    ] {
        let info = symbol(&rich, suffix);
        assert!(
            info.documentation.iter().any(|doc| doc.contains(expected)),
            "{suffix}: expected {expected:?}, got {:?}",
            info.documentation
        );
    }
    assert!(
        !symbol(&rich, "Own#work().")
            .documentation
            .iter()
            .any(|doc| doc.contains("Parent method documentation."))
    );
    assert!(
        !symbol(&rich, "OwnProperty#args.")
            .documentation
            .iter()
            .any(|doc| doc.contains("Generic interface property documentation."))
    );
}

#[test]
fn inherits_documentation_for_external_symbols() {
    let directory = tempfile::tempdir().unwrap();
    project(directory.path());
    let dependency = directory.path().join("node_modules/external-docs");
    fs::create_dir_all(&dependency).unwrap();
    fs::write(
        dependency.join("package.json"),
        r#"{"name":"external-docs","version":"1.0.0","types":"index.d.ts"}"#,
    )
    .unwrap();
    fs::write(
        dependency.join("index.d.ts"),
        "export interface Base {\n/** External property documentation. */\nvalue: string;\n}\nexport interface Derived extends Base { value: string; }\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("api.ts"),
        "import type { Derived } from 'external-docs';\nexport function read(item: Derived) { return item.value; }\n",
    )
    .unwrap();
    fs::write(directory.path().join("client.ts"), "export {};\n").unwrap();
    let rich = generate(directory.path(), "rich.scip", &[]);
    let minimal = generate(directory.path(), "minimal.scip", &["--minimal"]);
    assert_eq!(tuples(&rich), tuples(&minimal));
    assert_eq!(rich.documents.len(), 2);
    let info = rich
        .external_symbols
        .iter()
        .find(|info| info.symbol.ends_with("Derived#value."))
        .expect("external derived property");
    assert_eq!(info.documentation, ["External property documentation."]);
}

#[test]
fn preserves_jsdoc_tags_examples_and_tag_only_comments_in_javascript() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    fs::write(
        path.join("package.json"),
        r#"{"name":"jsdoc-test","version":"1.0.0","type":"module"}"#,
    )
    .unwrap();
    fs::write(
        path.join("tsconfig.json"),
        r#"{"compilerOptions":{"allowJs":true,"checkJs":true,"types":[]},"files":["api.js"]}"#,
    )
    .unwrap();
    fs::write(
        path.join("api.js"),
        r#"const prefix = 'é😀';
/** Returns its argument.
 * @template T
 * @param {T} value - The value, including é😀.
 *   Continued parameter documentation.
 * @returns {T} The original value.
 * @throws {Error} If invalid.
 * @deprecated Use identity instead.
 * @example
 * ```js
 * echo('é😀');
 * ```
 * @example
 *     echo('indented');
 * @customNote Custom documentation survives.
 */
export function echo(value) { return value; }
/** @type {number} */
export const total = 1;
"#,
    )
    .unwrap();
    let rich = generate(path, "rich.scip", &[]);
    let minimal = generate(path, "minimal.scip", &["--minimal"]);
    assert_eq!(tuples(&rich), tuples(&minimal));
    let documentation = symbol(&rich, "echo().").documentation.join("\n\n");
    for expected in [
        "Returns its argument.",
        "**@template**\n\nT",
        "**@param**\n\n{T} value - The value, including é😀.\n  Continued parameter documentation.",
        "**@returns**\n\n{T} The original value.",
        "**@throws**\n\n{Error} If invalid.",
        "**@deprecated**\n\nUse identity instead.",
        "**@example**\n\n```js\necho('é😀');\n```",
        "**@example**\n\n    echo('indented');",
        "**@customNote**\n\nCustom documentation survives.",
    ] {
        assert!(
            documentation.contains(expected),
            "missing {expected:?} in {documentation:?}"
        );
    }
    assert_eq!(
        symbol(&rich, "total.").documentation,
        ["**@type**\n\n{number}"]
    );
}

#[test]
fn solution_root_follows_references_once_and_resolves_cross_project_symbols() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    fs::create_dir(path.join("lib")).unwrap();
    fs::create_dir(path.join("app")).unwrap();
    fs::write(
        path.join("package.json"),
        r#"{"name":"solution-test","version":"1.0.0"}"#,
    )
    .unwrap();
    fs::write(path.join("tsconfig.json"), r#"{"files":[],"references":[{"path":"./lib"},{"path":"./app"},{"path":"./lib/tsconfig.json"}]}"#).unwrap();
    fs::write(
        path.join("lib/tsconfig.json"),
        r#"{"compilerOptions":{"composite":true,"types":[]},"files":["api.ts"]}"#,
    )
    .unwrap();
    fs::write(
        path.join("lib/api.ts"),
        "export interface Contract { run(): number }\nexport function compute() { return 42; }\n",
    )
    .unwrap();
    fs::write(path.join("app/tsconfig.json"), r#"{"compilerOptions":{"composite":true,"types":[]},"files":["client.ts"],"references":[{"path":"../lib"}]}"#).unwrap();
    fs::write(path.join("app/client.ts"), "import { Contract, compute } from '../lib/api';\nexport class Service implements Contract { run() { return compute(); } }\n").unwrap();
    let index = generate(path, "index.scip", &[]);
    let mut files: Vec<_> = index
        .documents
        .iter()
        .map(|d| d.relative_path.as_str())
        .collect();
    files.sort();
    assert_eq!(files, ["app/client.ts", "lib/api.ts"]);
    let function = symbol(&index, "compute().");
    assert!(
        index
            .documents
            .iter()
            .find(|d| d.relative_path == "app/client.ts")
            .unwrap()
            .occurrences
            .iter()
            .any(|o| o.symbol == function.symbol && o.symbol_roles == 0)
    );
    assert!(implements(symbol(&index, "Service#"), "Contract#", false));
    assert!(
        !index
            .external_symbols
            .iter()
            .any(|s| s.symbol == function.symbol)
    );
    assert!(!path.join("lib/api.d.ts").exists());
    assert!(!path.join("app/client.js").exists());
    fs::write(path.join("index.scip"), "previous output").unwrap();
    fs::write(
        path.join("app/tsconfig.json"),
        r#"{"files":[],"references":[{"path":"../missing"}]}"#,
    )
    .unwrap();
    assert!(!run(path, &["index"]).status.success());
    assert_eq!(
        fs::read_to_string(path.join("index.scip")).unwrap(),
        "previous output"
    );
}

#[test]
fn infers_config_in_memory_for_typescript_and_javascript() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    fs::write(
        path.join("main.ts"),
        "export function example() { return 1; }\n",
    )
    .unwrap();
    fs::write(path.join("other.js"), "export const value = 2;\n").unwrap();
    let index = generate(path, "index.scip", &["--infer-tsconfig"]);
    assert_eq!(index.documents.len(), 2);
    assert!(!path.join("tsconfig.json").exists());
}
