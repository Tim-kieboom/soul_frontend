use itertools::Itertools;
use crate::{abstract_syntax_tree::{block::Block, conditionals::{CaseDoKind, ElseKind, For, ForPattern, If, IfCaseKind, Match, Ternary, While}, expression_groups::ExpressionGroup, function::{FunctionCall, Lambda, LamdbaBodyKind, StaticMethod, StructConstructor}, literal::Literal, operator::{Binary, Unary}, soul_type::SoulType, spanned::Spanned, statment::Ident, syntax_display::{SyntaxDisplay, tree_prefix}}, error::Span, soul_names::{KeyWord, TypeWrapper}, soul_page_path::SoulPagePath};

/// An expression in the Soul language, wrapped with source location information.
pub type Expression = Spanned<ExpressionKind>;
/// A boxed expression (used to avoid deep recursion in the AST).
pub type BoxExpression = Box<Expression>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExpressionKind {
    /// An empty expression (placeholder).
    Empty,
    /// A `default` literal or default value.
    Default,
    /// A literal value (number, string, etc.).
    Literal(Literal),
    
    /// Indexing into a collection, e.g., `arr[i]`.
    Index(Index),
    /// A lambda `() => {}`.
    Lambda(Lambda),
    /// A function call, e.g., `foo(x, y)`.
    FunctionCall(FunctionCall),
    /// Constructing a struct, e.g., `Point { x: 1, y: 2 }`.
    StructConstructor(StructConstructor),

    /// Accessing a field on an instance, e.g., `obj.field`.
    FieldAccess(FieldAccess),
    /// Accessing a static field, e.g., `Type.field`.
    StaticFieldAccess(StaticFieldAccess),
    /// Calling a static method, e.g., `Type.method()`.
    StaticMethod(StaticMethod),

    /// Referring to a variable `var`.
    Variable(Ident),
    /// An external expression from another page/module `path.to.something.expression`.
    ExternalExpression(ExternalExpression),

    /// A unary operation (negation, increment, etc.) `!1`.
    Unary(Unary),
    /// A binary operation (addition, multiplication, comparison, etc.) `1 + 2`.
    Binary(Binary),

    /// An `if` expression `if true {Println("is true")}`.
    If(If),
    /// A `for` loop `for i in 1..2 {Println(f"num:{i}")}`.
    For(For),
    /// A `while` loop `while true {Println("loop")}`.
    While(While),
    /// A `match` expression `match booleanVar {true => (), false => }`.
    Match(Match),
    /// A ternary expression `cond ? a : b`.
    Ternary(Ternary),

    /// A dereference, e.g., `*ptr`.
    Deref(BoxExpression),
    /// reference, e.g., `&x`(mut) or `@x`(const).
    Ref{is_mutable: bool, expression: BoxExpression},

    /// A block of statements, returning the last expression `{/*stuff*/}`.
    Block(Block),
    /// Return-like expressions (`return`, `break`, `fall`) `return 1`.
    ReturnLike(ReturnLike),
    /// A grouped expression, e.g., tuples, namedTuples or arrays.
    ExpressionGroup(ExpressionGroup),
}

/// A `return`, `fall`, or `break`-like expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReturnLike {
    pub value: Option<BoxExpression>,
    pub kind: ReturnKind
}
/// The kind of return-like expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ReturnKind {
    /// A returns function.
    Return,
    /// A returns current loop.
    Break,
    /// continue loop.
    Continue,
}

/// An expression from an external page/module.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExternalExpression {
    /// The path to the external page/module.
    pub path: SoulPagePath,
    /// The expression being accessed.
    pub expr: BoxExpression,
}

/// A static field access on a type, e.g., `Type.field`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StaticFieldAccess {
    /// The type being accessed.
    pub object: SoulType,
    /// The field name.
    pub field: Ident,
}

/// A field access on an instance, e.g., `obj.field`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldAccess {
    /// The object expression.
    pub object: BoxExpression,
    /// The field name.
    pub field: Ident,
}

/// An index operation, e.g., `arr[i]`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Index {
    /// The collection being indexed.
    pub collection: BoxExpression,
    /// The index expression.
    pub index: BoxExpression,
}

impl Expression {
    pub fn new_literal(literal: Literal, span: Span) -> Self {
        Self::new(ExpressionKind::Literal(literal), span)
    }
}

impl SyntaxDisplay for ExpressionKind {
    
    fn display(&self) -> String {
        let mut sb = String::new();
        self.inner_display(&mut sb, 0, true);
        sb
    }
    
