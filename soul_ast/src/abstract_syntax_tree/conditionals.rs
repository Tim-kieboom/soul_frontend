use crate::{
    abstract_syntax_tree::{
        block::Block,
        expression::{BoxExpression, Expression, ExpressionKind},
        expression_groups::ExpressionGroup,
        soul_type::SoulType,
        spanned::Spanned,
        statment::{Ident, Variable},
        syntax_display::{DisplayKind, SyntaxDisplay},
    },
    sementic_models::scope::{NodeId, ScopeId},
};
use soul_utils::{SoulError, SoulErrorKind, SoulResult, Span};
use itertools::Itertools;

/// A ternary conditional expression, e.g., `cond ? a : b`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Ternary {
    /// The condition expression.
    pub condition: BoxExpression,
    /// The expression to evaluate if the condition is true.
    pub if_branch: BoxExpression,
    /// The expression to evaluate if the condition is false.
    pub else_branch: BoxExpression,
}

/// A `while` loop statement.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct While {
    /// Optional condition expression. If `None`, the loop runs indefinitely.
    pub condition: Option<BoxExpression>,
    /// The loop body block.
    pub block: Block,
}

/// A `for` loop statement.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct For {
    /// Optional pattern to bind loop elements to.
    pub element: Option<ForPattern>,
    /// The collection expression to iterate over.
    pub collection: BoxExpression,
    /// The loop body block.
    pub block: Block,
}

/// A `match` expression for pattern matching.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Match {
    /// The expression being matched against.
    pub condition: BoxExpression,
    /// The match cases/arms.
    pub cases: Vec<CaseSwitch>,
}

/// A single case/arm in a `match` expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CaseSwitch {
    /// The pattern to match against.
    pub if_kind: IfCaseKind,
    /// The expression or block to execute if the pattern matches.
    pub do_fn: CaseDoKind,
    pub scope_id: Option<ScopeId>,
}

/// The kind of pattern in a match case.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum IfCaseKind {
    /// A wildcard pattern (`_`), optionally with a binding name.
    WildCard(Option<Variable>),
    /// Match against a specific expression value.
    Expression(Expression),
    /// Match against a variant with tuple parameters.
    Variant { name: Ident, params: Vec<Variable> },
    /// Match against a variant with named tuple parameters.
    NamedVariant { name: Ident, params: Vec<(Ident, Variable)> },
    /// Bind a value to a name, optionally with a condition.
    Bind { variable: Variable, condition: Expression },
}

/// A pattern used in `for` loops to destructure elements.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ForPattern {
    /// A simple identifier pattern.
    Ident {
        ident: Ident,
        resolved: Option<NodeId>,
    },
    /// A tuple pattern for destructuring tuples.
    Tuple(Vec<ForPattern>),
    /// A named tuple pattern for destructuring named tuples.
    NamedTuple(Vec<(Ident, ForPattern)>),
}

/// The body of a match case (block or expression).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CaseDoKind {
    /// A block of statements.
    Block(Spanned<Block>),
    /// A single expression.
    Expression(Expression),
}

/// An `if` statement or expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct If {
    /// The condition expression.
    pub condition: BoxExpression,
    /// The block to execute if the condition is true.
    pub block: Block,
    /// Optional `else if` and `else` branches.
    pub else_branchs: Option<IfArm>,
}

pub type IfArm = Box<Spanned<ElseKind>>;
pub trait IfArmHelper{
    fn new_arm(kind: ElseKind, span: Span) -> Self;
    fn try_next_mut(&mut self) -> Option<&mut Option<IfArm>>;
}
impl IfArmHelper for IfArm {
    fn new_arm(kind: ElseKind, span: Span) -> Self {
        Self::new(Spanned::new(kind, span))
    }
    fn try_next_mut(&mut self) -> Option<&mut Option<IfArm>> {
        match &mut self.node {
            ElseKind::ElseIf(spanned) => Some(&mut spanned.node.else_branchs),
            ElseKind::Else(_) => None,
        }
    }
}

/// The kind of else branch in an `if` statement.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ElseKind {
    /// An `else if` branch (another conditional).
    ElseIf(Box<Spanned<If>>),
    /// An `else` branch (unconditional).
    Else(Spanned<Block>),
}

/// A type comparison expression (`typeof`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompareTypeOf {
    /// The left-hand side expression.
    pub left: BoxExpression,
    /// The type to compare against.
    pub ty: SoulType,
}

impl ForPattern {
    pub fn from_expression(expression: Expression) -> SoulResult<Self> {
        match expression.node {
            ExpressionKind::Variable { ident, resolved } => Ok(ForPattern::Ident {
                ident,
                resolved,
            }),
            ExpressionKind::ExpressionGroup(ExpressionGroup::Tuple(tuple)) => {
                let mut fors = Vec::with_capacity(tuple.values.len());
                for el in tuple.values {
                    fors.push(ForPattern::from_expression(el)?);
                }
                Ok(ForPattern::Tuple(fors))
            }
            ExpressionKind::ExpressionGroup(ExpressionGroup::NamedTuple(named)) => {
                let mut fors = Vec::with_capacity(named.values.len());
                for (name, el) in named.values {
                    fors.push((name, ForPattern::from_expression(el)?));
                }
                Ok(ForPattern::NamedTuple(fors))
            }
            _ => Err(SoulError::new(
                format!(
                    "'{}' should be ident, tuple or named tuple",
                    expression.node.display(DisplayKind::Parser)
                ),
                SoulErrorKind::InvalidExpression,
                Some(expression.span),
            )),
        }
    }

    pub fn display(&self) -> String {
        match self {
            ForPattern::Ident { ident, .. } => ident.node.clone(),
            ForPattern::Tuple(items) => {
                format!("({})", items.iter().map(|el| el.display()).join(", "))
            }
            ForPattern::NamedTuple(items) => format!(
                "({})",
                items
                    .iter()
                    .map(|(name, el)| format!("{}: {}", name.node, el.display()))
                    .join(", ")
            ),
        }
    }
}
