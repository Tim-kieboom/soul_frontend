use crate::{
    AstStore, NodeId,
    block::BlockId,
    expression::{Expression, ExpressionId, ExpressionKind, FunctionCall},
    soul_type::{Generic, SoulType},
};
use soul_utils::{
    FunctionId, Ident, TypeModifier,
    collections::soul_import_path::SoulImportPath,
    error::SoulResult,
    fault::Fault,
    impl_soul_ids,
    span::{ItemMetaData, Span, Spanned},
};

impl_soul_ids!(StatementId);

/// A statement in the Soul language, wrapped with source location information.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Statement {
    pub node: StatementKind,
    pub span: Span,
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
        id: Option<NodeId>,
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
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TypeDef {
    pub id: Option<NodeId>,
    pub new_type: SoulType,
    pub old_type: SoulType,
    pub is_distinct: bool,
}

/// A trait definition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Trait {
    pub id: Option<NodeId>,
    pub name: Ident,
    pub generics: Vec<Generic>,
    pub methods: Vec<FunctionId>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Enum {
    pub name: Ident,
    pub id: Option<NodeId>,
    pub variants: Vec<EnumVariant>,
    pub impl_type: Option<SoulType>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EnumVariant {
    Normal(Ident),
    Assigned {
        name: Ident,
        value: ExpressionId,
    },
    Union {
        name: Ident,
        parameters: Vec<Parameter>,
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    pub id: Option<NodeId>,
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
    pub value: FunctionId,
    pub is_public: bool,
}
impl Methode {
    pub fn new(value: FunctionId, is_public: bool) -> Self {
        Self { value, is_public }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UseBlock {
    pub use_generics: Vec<Generic>,
    pub ty: SoulType,
    pub impls: Vec<ImplBlock>,
    pub methodes: Vec<Methode>,
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
    pub id: Option<NodeId>,
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
            module: SoulImportPath::new(),
            kind: ImportKind::This,
            lib_name: None,
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
        this: bool,
        this_alias: Option<Ident>,
        items: Vec<ImportItem>,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImportItem {
    Normal(Ident),
    Alias { name: Ident, alias: Ident },
}

/// A variable declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    pub node_id: Option<NodeId>,
    /// The name of the variable.
    pub name: Ident,
    /// The modifier of the variable.
    pub modifier: TypeModifier,
    /// The type of the variable.
    pub ty: Option<SoulType>,
    /// Optional initial value expression.
    pub initialize_value: Option<ExpressionId>,
}

/// An assignment statement, e.g., `x = y + 1`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Assignment {
    pub node_id: Option<NodeId>,
    /// The left-hand side expression (the variable being assigned to).
    pub left: ExpressionId,
    /// The right-hand side expression (the value being assigned).
    pub right: ExpressionId,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExternalFunction {
    /// The function's signature (name, parameters, return type, etc.).
    pub signature: Spanned<FunctionSignature>,
}

/// A function definition with a signature and body block.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Function {
    /// The function's signature (name, parameters, return type, etc.).
    pub signature: Spanned<FunctionSignature>,
    /// The function's body block.
    pub block: BlockId,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FunctionSignature {
    pub id: FunctionId,

    pub modifier: TypeModifier,
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
    pub name: Ident,
    pub ty: SoulType,
    pub modifier: TypeModifier,
    pub node_id: Option<NodeId>,
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
    /// `&this`
    MutRef,
    /// ``
    Static,
    /// `this`
    Consume,
    /// `@this`
    ConstRef,
}
impl FunctionThisKind {
    pub fn display(&self) -> Option<&'static str> {
        match self {
            FunctionThisKind::Static => None,
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

    pub fn from_expression(
        store: &AstStore,
        expression: ExpressionId,
        ends_semicolon: bool,
    ) -> Self {
        let span = store.expressions[expression].span;
        Self::new(
            StatementKind::Expression {
                id: None,
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
                id: None,
                ends_semicolon,
                expression: store.insert_expression(expression),
            },
            span,
        )
    }

    pub fn is_public(&self) -> bool {
        self.is_public
    }

    pub fn try_set_is_public(&mut self, is_public: bool, span: Span) -> SoulResult<()> {
        match self.node {
            StatementKind::Enum(_)
            | StatementKind::Trait(_)
            | StatementKind::Struct(_)
            | StatementKind::TypeDef(_)
            | StatementKind::Variable(_)
            | StatementKind::Function(_)
            | StatementKind::ExternalFunction(_) => self.is_public = is_public,

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
