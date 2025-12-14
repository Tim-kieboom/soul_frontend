use itertools::Itertools;

use crate::{
    abstract_syntax_tree::{
        block::Block,
        enum_like::{Enum, Union},
        expression::{Expression, ExpressionKind},
        function::{Function, FunctionCall, FunctionSignature},
        objects::{Class, ClassChild, Field, Struct, Trait},
        soul_type::{GenericDeclare, SoulType},
        spanned::Spanned,
        syntax_display::{SyntaxDisplay, gap_prefix, tree_prefix},
    },
    error::Span,
    sementic_models::scope::NodeId,
    soul_names::KeyWord,
    soul_page_path::SoulPagePath,
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
pub type Ident = String;

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
}

impl SyntaxDisplay for StatementKind {
    fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, tab: usize, is_last: bool) {
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
                sb.push_str("Expression >> ");
                spanned.node.inner_display(sb, tab, is_last);
            }
            StatementKind::Variable(var) => {
                sb.push_str(&prefix);
                if let Some(id) = var.node_id {
                    sb.push_str("NodeId(");
                    sb.push_str(&id.display());
                    sb.push_str(") ");
                }
                sb.push_str("Variable >> ");
                sb.push_str(&var.name);
                sb.push_str(": ");
                var.ty.inner_display(sb, tab, is_last);
                if let Some(val) = &var.initialize_value {
                    sb.push_str(" = ");
                    val.node.inner_display(sb, tab, is_last);
                }
            }
            StatementKind::Assignment(assignment) => {
                sb.push_str(&prefix);
                sb.push_str("Assignment >> ");
                assignment.left.node.inner_display(sb, tab, is_last);
                sb.push_str(" = ");
                assignment.right.node.inner_display(sb, tab, is_last);
            }
            StatementKind::Function(function) => {
                sb.push_str(&prefix);
                if let Some(id) = function.node_id {
                    sb.push_str("NodeId(");
                    sb.push_str(&id.display());
                    sb.push_str(") ");
                }
                sb.push_str("Function >> ");
                inner_display_function_declaration(sb, &function.signature, tab, is_last);
                function.block.inner_display(sb, tab, is_last);
            }
            StatementKind::UseBlock(use_block) => {
                sb.push_str(&prefix);
                sb.push_str("UseBlock >> ");
                use_block.ty.inner_display(sb, tab, is_last);
                if let Some(impl_trait) = &use_block.impl_trait {
                    sb.push_str(" impl ");
                    impl_trait.inner_display(sb, tab, is_last);
                }
                use_block.block.inner_display(sb, tab, is_last);
            }
            StatementKind::Struct(_struct) => {
                const USE_LAST: bool = true;
                sb.push_str(&prefix);
                if let Some(id) = _struct.node_id {
                    sb.push_str("NodeId(");
                    sb.push_str(&id.display());
                    sb.push_str(") ");
                }
                sb.push_str("Struct >> ");
                sb.push_str(&_struct.name);
                inner_display_generic_parameters(sb, &_struct.generics);
                inner_display_fields(sb, &_struct.fields, tab + 1, USE_LAST);
            }
            StatementKind::Trait(_trait) => {
                const USE_LAST: bool = true;
                sb.push_str(&prefix);
                if let Some(id) = _trait.node_id {
                    sb.push_str("NodeId(");
                    sb.push_str(&id.display());
                    sb.push_str(") ");
                }
                sb.push_str("Trait >> ");
                sb.push_str(&_trait.signature.name);
                if !_trait.signature.for_types.is_empty() {
                    let fors = _trait
                        .signature
                        .for_types
                        .iter()
                        .map(|el| el.display())
                        .join(", ");
                    sb.push_str(&format!(" {} [{}]", KeyWord::For.as_str(), fors));
                }
                if !_trait.signature.implements.is_empty() {
                    let fors = _trait
                        .signature
                        .implements
                        .iter()
                        .map(|el| el.display())
                        .join(", ");
                    sb.push_str(&format!(" {} {}", KeyWord::Impl.as_str(), fors));
                }
                inner_display_methode_signatures(sb, &_trait.methods, tab + 1, USE_LAST);
            }
            StatementKind::Class(class) => {
                const USE_LAST: bool = true;
                sb.push_str(&prefix);
                if let Some(id) = class.node_id {
                    sb.push_str("NodeId(");
                    sb.push_str(&id.display());
                    sb.push_str(") ");
                }
                sb.push_str("Class >> ");
                sb.push_str(&class.name);
                inner_display_generic_parameters(sb, &class.generics);
                inner_display_classchild(sb, &class.members, tab + 1, USE_LAST);
            }
            StatementKind::Enum(_) => todo!(),
            StatementKind::Union(_) => todo!(),
            StatementKind::CloseBlock => sb.push_str(&prefix),
        }
    }
}

