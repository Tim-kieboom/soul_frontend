use itertools::Itertools;
use soul_utils::{
    Span,
    soul_names::KeyWord,
    SoulPagePath,
};
use crate::{
    abstract_syntax_tree::{
        block::Block,
        enum_like::{Enum, Union},
        expression::{Expression, ExpressionKind},
        function::{Function, FunctionCall, FunctionSignature},
        objects::{Class, ClassMember, Field, Struct, Trait},
        soul_type::{GenericDeclare, GenericDeclareKind, SoulType},
        spanned::Spanned,
        syntax_display::{DisplayKind, SyntaxDisplay, gap_prefix, tree_prefix},
    },
    sementic_models::scope::NodeId,
};

/// A statement in the Soul language, wrapped with source location information.
pub type Statement = Spanned<StatementKind>;

/// The different kinds of statements that can appear in the language.
///
/// Each variant corresponds to a syntactic construct, ranging from expressions
/// to type definitions and control structures.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    EndFile,

    /// Imported paths
    Import(Vec<SoulPagePath>),

    /// A standalone expression.
    Expression(Expression),

    /// A variable declaration.
    Variable(Variable),
    /// An assignment to an existing variable.
    Assignment(Assignment),

    /// A function declaration (with body block).
    Function(Function),
    /// A scoped `use` block (soul version of rusts 'impl' with optional trait implementation).
    UseBlock(UseBlock),

    /// A class declaration.
    Class(Class),
    /// A struct declaration.
    Struct(Struct),
    /// A trait declaration.
    Trait(Trait),

    /// An enum declaration (c like enum).
    Enum(Enum),
    /// A union declaration (rust like enum).
    Union(Union),

    /// Marker for closing a block (used during parsing).
    CloseBlock,
}

/// An identifier (variable name, type name, etc.).
pub type Ident = Spanned<String>;
impl Ident {
    pub fn as_str(&self) -> &str {
        &self.node
    }
}

/// An assignment statement, e.g., `x = y + 1`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Assignment {
    /// The left-hand side expression (the variable being assigned to).
    pub left: Expression,
    /// The right-hand side expression (the value being assigned).
    pub right: Expression,
}

/// A variable declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    /// The name of the variable.
    pub name: Ident,
    /// The type of the variable.
    pub ty: SoulType,
    /// Optional initial value expression.
    pub initialize_value: Option<Expression>,

    pub node_id: Option<NodeId>,
}

/// A `use` block (similar to Rust's `impl` block).
///
/// Can optionally implement a trait for a type, or just add methods to a type.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UseBlock {
    /// The trait being implemented, if any.
    pub impl_trait: Option<SoulType>,
    /// The type this block is for.
    pub ty: SoulType,
    /// The block containing method definitions.
    pub block: Block,
}

impl Statement {
    pub fn new_expression(kind: ExpressionKind, span: Span) -> Self {
        Self::new(StatementKind::new_expression(kind, span), span)
    }

    pub fn from_expression(expression: Expression) -> Self {
        let span = expression.span;
        Self::new(StatementKind::Expression(expression), span)
    }

    pub fn from_function_call(function: Spanned<FunctionCall>) -> Self {
        Self::from_expression(Expression::with_atribute(
            ExpressionKind::FunctionCall(function.node),
            function.span,
            function.attributes,
        ))
    }

    pub fn new_block(block: Block, span: Span) -> Self {
        Self::new_expression(ExpressionKind::Block(block), span)
    }
}

impl StatementKind {
    pub fn new_expression(kind: ExpressionKind, span: Span) -> Self {
        Self::Expression(Expression::new(kind, span))
    }

    pub fn from_expression(expression: Expression) -> Self {
        Self::Expression(expression)
    }

    pub fn new_block(block: Block, span: Span) -> Self {
        Self::new_expression(ExpressionKind::Block(block), span)
    }

