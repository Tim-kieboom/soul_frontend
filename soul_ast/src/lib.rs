//! Models for the Soul programming language.
//!
//! This crate provides the core data structures and types used to represent
//! Soul language programs, including the Abstract Syntax Tree (AST), error types,
//! scope management, and language keywords/symbols.

pub mod abstract_syntax_tree;
pub mod sementic_models;
use soul_utils::{soul_names::Operator, SementicLevel};

use crate::{abstract_syntax_tree::{AbstractSyntaxTree, BinaryOperatorKind, Ident, UnaryOperatorKind}, sementic_models::{AstMetadata, scope::NodeId}};

#[derive(Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StackArrayKind {
    Number(u64),
    Ident {
        ident: Ident,
        resolved: Option<NodeId>,
    },
}

pub trait ConvertOperator {
    fn to_unary(&self) -> Option<UnaryOperatorKind>;
    fn to_binary(&self) -> Option<BinaryOperatorKind>;
}
impl ConvertOperator for Operator {
    fn to_unary(&self) -> Option<UnaryOperatorKind> {
        Some(match self {
            Operator::Not => UnaryOperatorKind::Not,
            Operator::Sub => UnaryOperatorKind::Neg,
            Operator::Mul => UnaryOperatorKind::DeRef,
            Operator::BitAnd => UnaryOperatorKind::MutRef,
            Operator::ConstRef => UnaryOperatorKind::ConstRef,
            Operator::Incr => UnaryOperatorKind::Increment { before_var: true },
            Operator::Decr => UnaryOperatorKind::Decrement { before_var: true },
            _ => return None,
        })
    }

    fn to_binary(&self) -> Option<BinaryOperatorKind> {
        Some(match self {
            Operator::Eq => BinaryOperatorKind::Eq,
            Operator::Mul => BinaryOperatorKind::Mul,
            Operator::Div => BinaryOperatorKind::Div,
            Operator::Mod => BinaryOperatorKind::Mod,
            Operator::Add => BinaryOperatorKind::Add,
            Operator::Sub => BinaryOperatorKind::Sub,
            Operator::Root => BinaryOperatorKind::Root,
            Operator::Power => BinaryOperatorKind::Pow,
            Operator::LessEq => BinaryOperatorKind::Le,
            Operator::GreatEq => BinaryOperatorKind::Ge,
            Operator::LessThen => BinaryOperatorKind::Lt,
            Operator::NotEq => BinaryOperatorKind::NotEq,
            Operator::Range => BinaryOperatorKind::Range,
            Operator::BitOr => BinaryOperatorKind::BitOr,
            Operator::LogOr => BinaryOperatorKind::LogOr,
            Operator::GreatThen => BinaryOperatorKind::Gt,
            Operator::BitAnd => BinaryOperatorKind::BitAnd,
            Operator::BitXor => BinaryOperatorKind::BitXor,
            Operator::LogAnd => BinaryOperatorKind::LogAnd,

            Operator::Not | Operator::Incr | Operator::Decr | Operator::ConstRef => return None,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParseResonse {
    pub syntax_tree: AbstractSyntaxTree,
    pub sementic_info: AstMetadata,
}
impl ParseResonse {
    pub fn get_fatal_count(&self, fatal_level: SementicLevel) -> usize {
        self.sementic_info
            .faults
            .iter()
            .filter(|fault| fault.is_fatal(fatal_level))
            .count()
    }
}
