use crate::{
    AstStore, NodeId,
    block::BlockId,
    expression::{Binding, Expression, ExpressionId, ExpressionKind, FunctionCall},
    soul_type::{Generic, SoulType},
};
use soul_utils::{
    FunctionId, Ident, TypeModifier, bitflags,
    collections::soul_import_path::SoulImportPath,
    error::SoulResult,
    fault::Fault,
    impl_soul_ids, soul_error_internal,
    span::{ItemMetaData, Span, Spanned},
};

impl_soul_ids!(StatementId);

/// A statement in the Soul language, wrapped with source location information.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Statement {
    pub span: Span,
    pub node: StatementKind,
    pub meta_data: ItemMetaData,
    is_public: bool,
}

/// The different kinds of statements that can appear in the language.
///
/// Each variant corresponds to a syntactic construct, ranging from expressions
/// to type definitions and control structures.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StatementKind {
    /// Imported paths
    Import(Import),
    /// A standalone expression.
    Expression {
        expression: ExpressionId,
        ends_semicolon: bool,
    },
    /// A variable declaration.
    Variable(Variable),
    /// An assignment to an existing variable.
    Assignment(Assignment),
    /// A methode use block.
    UseBlock(UseBlock),
    /// A function declaration (with body block).
    Function(FunctionId),
    /// A external function declaration (without body block).
    ExternalFunction(FunctionId),

    Enum(Enum),
    Trait(Trait),
    Struct(Struct),
    TypeDef(TypeDef),
    /// A union declaration: `union Name { ... }`.
    /// Same data model as an enum, but without a backing `as T` type.
    Union(Enum),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TypeDef {
    pub new_type: SoulType,
    pub old_type: SoulType,
    pub is_distinct: bool,
}