    pub fn get_variant_name(&self) -> &'static str {
        match self {
            StatementKind::EndFile => "EndFile",
            StatementKind::Enum(_) => "Enum",
            StatementKind::Union(_) => "Union",
            StatementKind::Class(_) => "Class",
            StatementKind::Trait(_) => "Trait",
            StatementKind::Struct(_) => "Struct",
            StatementKind::Import(_) => "Import",
            StatementKind::Variable(_) => "Variable",
            StatementKind::Function(_) => "Function",
            StatementKind::UseBlock(_) => "UseBlock",
            StatementKind::Expression(_) => "Expression",
            StatementKind::Assignment(_) => "Assignment",
            StatementKind::CloseBlock => "CloseBlock",
         }
    }
}

impl SyntaxDisplay for StatementKind {
    fn display(&self, kind: DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: DisplayKind, tab: usize, is_last: bool) {
        let prefix = tree_prefix(tab, is_last);
        match self {
            StatementKind::EndFile => (),
            StatementKind::Import(paths) => {
                sb.push_str(&prefix);
                sb.push_str("Import >> ");
                let prefix = tree_prefix(tab + 1, is_last);
                for path in paths {
                    sb.push('\n');
                    sb.push_str(&prefix);
                    sb.push_str("Path >> ");
                    sb.push_str(path.as_str());
                }
            }
            StatementKind::Expression(spanned) => {
                sb.push_str(&prefix);
                let tag = if matches!(spanned.node, ExpressionKind::Block(_)) {
                    "Block >> "
                } else {
                    "Expression >> "
                };
                sb.push_str(tag);
                spanned.node.inner_display(sb, kind, tab, is_last);
            }
            StatementKind::Variable(var) => {
                sb.push_str(&prefix);
                try_display_node_id(sb, kind, var.node_id);
                sb.push_str("Variable >> ");
                sb.push_str(var.name.as_str());
                sb.push_str(": ");
                var.ty.inner_display(sb, kind, tab, is_last);
                if let Some(val) = &var.initialize_value {
                    sb.push_str(" = ");
                    val.node.inner_display(sb, kind, tab, is_last);
                }
            }
            StatementKind::Assignment(assignment) => {
                sb.push_str(&prefix);
                sb.push_str("Assignment >> ");
                assignment.left.node.inner_display(sb, kind, tab, is_last);
                sb.push_str(" = ");
                assignment.right.node.inner_display(sb, kind, tab, is_last);
            }
            StatementKind::Function(function) => {
                sb.push_str(&prefix);
                try_display_node_id(sb, kind, function.node_id);
                sb.push_str("Function >> ");
                inner_display_function_declaration(
                    sb,
                    kind,
                    &function.signature.node,
                    tab,
                    is_last,
                );
                function.block.inner_display(sb, kind, tab, is_last);
            }
            StatementKind::UseBlock(use_block) => {
                sb.push_str(&prefix);
                sb.push_str("UseBlock >> ");
                use_block.ty.inner_display(sb, kind, tab, is_last);
                if let Some(impl_trait) = &use_block.impl_trait {
                    sb.push_str(" impl ");
                    impl_trait.inner_display(sb, kind, tab, is_last);
                }
                use_block.block.inner_display(sb, kind, tab, is_last);
            }
            StatementKind::Struct(r#struct) => {
                const USE_LAST: bool = true;
                sb.push_str(&prefix);
                try_display_node_id(sb, kind, r#struct.node_id);
                sb.push_str("Struct >> ");
                sb.push_str(r#struct.name.as_str());
                inner_display_generic_parameters(sb, kind, &r#struct.generics);
                inner_display_fields(sb, kind, &r#struct.fields, tab + 1, USE_LAST);
            }
            StatementKind::Trait(r#trait) => {
                const USE_LAST: bool = true;
                sb.push_str(&prefix);
                try_display_node_id(sb, kind, r#trait.node_id);
                sb.push_str("Trait >> ");
                sb.push_str(r#trait.signature.name.as_str());
                if !r#trait.signature.for_types.is_empty() {
                    let fors = r#trait
                        .signature
                        .for_types
                        .iter()
                        .map(|el| el.display(kind))
                        .join(", ");
                    sb.push_str(&format!(" {} [{}]", KeyWord::For.as_str(), fors));
                }
                if !r#trait.signature.implements.is_empty() {
                    let fors = r#trait
                        .signature
                        .implements
                        .iter()
                        .map(|el| el.display(kind))
                        .join(", ");
                    sb.push_str(&format!(" {} {}", KeyWord::Impl.as_str(), fors));
                }
                inner_display_methode_signatures(sb, kind, &r#trait.methods, tab + 1, USE_LAST);
            }
            StatementKind::Class(class) => {
                const USE_LAST: bool = true;
                sb.push_str(&prefix);
                try_display_node_id(sb, kind, class.node_id);
                sb.push_str("Class >> ");
                sb.push_str(class.name.as_str());
                inner_display_generic_parameters(sb, kind, &class.generics);
                inner_display_classchild(sb, kind, &class.members, tab + 1, USE_LAST);
            }
            StatementKind::Enum(_) => todo!(),
            StatementKind::Union(_) => todo!(),
            StatementKind::CloseBlock => sb.push_str(&prefix),
        }
    }
}

fn inner_display_function_declaration(
    sb: &mut String,
    kind: DisplayKind,
    signature: &FunctionSignature,
    tab: usize,
    is_last: bool,
) {
    sb.push_str(
        &signature
            .callee
            .as_ref()
            .map(|el| format!("{} ", el.node.extention_type.display(kind)))
            .unwrap_or_default(),
    );
    sb.push_str(signature.name.node.as_str());
    inner_display_generic_parameters(sb, kind, &signature.generics);
    sb.push('(');
    sb.push_str(
        &signature
            .callee
            .as_ref()
            .map(|el| format!("{}, ", el.node.this.display()))
            .unwrap_or_default(),
    );
    sb.push_str(
        &signature
            .parameters
            .types
            .iter()
            .map(|(name, el, node_id)| {
                format!(
                    "{}{}: {}",
                    node_id_display(*node_id, kind),
                    name.node,
                    el.display(kind)
                )
            })
            .join(", "),
    );
    sb.push(')');
    sb.push_str(": ");
    signature.return_type.inner_display(sb, kind, tab, is_last);
}

fn inner_display_generic_parameters(
    sb: &mut String,
    kind: DisplayKind,
    parameters: &Vec<GenericDeclare>,
) {
    if parameters.is_empty() {
        return;
    }

    sb.push('<');
    for parameter in parameters {
        match &parameter.kind {
            GenericDeclareKind::Lifetime(lifetime) => {
                sb.push('\'');
                sb.push_str(lifetime.as_str());
            }
            GenericDeclareKind::Expression {
                name,
                for_type,
                default,
            } => {
                sb.push_str(name.as_str());
                if let Some(ty) = &for_type {
                    sb.push_str(" impl ");
                    ty.inner_display(sb, kind, 0, false);
                }

                if let Some(expression) = &default {
                    sb.push_str(" = ");
                    expression.node.inner_display(sb, kind, 0, false);
                }
            }
            GenericDeclareKind::Type {
                name,
                traits,
                default,
            } => {
                sb.push_str(name.as_str());
                if !traits.is_empty() {
                    sb.push_str(": ");
                    sb.push_str(&traits.iter().map(|el| el.display(kind)).join(", "))
                }

                if let Some(ty) = &default {
                    sb.push_str(" = ");
                    ty.inner_display(sb, kind, 0, false);
                }
            }
        }

        try_display_node_id(sb, kind, parameter.node_id);
    }
    sb.push('>');
}

fn inner_display_classchild(
    sb: &mut String,
    kind: DisplayKind,
    kinds: &Vec<Spanned<ClassMember>>,
    tab: usize,
    use_last: bool,
) {
    fn get_tag(child: &Spanned<ClassMember>) -> &'static str {
        match &child.node {
            ClassMember::Field(_) => "Field >> ",
            ClassMember::Method(_) => "Methode >> ",
            ClassMember::ImplBlock(_) => "ImplBlock >> ",
        }
    }

    let lat_index = kinds.len() - 1;

    for (i, member) in kinds.iter().enumerate() {
        let is_last = use_last && lat_index == i;
        let prefix = tree_prefix(tab, is_last);

        sb.push('\n');
        sb.push_str(&prefix);
        try_display_node_id(sb, kind, member.node.try_get_node_id());
        sb.push_str(get_tag(member));
        match &member.node {
            ClassMember::Field(field) => inner_display_field(sb, kind, field, tab, is_last),
            ClassMember::Method(function) => {
                inner_display_methode(sb, kind, function, tab, is_last)
            }
            ClassMember::ImplBlock(_) => todo!(),
        }
    }
}

fn inner_display_methode_signatures(
    sb: &mut String,
    kind: DisplayKind,
    methods: &Vec<Spanned<FunctionSignature>>,
    tab: usize,
    use_last: bool,
) {
    if methods.is_empty() {
        return;
    }

    let lat_index = methods.len() - 1;

    for (i, methode) in methods.iter().enumerate() {
        let is_last = use_last && lat_index == i;

        let prefix = tree_prefix(tab, is_last);
        sb.push('\n');
        sb.push_str(&prefix);
        sb.push_str("Methode >> ");
        inner_display_methode_signature(sb, kind, &methode.node, tab, is_last);
    }
}

fn inner_display_fields(
    sb: &mut String,
    kind: DisplayKind,
    fields: &Vec<Spanned<Field>>,
    tab: usize,
    use_last: bool,
) {
    let last_index = fields.len() - 1;

    for (i, Spanned { node: field, .. }) in fields.iter().enumerate() {
        let is_last = use_last && last_index == i;

        let prefix = tree_prefix(tab, is_last);
        sb.push('\n');
        sb.push_str(&prefix);
        try_display_node_id(sb, kind, field.node_id);
        sb.push_str("Field >> ");
        inner_display_field(sb, kind, field, tab, is_last);
        if last_index == i {
            sb.push('\n');
            sb.push_str(&gap_prefix(tab));
        }
    }
}

fn inner_display_methode(
    sb: &mut String,
    kind: DisplayKind,
    methode: &Function,
    tab: usize,
    is_last: bool,
) {
    inner_display_methode_signature(sb, kind, &methode.signature.node, tab, is_last);
    methode.block.inner_display(sb, kind, tab + 1, is_last);
}

fn inner_display_methode_signature(
    sb: &mut String,
    kind: DisplayKind,
    methode: &FunctionSignature,
    tab: usize,
    is_last: bool,
) {
    inner_display_function_declaration(sb, kind, &methode, tab, is_last);
}

fn inner_display_field(
    sb: &mut String,
    kind: DisplayKind,
    field: &Field,
    tab: usize,
    is_last: bool,
) {
    sb.push_str(field.name.as_str());
    sb.push_str(": ");
    field.ty.inner_display(sb, kind, tab, is_last);
    sb.push(' ');
    field.vis.inner_display(sb);
    if let Some(default) = &field.default_value {
        sb.push_str(" = ");
        default.node.inner_display(sb, kind, tab, is_last);
    }
}

pub(crate) fn try_display_many_node_ids(
    sb: &mut String,
    kind: DisplayKind,
    node_ids: &Vec<NodeId>,
) {
    if node_ids.is_empty() || kind != DisplayKind::NameResolver {
        return;
    }

    sb.push_str("\"|");
    for node_id in node_ids {
        sb.push_str(&node_id.display());
        sb.push(',');
    }
    sb.push_str("|\"");
}

pub(crate) fn node_id_display(node_id: Option<NodeId>, kind: DisplayKind) -> String {
    if kind != DisplayKind::NameResolver {
        return String::default();
    }

    node_id
        .map(|el| format!("\"|{}|\"", el.display()))
        .unwrap_or_default()
}

pub(crate) fn try_display_node_id(sb: &mut String, kind: DisplayKind, node_id: Option<NodeId>) {
    sb.push_str(&node_id_display(node_id, kind));
}
