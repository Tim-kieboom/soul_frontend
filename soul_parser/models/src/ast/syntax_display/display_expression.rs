use soul_utils::soul_names::{KeyWord, TypeWrapper};

use crate::{
    ElseKind, ExpressionKind, ReturnKind,
    ast::syntax_display::try_display_node_id,
    syntax_display::{DisplayKind, SyntaxDisplay, tree_prefix},
};

impl SyntaxDisplay for ExpressionKind {
    fn display(&self, kind: DisplayKind) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, kind, 0, true);
        sb
    }

    fn inner_display(&self, sb: &mut String, kind: DisplayKind, tab: usize, is_last: bool) {
        match self {
            ExpressionKind::Empty => sb.push_str("<empty>"),
            ExpressionKind::Default => sb.push_str("<default>"),
            ExpressionKind::Literal(literal) => sb.push_str(&literal.value_to_string()),
            ExpressionKind::Index(index) => {
                index.collection.node.inner_display(sb, kind, tab, is_last);
                sb.push('[');
                index.index.node.inner_display(sb, kind, tab, is_last);
                sb.push(']');
            }
            ExpressionKind::FunctionCall(function_call) => {
                try_display_node_id(sb, kind, function_call.node_id);
                sb.push_str(function_call.name.as_str());
                sb.push('(');
                for argument in &function_call.arguments {
                    argument.node.inner_display(sb, kind, tab, is_last);
                    sb.push(',');
                }
                sb.push(')');
            }
            ExpressionKind::Variable {
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
            ExpressionKind::MemberAccess(access) => {
                access.parent.node.inner_display(sb, kind, tab, is_last);
                sb.push('.');
                sb.push_str(access.member.as_str());
            }
            ExpressionKind::Unary(unary) => {
                sb.push_str(unary.operator.node.as_str());
                unary.expression.node.inner_display(sb, kind, tab, is_last);
            }
            ExpressionKind::Binary(binary) => {
                binary.left.node.inner_display(sb, kind, tab, is_last);
                sb.push(' ');
                sb.push_str(binary.operator.node.as_str());
                sb.push(' ');
                binary.right.node.inner_display(sb, kind, tab, is_last);
            }
            ExpressionKind::If(r#if) => {
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
            ExpressionKind::While(_while) => {
                sb.push_str(KeyWord::While.as_str());
                sb.push(' ');

                if let Some(condition) = &_while.condition {
                    condition.node.inner_display(sb, kind, tab, is_last);
                    sb.push(' ');
                }
                _while.block.inner_display(sb, kind, tab, is_last);
            }
            ExpressionKind::Deref(spanned) => {
                sb.push_str(TypeWrapper::Pointer.as_str());
                spanned.node.inner_display(sb, kind, tab, is_last);
            }
            ExpressionKind::Ref {
                is_mutable,
                expression,
            } => {
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
            ExpressionKind::ExpressionGroup(expression_group) => {
                expression_group.inner_display(sb, kind, tab, is_last)
            }
            ExpressionKind::Type(soul_type) => soul_type.inner_display(sb, kind, tab, is_last),
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