    fn inner_display(&self, sb: &mut String, tab: usize, is_last: bool) {
        match self {
            ExpressionKind::Empty => sb.push_str("<empty>"),
            ExpressionKind::Default => sb.push_str("<default>"),
            ExpressionKind::Literal(literal) => sb.push_str(&literal.value_to_string()),
            ExpressionKind::Index(index) => {
                index.collection.node.inner_display(sb, tab, is_last);
                sb.push('[');
                index.index.node.inner_display(sb, tab, is_last);
                sb.push(']');
            },
            ExpressionKind::Lambda(lambda) => {
                sb.push('(');
                sb.push_str(&lambda.arguments.values.iter().map(|el| el.node.display()).join(", "));
                sb.push(')');
                match &lambda.body {
                    LamdbaBodyKind::Block(block) => block.inner_display(sb, tab+1, is_last),
                    LamdbaBodyKind::Expression(spanned) => spanned.node.inner_display(sb, tab+1, is_last),
                }
            },
            ExpressionKind::FunctionCall(function_call) => {
                sb.push_str(&function_call.name);
                sb.push('(');
                sb.push_str(&function_call.arguments.values.iter().map(|el| el.node.display()).join(", "));
                sb.push(')');
            }
            ExpressionKind::StructConstructor(struct_constructor) => {
                struct_constructor.calle.inner_display(sb, tab, is_last);
                sb.push_str(&struct_constructor.arguments.values.iter().map(|(name, expr)| format!("{}: {}", name, expr.node.display())).join(", "));
            },
            ExpressionKind::FieldAccess(field_access) => {
                field_access.object.node.inner_display(sb, tab, is_last);
                sb.push('.');
                sb.push_str(&field_access.field);
            },
            ExpressionKind::StaticFieldAccess(static_field_access) => {
                static_field_access.object.inner_display(sb, tab, is_last);
                sb.push('.');
                sb.push_str(&static_field_access.field);
            },
            ExpressionKind::StaticMethod(static_method) => {
                static_method.callee.node.inner_display(sb, tab, is_last);
                sb.push('.');
                sb.push_str(&static_method.name);
                sb.push('(');
                sb.push_str(&static_method.arguments.values.iter().map(|el| el.node.display()).join(", "));
                sb.push(')');
            },
            ExpressionKind::Variable(variable) => {
                sb.push_str(&variable);
            },
            ExpressionKind::ExternalExpression(external_expression) => {
                sb.push_str(external_expression.path.as_str());
                sb.push_str("::");
                external_expression.expr.node.inner_display(sb, tab, is_last);
            },
            ExpressionKind::Unary(unary) => {
                sb.push_str(unary.operator.node.as_str());
                unary.expression.node.inner_display(sb, tab, is_last);
            },
            ExpressionKind::Binary(binary) => {
                binary.left.node.inner_display(sb, tab, is_last);
                sb.push(' ');
                sb.push_str(binary.operator.node.as_str());
                sb.push(' ');
                binary.right.node.inner_display(sb, tab, is_last);
            },
            ExpressionKind::If(_if) => {
                sb.push_str(KeyWord::If.as_str());
                sb.push(' ');
                _if.condition.node.inner_display(sb, tab, is_last);
                _if.block.inner_display(sb, tab, is_last);
                for else_kind in &_if.else_branchs {
                    
                    match &else_kind.node {
                        ElseKind::ElseIf(spanned) => {
                            sb.push_str(KeyWord::Else.as_str());
                            sb.push(' ');
                            sb.push_str(KeyWord::If.as_str());
                            sb.push(' ');
                            spanned.node.condition.node.inner_display(sb, tab, is_last);
                            spanned.node.block.inner_display(sb, tab, is_last);
                        },
                        ElseKind::Else(spanned) => {
                            sb.push_str(KeyWord::Else.as_str());
                            sb.push(' ');
                            spanned.node.inner_display(sb, tab, is_last);
                        },
                    }
                }
            },
            ExpressionKind::For(_for) => {
                sb.push_str(KeyWord::For.as_str());
                sb.push(' ');
                if let Some(element) = &_for.element {

                    for_pattern_to_string(sb, element);
                    sb.push(' ');
                    sb.push_str(KeyWord::InForLoop.as_str());
                    sb.push(' ');
                }
                _for.collection.node.inner_display(sb, tab, is_last);
                _for.block.inner_display(sb, tab+1, is_last);
            },
            ExpressionKind::While(_while) => {
                sb.push_str(KeyWord::While.as_str());
                sb.push(' ');
                
                if let Some(condition) = &_while.condition {
                    condition.node.inner_display(sb, tab, is_last);
                    sb.push(' ');
                }
                _while.block.inner_display(sb, tab, is_last);
            },
            ExpressionKind::Match(_match) => {
                let prefix = tree_prefix(tab+1, is_last);
                
                sb.push_str(KeyWord::While.as_str());
                sb.push(' ');
                
                _match.condition.node.inner_display(sb, tab, is_last);
                sb.push(' ');
                for case in &_match.cases {
                    
                    match &case.if_kind {
                        IfCaseKind::WildCard(ident) => {
                            sb.push('\n');
                            sb.push_str(&prefix);
                            sb.push_str(ident.as_ref().unwrap_or(&String::new()));
                            sb.push_str(" => ");
                        },
                        IfCaseKind::Expression(spanned) => {
                            sb.push('\n');
                            sb.push_str(&prefix);
                            spanned.node.inner_display(sb, tab, is_last);
                            sb.push_str(" => ");
                        },
                        IfCaseKind::Variant { name, params } => {
                            sb.push('\n');
                            sb.push_str(&prefix);
                            sb.push_str(&name);
                            sb.push('(');
                            for value in &params.values {
                                value.node.inner_display(sb, tab, is_last);
                                sb.push_str(", ");
                            }
                            sb.push(')');
                            sb.push_str(" => ");
                        },
                        IfCaseKind::NamedVariant { name, params } => {
                            sb.push('\n');
                            sb.push_str(&prefix);
                            sb.push_str(&name);
                            sb.push('{');
                            for (name, value) in &params.values {
                                sb.push_str(&name);
                                sb.push_str(": ");
                                value.node.inner_display(sb, tab, is_last);
                                sb.push_str(", ");
                            }
                            sb.push('}');
                            sb.push_str(" => ");
                        },
                        IfCaseKind::Bind { name, condition } => {
                            sb.push('\n');
                            sb.push_str(&prefix);
                            sb.push_str(&name);
                            sb.push(' ');
                            sb.push_str(KeyWord::If.as_str());
                            sb.push(' ');
                            condition.node.inner_display(sb, tab, is_last);
                            sb.push_str(" => ");
                        },
                    }

                    match &case.do_fn {
                        CaseDoKind::Block(spanned) => spanned.node.inner_display(sb, tab+2, is_last),
                        CaseDoKind::Expression(spanned) => spanned.node.inner_display(sb, tab, is_last),
                    }
                }
            },
            ExpressionKind::Ternary(ternary) => {
                ternary.condition.node.inner_display(sb, tab, is_last);
                sb.push_str(" ? ");
                ternary.if_branch.node.inner_display(sb, tab, is_last);
                sb.push_str(" : ");
                ternary.else_branch.node.inner_display(sb, tab, is_last);
            },
            ExpressionKind::Deref(spanned) => {
                sb.push_str(TypeWrapper::Pointer.as_str());
                spanned.node.inner_display(sb, tab, is_last);
            },
            ExpressionKind::Ref { is_mutable, expression } => {
                if *is_mutable {
                    sb.push_str(TypeWrapper::MutRef.as_str());
                }
                else {
                    sb.push_str(TypeWrapper::ConstRef.as_str());
                }

                expression.node.inner_display(sb, tab, is_last);
            }
            ExpressionKind::Block(block) => block.inner_display(sb, tab, is_last),
            ExpressionKind::ReturnLike(return_like) => {
                sb.push_str(return_like.kind.as_keyword().as_str());
                if let Some(value) = &return_like.value {
                    sb.push(' ');
                    value.node.inner_display(sb, tab, is_last);
                }
            },
            ExpressionKind::ExpressionGroup(expression_group) => expression_group.inner_display(sb, tab, is_last),
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

fn for_pattern_to_string(sb: &mut String, el: &ForPattern) {
    match el {
        ForPattern::Ident(ident) => sb.push_str(ident),
        ForPattern::Tuple(items) => {
            sb.push('(');
            for item in items {
                for_pattern_to_string(sb, item);
                sb.push_str(", ");
            }
            sb.push(')');
        },
        ForPattern::NamedTuple(items) => {
            sb.push('(');
            for (name, item) in items {
                sb.push_str(&name);
                sb.push_str(": ");
                for_pattern_to_string(sb, item);
                sb.push_str(", ");
            }
            sb.push(')');
        },
    }
}
