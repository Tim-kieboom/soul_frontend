use itertools::Itertools;

use crate::{abstract_syntax_tree::{block::Block, enum_like::{Enum, Union}, expression::{Expression, ExpressionKind}, function::Function, objects::{Class, Field, Struct, Trait}, soul_type::SoulType, spanned::Spanned, syntax_display::{SyntaxDisplay, tree_prefix}}, error::Span};

/// A statement in the Soul language, wrapped with source location information.
pub type Statement = Spanned<StatementKind>;

/// The different kinds of statements that can appear in the language.
///
/// Each variant corresponds to a syntactic construct, ranging from expressions
/// to type definitions and control structures.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    EndFile,

    /// A standalone expression.
    Expression(Expression),

    /// A variable declaration.
    Variable(Ident),
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
            StatementKind::Expression(spanned) => {
                sb.push_str(&prefix);
                sb.push_str("Expression >> ");
                spanned.node.inner_display(sb, tab, is_last);
            },
            StatementKind::Variable(var) => {
                sb.push_str(&prefix);
                sb.push_str("Variable >> ");
                sb.push_str(&var);
            },
            StatementKind::Assignment(assignment) => {
                sb.push_str(&prefix);
                sb.push_str("Assignment >> ");
                assignment.left.node.inner_display(sb, tab, is_last);
                sb.push_str(" = ");
                assignment.right.node.inner_display(sb, tab, is_last);
            },
            StatementKind::Function(function) => {
                sb.push_str(&prefix);
                sb.push_str("Function >> ");
                sb.push_str(&function.signature.name);
                sb.push('(');
                sb.push_str(&function.signature.parameters.types.iter().map(|(name, el)| format!("{name}: {}", el.display())).join(", "));
                sb.push(')');
                sb.push_str(": ");
                function.signature.return_type.inner_display(sb, tab, is_last);
                function.block.inner_display(sb, tab, is_last);
            },
            StatementKind::UseBlock(use_block) => {
                sb.push_str(&prefix);
                sb.push_str("UseBlock >> ");
                if let Some(impl_trait) = &use_block.impl_trait {
                    sb.push_str(" impl ");
                    impl_trait.inner_display(sb, tab, is_last);
                }
                use_block.block.inner_display(sb, tab, is_last);
            },
            StatementKind::Struct(_struct) => {
                const USE_LAST: bool = true;
                sb.push_str(&prefix);
                sb.push_str("Struct >> ");
                sb.push_str(&_struct.name);
                inner_display_fields(sb, &_struct.fields, tab+1, USE_LAST);
            },
            StatementKind::Class(_) => todo!(),
            StatementKind::Trait(_) => todo!(),
            StatementKind::Enum(_) => todo!(),
            StatementKind::Union(_) => todo!(),
            StatementKind::CloseBlock => sb.push_str(&prefix),
        }
    }
}

fn inner_display_fields(sb: &mut String, fields: &Vec<Spanned<Field>>, tab: usize, use_last: bool) {
    
    let get_is_last = |i: usize| fields.len() -1 == i;

    for (i, Spanned{node: field, ..}) in fields.iter().enumerate() {
        let is_last = use_last && get_is_last(i);
        let prefix = tree_prefix(tab, is_last);

        sb.push('\n');
        sb.push_str(&prefix);
        sb.push_str("Field >> ");
        sb.push_str(&field.name);
        sb.push_str(": ");
        field.ty.inner_display(sb, tab, is_last);
        sb.push(' ');
        field.vis.inner_display(sb);
        if let Some(default) = &field.default_value {
            sb.push_str(" = ");
            default.node.inner_display(sb, tab, is_last);
        }
        
        if field.vis.get.is_none() && field.vis.set.is_none() {
            return
        }

    }
}