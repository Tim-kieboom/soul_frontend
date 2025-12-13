use crate::{abstract_syntax_tree::{expression::Expression, function::{Function, FunctionSignature}, soul_type::{GenericDeclare, SoulType}, spanned::Spanned, statment::{Ident, UseBlock}}, scope::scope::ScopeId};

/// A struct type definition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    /// The name of the struct.
    pub name: Ident,
    /// Generic type parameters.
    pub generics: Vec<GenericDeclare>,
    /// The fields of the struct.
    pub fields: Vec<Spanned<Field>>,
    /// The scope identifier for this struct.
    pub scope_id: ScopeId,
}

/// A class type definition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Class {
    /// The name of the class.
    pub name: Ident,
    /// Generic type parameters.
    pub generics: Vec<GenericDeclare>,
    /// The members of the class (fields, methods, impl blocks).
    pub members: Vec<Spanned<ClassChild>>,
    /// The scope identifier for this class.
    pub scope_id: ScopeId,
}

/// A trait definition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Trait {
    /// The trait's signature.
    pub signature: TraitSignature,
    /// The method signatures defined in this trait.
    pub methods: Vec<Spanned<FunctionSignature>>,
    /// The scope identifier for this trait.
    pub scope_id: ScopeId,
}

/// The signature of a trait.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TraitSignature {
    /// The name of the trait.
    pub name: Ident,
    /// Generic type parameters.
    pub generics: Vec<GenericDeclare>,
    /// Traits that this trait implements/extends.
    pub implements: Vec<SoulType>,
    pub for_types: Vec<SoulType>,
}

/// A child element of a class (field, method, or impl block).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ClassChild {
    /// A field definition.
    Field(Field),
    /// A method definition.
    Method(Function),
    /// An implementation block.
    ImplBlock(UseBlock),
}

/// A field definition in a struct or class.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Field {
    /// The name of the field.
    pub name: Ident,
    /// The type of the field.
    pub ty: SoulType,
    /// Optional default value for the field.
    pub default_value: Option<Expression>,
    /// Field access visibility settings.
    pub vis: FieldAccess,
    pub allignment: u32,
}

/// Field access visibility and mutability settings.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldAccess {
    /// Getter visibility. `None` means use default (e.g., public).
    pub get: Option<Visibility>, 
    /// Setter visibility. `None` means disallow setting.
    pub set: Option<Visibility>, 
}

/// Visibility modifier for fields and methods.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Visibility {
    /// Public visibility (accessible from anywhere).
    Public,
    /// Private visibility (only accessible within the same module/type).
    Private,
}

impl FieldAccess {
    pub const PUBLIC_GET: &str = "Get";
    pub const PRIVATE_GET: &str = "get";
    pub const PUBLIC_SET: &str = "Set";
    pub const PRIVATE_SET: &str = "set";

    pub fn inner_display(&self, sb: &mut String) {
        
        if let Some(get) = &self.get {

            let str = match get {
                Visibility::Public => Self::PUBLIC_GET,
                Visibility::Private => Self::PRIVATE_GET,
            };

            sb.push_str(str);
        }

        if let Some(set) = &self.set {
            if self.get.is_some() {
                sb.push(' ');
            }

            let str = match set {
                Visibility::Public => Self::PUBLIC_SET,
                Visibility::Private => Self::PRIVATE_SET,
            };

            sb.push_str(str);
        }
    }
}
