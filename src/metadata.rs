use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use scip::types::{Document, Relationship, Signature, SymbolInformation, symbol_information};
use tsrs_ast::{self as ast, Kind, ModifierFlags, Node, SourceFile};
use tsrs_core::P;
use tsrs_scanner::{get_text_of_jsdoc_comment, get_text_of_node, get_token_pos_of_node};

use crate::{node_range, scip_io, source_range};

#[derive(Default)]
pub struct IndexMetadata {
    defined: HashSet<String>,
    references: BTreeMap<String, P<Node>>,
}

impl IndexMetadata {
    fn define(&mut self, name: &str) {
        if !name.starts_with("local ") {
            self.defined.insert(name.to_owned());
            self.references.remove(name);
        }
    }

    pub fn reference(&mut self, name: &str, decl: P<Node>) {
        if !name.starts_with("local ") && !self.defined.contains(name) {
            self.references.entry(name.to_owned()).or_insert(decl);
        }
    }

    pub fn into_external_symbols(self) -> Vec<SymbolInformation> {
        self.references
            .into_iter()
            .map(|(name, decl)| symbol_information(name, decl, String::new()))
            .collect()
    }
}

pub struct FileMetadata<'a> {
    pub index: &'a mut IndexMetadata,
    seen: HashSet<String>,
    definitions: Vec<(usize, P<Node>)>,
    bases: HashMap<P<Node>, Vec<Base>>,
}

struct Base {
    symbol: String,
    members: Vec<(String, P<Node>)>,
}

impl<'a> FileMetadata<'a> {
    pub fn new(index: &'a mut IndexMetadata) -> Self {
        Self {
            index,
            seen: HashSet::new(),
            definitions: Vec::new(),
            bases: HashMap::new(),
        }
    }

    pub fn define(
        &mut self,
        doc: &mut Document,
        name: &str,
        decl: P<Node>,
        enclosing_symbol: String,
    ) {
        if self.seen.insert(name.to_owned()) {
            self.definitions.push((doc.symbols.len(), decl));
            doc.symbols
                .push(symbol_information(name.to_owned(), decl, enclosing_symbol));
            self.index.define(name);
        }
    }

    pub fn record_base(
        &mut self,
        owner: P<Node>,
        symbol: &str,
        decl: P<Node>,
        members: Vec<(String, P<Node>)>,
    ) {
        if !symbol.starts_with("local ") && is_named_type(decl) {
            self.bases.entry(owner).or_default().push(Base {
                symbol: symbol.to_owned(),
                members,
            });
        }
    }

    pub fn finish(self, doc: &mut Document) {
        for (index, decl) in self.definitions {
            let info = &mut doc.symbols[index];
            let mut related = BTreeSet::new();
            if let Some(bases) = self.bases.get(&decl) {
                for base in bases {
                    add_relationship(info, &mut related, base.symbol.clone(), false);
                }
            } else if let Some((owner, member_name, kind)) = instance_member(decl)
                && let Some(bases) = self.bases.get(&owner)
            {
                for base in bases {
                    for (name, member) in &base.members {
                        let member = *member;
                        if !instance_member(member).is_some_and(|(_, name, member_kind)| {
                            name == member_name && member_kind == kind
                        }) {
                            continue;
                        }
                        self.index.reference(name, member);
                        add_relationship(info, &mut related, name.clone(), true);
                    }
                }
            }
        }
    }
}

fn add_relationship(
    info: &mut SymbolInformation,
    seen: &mut BTreeSet<String>,
    symbol: String,
    is_reference: bool,
) {
    if symbol != info.symbol && seen.insert(symbol.clone()) {
        info.relationships.push(Relationship {
            symbol,
            is_implementation: true,
            is_reference,
            ..Relationship::default()
        });
    }
}

fn is_named_type(node: P<Node>) -> bool {
    matches!(
        node.kind(),
        Kind::ClassDeclaration | Kind::InterfaceDeclaration
    )
}

