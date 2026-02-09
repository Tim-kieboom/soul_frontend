use std::fmt::Write;
use soul_utils::soul_names::{KeyWord, TypeWrapper};

use crate::{
    ast::{
        ElseKind, ExpressionKind, ReturnKind,
        syntax_display::{DisplayKind, try_display_infered_type, try_display_node_id},
    },
    syntax_display::{SyntaxDisplay, tree_prefix},
};

impl SyntaxDisplay for ExpressionKind {
    fn display(&self, kind: &DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: &DisplayKind, tab: usize, is_last: bool) {
        match self {
            ExpressionKind::Null(_) => {
                sb.push_str(KeyWord::Null.as_str());
            }
            ExpressionKind::As(type_cast) => {
                type_cast.left.node.inner_display(sb, kind, tab, is_last);
                sb.push_str(" as ");
                type_cast.type_cast.inner_display(sb, kind, tab, is_last);
            }
            ExpressionKind::Default(id) => {
                try_display_infered_type(sb, kind, *id);
                sb.push_str("<default>");
            }
            ExpressionKind::Literal((id, literal)) => {
                try_display_node_id(sb, kind, *id);
                write!(sb, "{:?}", literal).expect("no write err");
            }
            ExpressionKind::Array(array) => {
                try_display_infered_type(sb, kind, array.id);
                if let Some(ty) = &array.collection_type {
                    ty.inner_display(sb, kind, tab, is_last)
                }
                sb.push('[');
                if let Some(ty) = &array.element_type {
                    ty.inner_display(sb, kind, tab, is_last);
                    sb.push_str(": ");
                }
                
                let last_index = array.values.len().saturating_sub(1);
                for (i, value) in array.values.iter().enumerate() {
                    value.node.inner_display(sb, kind, tab, is_last);
                    if i != last_index {
                        sb.push_str(", ");
                    }
                }

                sb.push(']');
            }
            ExpressionKind::Index(index) => {
                try_display_infered_type(sb, kind, index.id);
                index.collection.node.inner_display(sb, kind, tab, is_last);
                sb.push('[');
                index.index.node.inner_display(sb, kind, tab, is_last);
                sb.push(']');
            }
            ExpressionKind::FunctionCall(function_call) => {
                try_display_infered_type(sb, kind, function_call.id);
                try_display_node_id(sb, kind, function_call.resolved);
                sb.push_str(function_call.name.as_str());
                sb.push('(');

                let last_index = function_call.arguments.len().saturating_sub(1);
                for (i, argument) in function_call.arguments.iter().enumerate() {
                    argument.node.inner_display(sb, kind, tab, is_last);
                    if i != last_index {
                        sb.push_str(", ");
                    }
                }
                sb.push(')');
            }
            ExpressionKind::Variable {
                id:_,
                ident: variable,
                resolved,
            } => {
                try_display_node_id(sb, kind, *resolved);
                sb.push_str(variable.as_str());
            }
            ExpressionKind::ExternalExpression(external_expression) => {
                sb.push_str(external_expression.path.as_str());
                sb.push_str("::");
                external_expression
                    .expr
                    .node
                    .inner_display(sb, kind, tab, is_last);
            }
            ExpressionKind::Unary(unary) => {
                try_display_infered_type(sb, kind, unary.id);
                sb.push_str(unary.operator.node.as_str());
                unary.expression.node.inner_display(sb, kind, tab, is_last);
            }
            ExpressionKind::Binary(binary) => {
                sb.push('(');
                try_display_infered_type(sb, kind, binary.id);
                binary.left.node.inner_display(sb, kind, tab, is_last);
                sb.push(' ');
                sb.push_str(binary.operator.node.as_str());
                sb.push(' ');
                binary.right.node.inner_display(sb, kind, tab, is_last);
                sb.push(')');
            }
            ExpressionKind::If(r#if) => {
                try_display_infered_type(sb, kind, r#if.id);
                sb.push_str(KeyWord::If.as_str());
                sb.push(' ');
                r#if.condition.node.inner_display(sb, kind, tab, is_last);
                r#if.block.inner_display(sb, kind, tab, is_last);

                let mut current = r#if.else_branchs.as_ref();
                while let Some(else_kind) = current {
                    sb.push('\n');
                    let prefix = tree_prefix(tab, is_last);
                    sb.push_str(&prefix);

                    match &else_kind.node {
                        ElseKind::ElseIf(elif) => {
                            sb.push_str(KeyWord::Else.as_str());
                            sb.push(' ');
                            sb.push_str(KeyWord::If.as_str());
                            sb.push(' ');
                            elif.node
                                .condition
                                .node
                                .inner_display(sb, kind, tab, is_last);
                            elif.node.block.inner_display(sb, kind, tab, is_last);
                            current = elif.node.else_branchs.as_ref();
                        }
                        ElseKind::Else(el) => {
                            sb.push_str(KeyWord::Else.as_str());
                            sb.push(' ');
                            el.node.inner_display(sb, kind, tab, is_last);
                            current = None;
                        }
                    }
                }
            }
            ExpressionKind::While(r#while) => {
                sb.push_str(KeyWord::While.as_str());
                sb.push(' ');

                if let Some(condition) = &r#while.condition {
                    condition.node.inner_display(sb, kind, tab, is_last);
                    sb.push(' ');
                }
                r#while.block.inner_display(sb, kind, tab, is_last);
            }
            ExpressionKind::Deref{inner, id} => {
                try_display_infered_type(sb, kind, *id);
                sb.push_str(TypeWrapper::Pointer.as_str());
                inner.node.inner_display(sb, kind, tab, is_last);
            }
            ExpressionKind::Ref {
                id,
                is_mutable,
                expression,
            } => {
                try_display_infered_type(sb, kind, *id);
                if *is_mutable {
                    sb.push_str(TypeWrapper::MutRef.as_str());
                } else {
                    sb.push_str(TypeWrapper::ConstRef.as_str());
                }

                expression.node.inner_display(sb, kind, tab, is_last);
            }
            ExpressionKind::Block(block) => block.inner_display(sb, kind, tab + 1, is_last),
            ExpressionKind::ReturnLike(return_like) => {
                sb.push_str(return_like.kind.as_keyword().as_str());
                if let Some(value) = &return_like.value {
                    sb.push(' ');
                    value.node.inner_display(sb, kind, tab, is_last);
                }
            }
        }
    }
}

impl ReturnKind {
    pub fn as_keyword(&self) -> KeyWord {
        match self {
            ReturnKind::Break => KeyWord::Break,
            ReturnKind::Return => KeyWord::Return,
            ReturnKind::Continue => KeyWord::Continue,
        }
    }
}