/// A trait definition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Trait {
    pub id: NodeId,

    pub name: Ident,
    pub generics: Vec<Generic>,
    pub trait_impls: Vec<Ident>,
    pub typedefs: Vec<SoulType>,
    pub methods: Vec<FunctionId>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Enum {
    pub id: NodeId,

    pub name: Ident,
    pub variants: Vec<EnumVariant>,
    pub impl_type: Option<SoulType>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EnumVariant {
    Normal(Ident),
    Assigned { name: Ident, value: ExpressionId },
    Union(UnionKind),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum UnionKind {
    Tuple {
        name: Ident,
        parameters: Vec<SoulType>,
    },
    NamedTuple {
        name: Ident,
        parameters: Vec<(Ident, SoulType)>,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    pub id: NodeId,

    pub name: Ident,
    pub fields: Vec<Field>,
    pub generics: Vec<Generic>,
    pub statements: Vec<StatementId>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub value: Variable,
    pub is_public: bool,
}
impl Field {
    pub fn new(value: Variable, is_public: bool) -> Self {
        Self { value, is_public }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Methode {
    pub id: FunctionId,
    pub is_public: bool,
}
impl Methode {
    pub fn new(id: FunctionId, is_public: bool) -> Self {
        Self { id, is_public }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UseBlock {
    pub use_generics: Vec<Generic>,
    pub ty: SoulType,
    pub impls: Vec<ImplBlock>,
    pub methods: Vec<Methode>,
    pub statements: Vec<StatementId>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImplBlock {
    pub impl_trait: SoulType,
    pub methods: Vec<FunctionId>,
}

/// Imported paths
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Import {
    pub paths: Vec<ImportPath>,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportPath {
    pub lib_name: Option<String>,
    pub module: SoulImportPath,
    pub kind: ImportKind,
}
impl ImportPath {
    pub fn new() -> Self {
        Self {
            lib_name: None,
            kind: ImportKind::This,
            module: SoulImportPath::default(),
        }
    }
}
impl Default for ImportPath {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImportKind {
    This,
    Glob,
    Alias(Ident),
    Module,
    Items {
        has_this: bool,
        this_alias: Option<Ident>,
        items: Vec<ImportItem>,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImportItem {
    Normal(Ident),
    Alias { name: Ident, alias: Ident },
}

/// A destructuring pattern for variable declarations.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum VarPattern {
    /// Discard: `_` binds nothing.
    Discard,
    /// Simple binding: `name` or `mut name`.
    Simple {
        binding: Binding,
        modifier: TypeModifier,
    },
    /// Tuple destructuring: `(a, b, ..)`.
    Tuple(TuplePattern),
    /// Named tuple / record destructuring: `{field1, field2: alias, ..}`.
    NamedTuple(NamedTuplePattern),
    /// Constructor destructuring: `Struct{a, b: alias, ..}`.
    Constructor(VarConstructorPattern),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TuplePattern {
    pub elements: Vec<VarPattern>,
    /// Whether `..` (rest) is present at the end.
    pub rest: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NamedTuplePattern {
    pub fields: Vec<VarNamedPattern>,
    pub rest: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VarNamedPattern {
    /// The variable to bind (name = alias if `field: alias`, else field name).
    /// `None` when the field is discarded (`{field: _}`).
    pub binding: Option<Binding>,
    /// The field name being destructured.
    pub field: Ident,
    /// The modifier of the binding (mut or const).
    pub modifier: TypeModifier,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VarConstructorPattern {
    pub type_name: Ident,
    pub fields: Vec<VarNamedPattern>,
    pub rest: bool,
}

/// A variable declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    pub id: NodeId,
    pub is_public: bool,

    /// The destructuring pattern.
    pub pattern: VarPattern,
    /// The modifier of the variable.
    pub modifier: TypeModifier,
    /// The type of the variable.
    pub ty: Option<SoulType>,
    /// Optional initial value expression.
    pub initialize_value: Option<ExpressionId>,
}

impl Variable {
    pub fn new_const(
        id: NodeId,
        pattern: VarPattern,
        ty: Option<SoulType>,
        value: Option<ExpressionId>,
    ) -> Self {
        Self {
            id,
            pattern,
            ty,
            is_public: false,
            initialize_value: value,
            modifier: TypeModifier::Const,
        }
    }

    pub fn apply_modifier(mut self, modifier: TypeModifier) -> Self {
        self.modifier = modifier;
        self
    }

    /// If this is a simple pattern, returns the variable name.
    pub fn name(&self) -> Option<&Ident> {
        match &self.pattern {
            VarPattern::Simple { binding, .. } => Some(&binding.ident),
            _ => None,
        }
    }
}

/// An assignment statement, e.g., `x = y + 1`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Assignment {
    /// The left-hand side expression (the variable being assigned to).
    pub left: ExpressionId,
    /// The right-hand side expression (the value being assigned).
    pub right: ExpressionId,
}

/// A function definition with a signature and body block.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Function {
    /// The function's signature (name, parameters, return type, etc.).
    pub signature: FunctionSignature,
    /// The function's body block.
    pub block: BlockId,
}

bitflags! {
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct FunctionModifier: u8 {
        PUBLIC    = 1 << 0,
        ASYNC     = 1 << 1,
        CONST     = 1 << 2,
    }
}

pub type FunctionSignature = Box<Spanned<InnerFunctionSignature>>;
pub trait FunctionSignatureHelper {
    fn with_span(value: InnerFunctionSignature, span: Span) -> Self;
}
impl FunctionSignatureHelper for FunctionSignature {
    fn with_span(value: InnerFunctionSignature, span: Span) -> Self {
        Self::new(Spanned::new(value, span))
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InnerFunctionSignature {
    pub id: FunctionId,
    pub node_id: NodeId,
    pub modifier: FunctionModifier,

    /// The name of the function.
    pub name: Ident,
    /// Method type, if specified.
    pub method_type: SoulType,
    /// Return type, if specified.
    pub return_type: SoulType,
    /// Function parameters.
    pub parameters: Vec<Parameter>,
    pub generics: Vec<Generic>,
    pub function_kind: FunctionThisKind,
    pub external: Option<ExternLanguage>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Parameter {
    pub id: NodeId,
    pub name: Ident,
    pub ty: SoulType,
    pub is_mut: bool,
    pub default: Option<ExpressionId>,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExternLanguage {
    C,
}
impl ExternLanguage {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExternLanguage::C => "C",
        }
    }
}

/// Optional `this` parameter type.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FunctionThisKind {
    /// `This.(..)`
    Ctor,
    /// `This.[T](array)`
    ArrayCtor,
    /// `func(..)`
    Static,
    /// `func(&mut this, ..)`
    MutRef,
    /// `func(this, ..)`
    Consume,
    /// `func(&this, ..)`
    ConstRef,
}
impl FunctionThisKind {
    pub fn display(&self) -> Option<&'static str> {
        match self {
            FunctionThisKind::Ctor | FunctionThisKind::Static | FunctionThisKind::ArrayCtor => None,
            FunctionThisKind::MutRef => Some("&mut this"),
            FunctionThisKind::Consume => Some("this"),
            FunctionThisKind::ConstRef => Some("&this"),
        }
    }
}

impl EnumVariant {
    pub const fn get_variant_name(&self) -> &'static str {
        match self {
            EnumVariant::Normal(_) => "Normal",
            EnumVariant::Union { .. } => "Union",
            EnumVariant::Assigned { .. } => "Assign",
        }
    }
}

impl Statement {
    pub const fn is_expression(&self) -> bool {
        matches!(self.node, StatementKind::Expression { .. })
    }

    pub const fn error() -> Self {
        Self {
            node: StatementKind::Function(FunctionId::ERROR),
            span: Span::error(),
            meta_data: ItemMetaData {
                attributes: Vec::new(),
            },
            is_public: false,
        }
    }

    pub const fn new(node: StatementKind, span: Span) -> Self {
        Self {
            node,
            span,
            is_public: false,
            meta_data: ItemMetaData { attributes: vec![] },
        }
    }

    pub const fn new_variable(variable: Variable, span: Span) -> Self {
        Self {
            span,
            is_public: false,
            node: StatementKind::Variable(variable),
            meta_data: ItemMetaData { attributes: vec![] },
        }
    }

    pub const fn with_meta_data(node: StatementKind, span: Span, meta_data: ItemMetaData) -> Self {
        Self {
            node,
            span,
            meta_data,
            is_public: false,
        }
    }

    pub fn from_typedef(spanned: Spanned<TypeDef>) -> Self {
        let Spanned { value, span } = spanned;
        Self::new(StatementKind::TypeDef(value), span)
    }

    pub fn from_expression(
        store: &AstStore,
        expression: ExpressionId,
        ends_semicolon: bool,
    ) -> Self {
        let span = store.expressions[expression].span;
        Self::new(
            StatementKind::Expression {
                expression,
                ends_semicolon,
            },
            span,
        )
    }

    pub fn from_external_function(function: Spanned<FunctionId>) -> Self {
        let Spanned { value, span } = function;
        Self::new(StatementKind::ExternalFunction(value), span)
    }

    pub fn from_function(function: Spanned<FunctionId>) -> Self {
        let Spanned { value, span } = function;
        Self::new(StatementKind::Function(value), span)
    }

    pub fn from_function_call(
        store: &mut AstStore,
        call: Spanned<FunctionCall>,
        ends_semicolon: bool,
    ) -> Self {
        let value = store.insert_expression(Expression::from_function_call(call));
        Self::from_expression(store, value, ends_semicolon)
    }

    pub fn new_block(
        store: &mut AstStore,
        block: BlockId,
        span: Span,
        ends_semicolon: bool,
    ) -> Self {
        let expression = Expression::new(ExpressionKind::Block(block), span);

        Self::new(
            StatementKind::Expression {
                ends_semicolon,
                expression: store.insert_expression(expression),
            },
            span,
        )
    }

    pub fn is_public(&self) -> bool {
        self.is_public
    }

    pub fn try_set_async(&mut self, store: &mut AstStore, span: Span) -> SoulResult<()> {
        match &mut self.node {
            StatementKind::Function(id) | StatementKind::ExternalFunction(id) => {
                let kind = store.functions.get_mut(*id).ok_or(soul_error_internal!(
                    format!("{id:?} not found"),
                    Some(span)
                ))?;
                kind.signature_mut().modifier |= FunctionModifier::ASYNC;
                Ok(())
            }
            _ => Err(Fault::error(
                "only functions can be declared `async`",
                Some(span),
            )),
        }
    }

    pub fn try_set_public(
        &mut self,
        store: &mut AstStore,
        is_public: bool,
        span: Span,
    ) -> SoulResult<()> {
        match &mut self.node {
            StatementKind::Enum(_)
            | StatementKind::Trait(_)
            | StatementKind::Struct(_)
            | StatementKind::TypeDef(_)
            | StatementKind::Union(_) => self.is_public = is_public,

            StatementKind::Function(id) | StatementKind::ExternalFunction(id) => {
                let kind = store.functions.get_mut(*id).ok_or(soul_error_internal!(
                    format!("{id:?} not found"),
                    Some(span)
                ))?;
                kind.signature_mut().modifier |= FunctionModifier::PUBLIC;
                self.is_public = is_public;
            }

            StatementKind::Variable(variable) => {
                self.is_public = is_public;
                variable.is_public = is_public;
            }

            StatementKind::Import(_)
            | StatementKind::UseBlock(_)
            | StatementKind::Assignment(_)
            | StatementKind::Expression { .. } => {
                return Err(Fault::error(
                    format!("{} can not be public", self.node.variant_name()),
                    Some(span),
                ));
            }
        }

        Ok(())
    }
}
impl StatementKind {
    pub const fn variant_name(&self) -> &'static str {
        match self {
            StatementKind::Enum(_) => "enum",
            StatementKind::Union(_) => "union",
            StatementKind::Trait(_) => "trait",
            StatementKind::Struct(_) => "struct",
            StatementKind::Import(_) => "import",
            StatementKind::TypeDef(_) => "type",
            StatementKind::UseBlock(_) => "useBlock",
            StatementKind::Variable(_) => "variable",
            StatementKind::Function(_) => "function",
            StatementKind::Assignment(_) => "assignment",
            StatementKind::Expression { .. } => "expression",
            StatementKind::ExternalFunction(_) => "externalFunction",
        }
    }
}