// The token identifies the source-level heritage clause. SCIP uses one bit for
// navigation across both clause forms. Explicit kinds let consumers distinguish
// classes from interfaces without guessing from the symbol descriptor.
pub fn heritage_owner(node: P<Node>) -> Option<P<Node>> {
    let parent = node.parent()?;
    let name = match parent.kind() {
        Kind::PropertyAccessExpression if parent.name() == Some(node) => parent,
        Kind::QualifiedName if parent.as_qualified_name().right() == node => parent,
        _ => node,
    };
    let heritage_type = name.parent()?;
    let target = match heritage_type.kind() {
        Kind::ExpressionWithTypeArguments => heritage_type
            .as_expression_with_type_arguments()
            .expression(),
        Kind::TypeReference => heritage_type.as_type_reference_node().type_name(),
        _ => return None,
    };
    if target != name {
        return None;
    }
    let clause = heritage_type
        .parent()
        .filter(|p| p.kind() == Kind::HeritageClause)?;
    let owner = clause.parent()?;
    match (clause.as_heritage_clause().token(), owner.kind()) {
        (Kind::ExtendsKeyword, Kind::ClassDeclaration | Kind::InterfaceDeclaration)
        | (Kind::ImplementsKeyword, Kind::ClassDeclaration) => Some(owner),
        _ => None,
    }
}

pub(crate) fn instance_member(
    node: P<Node>,
) -> Option<(P<Node>, String, symbol_information::Kind)> {
    let kind = match node.kind() {
        Kind::MethodDeclaration | Kind::MethodSignature => symbol_information::Kind::Method,
        Kind::PropertyDeclaration | Kind::PropertySignature => symbol_information::Kind::Property,
        _ => return None,
    };
    let owner = node.parent().filter(|&owner| is_named_type(owner))?;
    if ast::has_syntactic_modifier(node, ModifierFlags::Static) {
        return None;
    }
    Some((
        owner,
        ast::try_get_text_of_property_name(node.name()?)?,
        kind,
    ))
}

pub fn enclosing_range(file: P<SourceFile>, decl: P<Node>) -> Option<scip_io::Range> {
    if decl.kind() == Kind::SourceFile {
        return Some(source_range(file, 0, file.text().len() as u32));
    }
    let scope = match decl.kind() {
        Kind::VariableDeclaration if decl.initializer().is_some_and(ast::is_function_like) => decl,
        Kind::FunctionDeclaration
        | Kind::MethodDeclaration
        | Kind::Constructor
        | Kind::GetAccessor
        | Kind::SetAccessor
        | Kind::ClassDeclaration
        | Kind::InterfaceDeclaration
        | Kind::EnumDeclaration
        | Kind::TypeAliasDeclaration => decl,
        _ => return None,
    };
    (!scope.pos().is_synthetic() && !scope.end().is_synthetic()).then(|| node_range(file, scope))
}

pub fn is_enclosing_symbol(node: P<Node>) -> bool {
    matches!(
        node.kind(),
        Kind::SourceFile
            | Kind::ModuleDeclaration
            | Kind::ClassDeclaration
            | Kind::InterfaceDeclaration
            | Kind::EnumDeclaration
            | Kind::TypeAliasDeclaration
            | Kind::FunctionDeclaration
            | Kind::MethodDeclaration
            | Kind::MethodSignature
            | Kind::Constructor
            | Kind::GetAccessor
            | Kind::SetAccessor
    )
}

fn symbol_information(
    symbol: String,
    decl: P<Node>,
    enclosing_symbol: String,
) -> SymbolInformation {
    let file = ast::get_source_file_of_node(decl);
    let display_name = match decl.kind() {
        Kind::Constructor => "constructor".into(),
        Kind::SourceFile => Path::new(decl.as_source_file().file_name())
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned(),
        _ => decl
            .name()
            .filter(|name| !name.pos().is_synthetic() && !name.end().is_synthetic())
            .map(|name| {
                ast::try_get_text_of_property_name(name).unwrap_or_else(|| get_text_of_node(name))
            })
            .unwrap_or_default(),
    };
    let signature_documentation =
        file.and_then(|file| signature(decl, file))
            .map(|text| Signature {
                language: "TypeScript".into(),
                text,
                ..Signature::default()
            });
    SymbolInformation {
        symbol,
        display_name,
        kind: symbol_kind(decl.kind()).into(),
        enclosing_symbol,
        documentation: file
            .map(|file| documentation(decl, file))
            .unwrap_or_default(),
        signature_documentation: signature_documentation.into(),
        ..SymbolInformation::default()
    }
}