fn inner_display_function_declaration(
    sb: &mut String,
    signature: &FunctionSignature,
    tab: usize,
    is_last: bool,
) {
    sb.push_str(
        &signature
            .callee
            .as_ref()
            .map(|el| format!("{} ", el.node.extention_type.display()))
            .unwrap_or_default(),
    );
    sb.push_str(&signature.name);
    inner_display_generic_parameters(sb, &signature.generics);
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
            .map(|(name, el, node_id)| format!("{}{}: {}", node_id.map(|el| format!("|NodeId({})|", el.display())).unwrap_or(String::new()), name, el.display()))
            .join(", "),
    );
    sb.push(')');
    sb.push_str(": ");
    signature.return_type.inner_display(sb, tab, is_last);
}

fn inner_display_generic_parameters(sb: &mut String, parameters: &Vec<GenericDeclare>) {
    if parameters.is_empty() {
        return;
    }

    sb.push('<');
    for parameter in parameters {
        match parameter {
            GenericDeclare::Lifetime(lifetime) => {
                sb.push('\'');
                sb.push_str(lifetime);
            }
            GenericDeclare::Expression {
                name,
                for_type,
                default,
            } => {
                sb.push_str(&name);
                if let Some(ty) = &for_type {
                    sb.push_str(" impl ");
                    ty.inner_display(sb, 0, false);
                }

                if let Some(expression) = &default {
                    sb.push_str(" = ");
                    expression.node.inner_display(sb, 0, false);
                }
            }
            GenericDeclare::Type {
                name,
                traits,
                default,
            } => {
                sb.push_str(&name);
                if !traits.is_empty() {
                    sb.push_str(": ");
                    sb.push_str(&traits.iter().map(|el| el.display()).join(", "))
                }

                if let Some(ty) = &default {
                    sb.push_str(" = ");
                    ty.inner_display(sb, 0, false);
                }
            }
        }
    }
    sb.push('>');
}

fn inner_display_classchild(
    sb: &mut String,
    kinds: &Vec<Spanned<ClassChild>>,
    tab: usize,
    use_last: bool,
) {
    fn get_tag(child: &Spanned<ClassChild>) -> &'static str {
        match &child.node {
            ClassChild::Field(_) => "Field >> ",
            ClassChild::Method(_) => "Methode >> ",
            ClassChild::ImplBlock(_) => "ImplBlock >> ",
        }
    }

    let lat_index = kinds.len() - 1;

    for (i, child) in kinds.iter().enumerate() {
        let is_last = use_last && lat_index == i;
        let prefix = tree_prefix(tab, is_last);

        sb.push('\n');
        sb.push_str(&prefix);
        sb.push_str(get_tag(child));
        match &child.node {
            ClassChild::Field(field) => inner_display_field(sb, field, tab, is_last),
            ClassChild::Method(function) => inner_display_methode(sb, function, tab, is_last),
            ClassChild::ImplBlock(_) => todo!(),
        }
    }
}

fn inner_display_methode_signatures(
    sb: &mut String,
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
        inner_display_methode_signature(sb, &methode.node, tab, is_last);
    }
}

fn inner_display_fields(sb: &mut String, fields: &Vec<Spanned<Field>>, tab: usize, use_last: bool) {
    let last_index = fields.len() - 1;

    for (i, Spanned { node: field, .. }) in fields.iter().enumerate() {
        let is_last = use_last && last_index == i;

        let prefix = tree_prefix(tab, is_last);
        sb.push('\n');
        sb.push_str(&prefix);
        sb.push_str("Field >> ");
        inner_display_field(sb, field, tab, is_last);
        if last_index == i {
            sb.push('\n');
            sb.push_str(&gap_prefix(tab));
        }
    }
}

fn inner_display_methode(sb: &mut String, methode: &Function, tab: usize, is_last: bool) {
    inner_display_methode_signature(sb, &methode.signature, tab, is_last);
    methode.block.inner_display(sb, tab + 1, is_last);
}

fn inner_display_methode_signature(
    sb: &mut String,
    methode: &FunctionSignature,
    tab: usize,
    is_last: bool,
) {
    inner_display_function_declaration(sb, &methode, tab, is_last);
}

fn inner_display_field(sb: &mut String, field: &Field, tab: usize, is_last: bool) {
    sb.push_str(&field.name);
    sb.push_str(": ");
    field.ty.inner_display(sb, tab, is_last);
    sb.push(' ');
    field.vis.inner_display(sb);
    if let Some(default) = &field.default_value {
        sb.push_str(" = ");
        default.node.inner_display(sb, tab, is_last);
    }
}