fn documentation(decl: P<Node>, file: P<SourceFile>) -> Vec<String> {
    let owner = if decl.kind() == Kind::VariableDeclaration {
        decl.parent()
            .and_then(|list| list.parent())
            .filter(|owner| owner.kind() == Kind::VariableStatement)
            .unwrap_or(decl)
    } else {
        decl
    };
    owner
        .jsdoc(Some(file.get()))
        .iter()
        .filter(|doc| doc.kind() == Kind::JSDoc)
        .map(|doc| {
            let jsdoc = doc.as_jsdoc();
            let mut sections = vec![get_text_of_jsdoc_comment(Some(jsdoc.comment()))];
            if let Some(tags) = jsdoc.tags() {
                for tag in tags.nodes() {
                    let Some(start) = tag.pos().source_offset() else {
                        continue;
                    };
                    let Some(end) = tag.end().source_offset() else {
                        continue;
                    };
                    let Some(text) = file.text().get(start as usize..end as usize) else {
                        continue;
                    };
                    sections.push(render_jsdoc_tag(text));
                }
            }
            sections
                .into_iter()
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .filter(|text| !text.is_empty())
        .collect()
}

/// Keep the compiler's tag boundaries and source content, including custom
/// tags, while removing only the comment margin. Examples retain indentation.
fn render_jsdoc_tag(text: &str) -> String {
    let text = text
        .lines()
        .map(|line| {
            let margin = line.trim_start_matches([' ', '\t']);
            margin
                .strip_prefix('*')
                .map(|line| line.strip_prefix(' ').unwrap_or(line))
                .unwrap_or(line)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let text = text.trim();
    let split = text.find(char::is_whitespace).unwrap_or(text.len());
    let (name, rest) = text.split_at(split);
    let rest = rest
        .strip_prefix('\n')
        .unwrap_or_else(|| rest.trim_start_matches([' ', '\t']));
    if rest.is_empty() {
        format!("**{name}**")
    } else {
        format!("**{name}**\n\n{rest}")
    }
}

fn signature(decl: P<Node>, file: P<SourceFile>) -> Option<String> {
    let start = decl.pos().source_offset()?;
    let end = decl.end().source_offset()?;
    if start >= end || decl.kind() == Kind::SourceFile {
        return None;
    }
    let text = file.text();
    let start = get_token_pos_of_node(decl, file, false) as usize;
    let end = if let Some(body) = decl.body() {
        get_token_pos_of_node(body, file, false) as usize
    } else if matches!(
        decl.kind(),
        Kind::ClassDeclaration | Kind::InterfaceDeclaration | Kind::EnumDeclaration
    ) {
        decl.member_list()?.pos().source_offset()? as usize
    } else if matches!(
        decl.kind(),
        Kind::VariableDeclaration | Kind::PropertyDeclaration | Kind::Parameter
    ) && let Some(initializer) = decl.initializer()
    {
        get_token_pos_of_node(initializer, file, false) as usize
    } else {
        end as usize
    };
    let text = text
        .get(start..end)?
        .trim_end()
        .trim_end_matches(['{', '='])
        .trim_end();
    (!text.is_empty() && text.len() <= 4096).then(|| text.to_owned())
}

fn symbol_kind(kind: Kind) -> symbol_information::Kind {
    use symbol_information::Kind as Scip;
    match kind {
        Kind::SourceFile => Scip::File,
        Kind::ClassDeclaration | Kind::ClassExpression => Scip::Class,
        Kind::InterfaceDeclaration => Scip::Interface,
        Kind::EnumDeclaration => Scip::Enum,
        Kind::EnumMember => Scip::EnumMember,
        Kind::Constructor => Scip::Constructor,
        Kind::FunctionDeclaration => Scip::Function,
        Kind::MethodDeclaration | Kind::MethodSignature => Scip::Method,
        Kind::PropertyDeclaration
        | Kind::PropertySignature
        | Kind::PropertyAssignment
        | Kind::ShorthandPropertyAssignment => Scip::Property,
        Kind::GetAccessor => Scip::Getter,
        Kind::SetAccessor => Scip::Setter,
        Kind::VariableDeclaration | Kind::BindingElement => Scip::Variable,
        Kind::Parameter => Scip::Parameter,
        Kind::TypeParameter => Scip::TypeParameter,
        Kind::TypeAliasDeclaration => Scip::TypeAlias,
        Kind::ModuleDeclaration => Scip::Namespace,
        _ => Scip::UnspecifiedKind,
    }
}
