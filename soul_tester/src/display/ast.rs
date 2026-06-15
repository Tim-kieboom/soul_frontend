use std::{backtrace::Backtrace, fmt::{Arguments, Debug}, path::Path};

use anyhow::Result;
use ast_model::{AbstractSyntaxTree, AstStore, FunctionKind, block::BlockId, expression::{AnyArray, ExpressionId, ExpressionKind, ForCondition, IfBranch, MatchPattern}, soul_type::{ArrayKind, Generic, SoulType}, statements::{Assignment, Enum, ImplBlock, Import, ImportItem, ImportKind, StatementId, StatementKind, Struct, Trait, TypeDef, UseBlock, Variable}};
use soul_tokenizer::model::keyword::KeyWord;
use soul_utils::{FunctionId, TypeModifier, collections::vec_map::{VecMap, VecMapIndex}, soul_names::{PrimitiveTypes, Symbol}, span::ModuleId};
use crate::display::writer::Writer;

const IF_STR: &str = KeyWord::If.as_str();
const NEW_STR: &str = KeyWord::New.as_str();
const FOR_STR: &str = KeyWord::For.as_str();
const USE_STR: &str = KeyWord::Use.as_str();
const ELSE_STR: &str = KeyWord::Else.as_str();
const IMPL_STR: &str = KeyWord::Impl.as_str();
const TYPE_STR: &str = KeyWord::Type.as_str();
const ENUM_STR: &str = KeyWord::Enum.as_str();
const MATCH_STR: &str = KeyWord::Match.as_str();
const TRAIT_STR: &str = KeyWord::Trait.as_str();
const TYPEOF_STR: &str = KeyWord::Typeof.as_str();
const SIZEOF_STR: &str = KeyWord::Sizeof.as_str();
const STRUCT_STR: &str = KeyWord::Struct.as_str();
const IMPORT_STR: &str = KeyWord::Import.as_str();
const DISTINCT_STR: &str = KeyWord::Distinct.as_str();
const IN_FOR_LOOP_STR: &str = KeyWord::InForLoop.as_str();
const LAMDA_ARROW_STR: &str = Symbol::LambdaArrow.as_str();
struct Displayer<'a, W: Writer> {
    writer: &'a mut W,
    depth: String,

    root_dir: &'a Path,
    store: &'a AstStore,
    ast: &'a AbstractSyntaxTree, 
}

pub(crate) fn display_ast_tree<'a>(ast: &AbstractSyntaxTree, root_dir: &Path, store: &AstStore, writer: &mut impl Writer) -> Result<()> {
    let mut displayer = Displayer::new(ast, root_dir, store, writer);
    displayer.write_module(ast.root)
}

impl<'a, W: Writer> Displayer<'a, W> {
    fn new(ast: &'a AbstractSyntaxTree, root_dir: &'a Path, store: &'a AstStore, writer: &'a mut W) -> Self {
        Self { writer, root_dir, depth: String::new(), store, ast }
    }
    
    fn write_module(&mut self, id: ModuleId) -> Result<()> {
        
        let module = self.ast.modules.as_vecmap().get_err(id)?;
        self.write_fmt(format_args!("mod {} {{\n", module.name))?;
        let block = self.store.blocks.get_err(module.global)?;
    
        self.push_depth();
        for id in &block.statements {
            self.write_depth()?;
            self.write_statement(*id)?;
            self.write_endln()?;
        }

        for id in module.modules.entries() {
            self.write_depth()?;
            self.write_module(id)?;
            self.write_endln()?;
        }
        self.pop_depth();
        self.write_char('}')?;
        Ok(())
    }

    fn write_block(&mut self, id: BlockId) -> Result<()> {
        let block = self.store.blocks.get_err(id)?;
        if block.modifier != TypeModifier::Mut {
            self.write_str(block.modifier.as_str())?;
            self.write_char(' ')?;
        }
        self.write_str("{\n")?;
        self.push_depth();
        for id in &block.statements {
            self.write_depth()?;
            self.write_statement(*id)?;
            self.write_endln()?;
        }

        self.pop_depth();
        self.write_depth()?;
        self.write_char('}')?;
        Ok(())
    }

    fn write_statement(&mut self, id: StatementId) -> Result<()> {
        let statement = self.store.statements.get_err(id)?;
        match &statement.node {
            StatementKind::Enum(enum_) => self.write_enum(enum_),
            StatementKind::Trait(trait_) => self.write_trait(trait_),
            StatementKind::Import(import) => self.write_import(import),
            StatementKind::Struct(struct_) => self.write_struct(struct_),
            StatementKind::TypeDef(type_def) => self.write_typedef(type_def),
            StatementKind::Variable(variable) => self.write_variable(variable),
            StatementKind::UseBlock(use_block) => self.write_use_block(use_block),
            StatementKind::Function(function_id) => self.write_any_function(*function_id),
            StatementKind::Assignment(assignment) => self.write_assignment(assignment),
            StatementKind::ExternalFunction(function_id) => self.write_any_function(*function_id),
            StatementKind::Expression { expression, ends_semicolon, .. } => {
                self.write_expression(*expression)?;
                if *ends_semicolon {
                    self.write_char(';')?;
                }
                Ok(())
            }
        }
    }

    fn write_assignment(&mut self, assignment: &Assignment) -> Result<()> {
        self.write_expression(assignment.left)?;
        self.write_str(" = ")?;
        self.write_expression(assignment.right)
    }

    fn write_use_block(&mut self, use_block: &UseBlock) -> Result<()> {
        self.write_str(USE_STR)?;
        self.write_generic_defines(&use_block.use_generics)?;
        self.write_char(' ')?;
        self.write_type(&use_block.ty)?;
        self.write_generic_types(&use_block.type_generics)?;
        self.write_str(" {\n")?;
        self.push_depth();
        for methode in &use_block.methodes {
            self.write_depth()?;
            self.write_any_function(*methode)?;
            self.write_endln()?;
        }
        for impl_ in &use_block.impls {
            self.write_depth()?;
            self.write_impl(impl_)?;
            self.write_endln()?;
        }
        self.pop_depth();
        self.write_depth()?;
        self.write_char('}')
    }

    fn write_impl(&mut self, impl_: &ImplBlock) -> Result<()> {
        self.write_str(IMPL_STR)?;
        self.write_char(' ')?;
        self.write_type(&impl_.impl_trait)?;
        self.write_generic_types(&impl_.type_generics)?;
        self.write_str(" {\n")?;
        self.push_depth();
        for methode in &impl_.methodes {
            self.write_depth()?;
            self.write_any_function(*methode)?;
            self.write_endln()?;
        }
        self.pop_depth();
        self.write_depth()?;
        self.write_char('}')
    }

    fn write_variable(&mut self, variable: &Variable) -> Result<()> {
        self.write_fmt(format_args!("{} {}", variable.modifier.as_str(), variable.name.as_str()))?;
        if let Some(ty) = &variable.ty {
            self.write_str(": ")?;
            self.write_type(ty)?;
        }

        if let Some(value) = variable.initialize_value {
            self.write_str(" = ")?;
            self.write_expression(value)?;
        }

        Ok(())
    }

    fn write_typedef(&mut self, type_def: &TypeDef) -> Result<()> {
        self.write_str(TYPE_STR)?;
        self.write_char(' ')?;
        self.write_type(&type_def.new_type)?;
        self.write_str(" = ")?;
        if type_def.is_distinct {
            self.write_fmt(format_args!("{DISTINCT_STR} "))?;
        }
        self.write_type(&type_def.old_type)
    }

    fn write_struct(&mut self, struct_: &Struct) -> Result<()> {
        self.write_fmt(format_args!("{STRUCT_STR} {}", struct_.name.as_str()))?;
        self.write_generic_defines(&struct_.generics)?;
        self.write_str("{\n")?;
        self.push_depth();
        self.write_depth()?;
        for field in &struct_.fields {
            if field.is_pubic {
                self.write_str("pub ")?;
            }

            self.write_fmt(format_args!("{} {}: ", field.modifier.as_str(), field.name.as_str()))?;
            self.write_type(&field.ty)?;
            if let Some(value) = &field.default {
                self.write_str(" = ")?;
                self.write_expression(*value)?;
            }
        }

        for methode in &struct_.methods {
            self.write_any_function(*methode)?;
        }
        self.pop_depth();
        self.write_depth()?;
        self.write_char('}')?;
        Ok(())
    }

    fn write_import(&mut self, import: &Import) -> Result<()> {
        self.write_fmt(format_args!("{IMPORT_STR} (\n"))?;
        self.push_depth();
        for path in &import.paths {
            self.write_depth()?;
            self.write_str(&path.module.display(self.root_dir)?)?;
            match &path.kind {
                ImportKind::This => self.write_str("this")?,
                ImportKind::Glob => self.write_char('*')?,
                ImportKind::Alias(ident) => self.write_fmt(format_args!(" as {}", ident.as_str()))?,
                ImportKind::Module => (),
                ImportKind::Items { this, this_alias, items } => {
                    self.write_char('{')?;
                    if *this {
                        self.write_str("this")?;
                        if let Some(alias) = &this_alias {
                            self.write_fmt(format_args!(" as {}", alias.as_str()))?;
                        }
                    }

                    let last_index = items.len().saturating_sub(1);
                    for (i, item) in items.iter().enumerate() {
                        match item {
                            ImportItem::Normal(ident) => self.write_str(ident.as_str())?,
                            ImportItem::Alias { name, alias } => self.write_fmt(format_args!("{} as {}", name.as_str(), alias.as_str()))?,
                        }
                        if i != last_index {
                            self.write_str(", ")?;
                        }
                    }
                    self.write_char('}')?;
                }
            }
            self.write_endln()?;
        }
        self.pop_depth();
        self.write_depth()?;
        self.write_char(')')?;
        Ok(())
    }

    fn write_enum(&mut self, enum_: &Enum) -> Result<()> {
        self.write_fmt(format_args!("{ENUM_STR} {} {{\n", enum_.name.as_str()))?;
        self.push_depth();
        let last_index = enum_.variants.len().saturating_sub(1);
        for (i, variant) in enum_.variants.iter().enumerate() {
            self.write_depth()?;
            self.write_str(variant.as_str())?;
            if i != last_index {
                self.write_char(',')?;
            }
            self.write_endln()?;
        }
        self.pop_depth();
        self.write_depth()?;
        self.write_str("}\n")
    }

    fn write_trait(&mut self, trait_: &Trait) -> Result<()> {
        self.write_fmt(format_args!("{TRAIT_STR} {} {{\n", trait_.name.as_str()))?;
        self.push_depth();
        
        for methode in &trait_.methods {
            self.write_depth()?;
            self.write_any_function(*methode)?;
            self.write_endln()?;
        }
        self.pop_depth();
        self.write_depth()?;
        self.write_str("}\n")
    }

    fn write_any_function(&mut self, id: FunctionId) -> Result<()> {
        let function = self.store.functions.get_err(id)?;
        let signature = &function.signature().value;

        if let Some(external) = signature.external {
            self.write_fmt(format_args!("{} {} ", KeyWord::Extern.as_str(), external.as_str()))?;
        }

        if signature.is_public {
            self.write_str("pub ")?;
        }
        self.write_str(signature.modifier.as_str())?;
        self.write_char(' ')?;
        if let Some(kind) = signature.function_kind.display() {
            self.write_fmt(format_args!(" {kind} "))?;
        }
        self.write_str(signature.name.as_str())?;
        self.write_generic_defines(&signature.generics)?;
        self.write_char('(')?;
        let last_index = signature.parameters.len().saturating_sub(1);
        for (i, parameter) in signature.parameters.iter().enumerate() {
            self.write_fmt(format_args!("{} {}: ", parameter.modifier.as_str(), parameter.name.as_str()))?;
            self.write_type(&parameter.ty)?;
            if let Some(value) = parameter.default {
                self.write_expression(value)?;
            }
            if i != last_index {
                self.write_str(", ")?;
            }
        }
        self.write_str("): ")?;
        self.write_type(&signature.return_type)?;
        let block = match function {
            FunctionKind::Normal(function) => function.block,
            FunctionKind::External(_) => return Ok(()),
        };
        self.write_char(' ')?;
        self.write_block(block)
    }

    fn write_expression(&mut self, id: ExpressionId) -> Result<()> {
        let expression = self.store.expressions.get_err(id)?;
        match &expression.node {
            ExpressionKind::Null(_) => self.write_str("null"),
            ExpressionKind::Default(_) => self.write_str("()"),
            ExpressionKind::Literal((_, literal)) => self.write_fmt(format_args!("{literal:?}")),
            ExpressionKind::Index(index) => {
                self.write_expression(index.collection)?;
                self.write_char('[')?;
                self.write_expression(index.index)?;
                self.write_char(']')
            }
            ExpressionKind::FieldAccess(field_access) => {
                self.write_expression(field_access.object)?;
                self.write_char('.')?;
                self.write_str(field_access.field.as_str())
            }
            ExpressionKind::FunctionCall(function_call) => {
                self.write_str(function_call.name.as_str())?;
                self.write_generic_types(&function_call.generics)?;
                self.write_char('(')?;
                let last_index = function_call.arguments.len().saturating_sub(1);
                for (i, arg) in function_call.arguments.iter().enumerate() {
                    if let Some(name) = &arg.name {
                        self.write_fmt(format_args!("{name}: "))?;
                    }
                    self.write_expression(arg.value)?;
                    if i != last_index {
                        self.write_str(", ")?;
                    }
                }
                self.write_char(')')
            }
            ExpressionKind::Constructor(constructor) => {
                self.write_type(&constructor.ty)?;
                self.write_str(".(")?;
                let last_index = constructor.arguments.len().saturating_sub(1);
                for (i, arg) in constructor.arguments.iter().enumerate() {
                    if let Some(name) = &arg.name {
                        self.write_fmt(format_args!("{name}: "))?;
                    }
                    self.write_expression(arg.value)?;
                    if i != last_index {
                        self.write_str(", ")?;
                    }
                }
                self.write_char(')')
            }
            ExpressionKind::StructConstructor(ctor) => {
                self.write_type(&ctor.struct_type)?;
                let last_index = ctor.values.len().saturating_sub(1);
                self.write_char('{')?;
                for (i, arg) in ctor.values.iter().enumerate() {
                    self.write_fmt(format_args!("{}: ", arg.0))?;
                    self.write_expression(arg.1)?;
                    if i != last_index {
                        self.write_str(", ")?;
                    }
                } 
                self.write_char('}')
            }
            ExpressionKind::Variable(variable) => {
                self.write_str(variable.name.as_str())
            }
            ExpressionKind::Array(any_array) => {
                self.write_any_array(any_array)
            },
            ExpressionKind::Sizeof(soul_type) => {
                self.write_type(soul_type)?;
                self.write_fmt(format_args!(".{SIZEOF_STR}"))
            }
            ExpressionKind::New(expression_id) => {
                self.write_fmt(format_args!("{NEW_STR}("))?;
                self.write_expression(*expression_id)?;
                self.write_char(')')
            }
            ExpressionKind::NewArray(any_array) => {
                self.write_str(NEW_STR)?;
                self.write_any_array(any_array)
            }
            ExpressionKind::Unary(unary) => {
                self.write_str(unary.operator.value.as_str())?;
                self.write_expression(unary.value)
            }
            ExpressionKind::Binary(binary) => {
                self.write_expression(binary.left)?;
                self.write_char(' ')?;
                self.write_str(binary.operator.value.as_str())?;
                self.write_char(' ')?;
                self.write_expression(binary.right)
            }
            ExpressionKind::Ref(ref_) => {
                self.write_char('&')?;
                if ref_.is_mutable {
                    self.write_str("mut ")?;
                }
                self.write_expression(ref_.value)
            }
            ExpressionKind::Deref(deref) => {
                self.write_char('*')?;
                self.write_expression(deref.value)
            }
            ExpressionKind::If(if_) => {
                self.write_str(IF_STR)?;
                self.write_char(' ')?;
                self.write_expression(if_.condition)?;
                self.write_block(if_.block)?;
                self.display_branch(&if_.branch)
            }
            ExpressionKind::Match(match_) => {
                self.write_fmt(format_args!("{MATCH_STR} "))?;
                self.write_expression(match_.scrutinee)?;
                self.write_str(" {\n")?;
                self.push_depth();
                for arm in &match_.arms {
                    self.write_match_pattern(&arm.pattern)?;
                    self.write_fmt(format_args!(" {LAMDA_ARROW_STR} "))?;
                    self.write_block(arm.body)?;
                }
                self.pop_depth();
                Ok(())
            }
            ExpressionKind::MatchMethod(match_methode) => {
                self.write_expression(match_methode.scrutinee)?;
                self.push_depth();                
                let last_index = match_methode.arms.len().saturating_sub(1);
                for (i, arm) in match_methode.arms.iter().enumerate() {
                    self.write_fmt(format_args!(".{}{{", arm.variant_name.as_str()))?;
                    if let Some(binding) = &arm.binding {
                        self.write_fmt(format_args!("{} {LAMDA_ARROW_STR}", binding.ident.as_str()))?;
                    }
                    self.write_block(arm.body)?;
                    if i != last_index {
                        self.write_endln()?;
                    }
                }
                self.pop_depth();
                Ok(())
            }
            ExpressionKind::For(for_) => {
                self.write_fmt(format_args!("{FOR_STR} "))?;
                match &for_.condition {
                    ForCondition::Loop => (),
                    ForCondition::While(condition) => self.write_expression(*condition)?,
                    ForCondition::Foreach { element, index, collection } => {
                        if let Some(index) = index {
                            self.write_fmt(format_args!("{}, ", index.as_str()))?;
                        }
                        self.write_fmt(format_args!("{} {IN_FOR_LOOP_STR} ", element.as_str()))?;
                        self.write_expression(*collection)?;
                    }
                }
                self.write_block(for_.block)
            }
            ExpressionKind::Block(block_id) => self.write_block(*block_id),
            ExpressionKind::TypeOf(type_of) => {
                self.write_expression(type_of.value)?;
                self.write_fmt(format_args!(" {TYPEOF_STR} "))?;
                self.write_fmt(format_args!("{}.{}", type_of.type_name.as_str(), type_of.variant_name.as_str()))?;
                if let Some(binding) = &type_of.binding {
                    self.write_fmt(format_args!("({})", binding.as_str()))?;
                }
                Ok(())
            }
            ExpressionKind::Break => self.write_str(KeyWord::Break.as_str()),
            ExpressionKind::Continue => self.write_str(KeyWord::Continue.as_str()),
            ExpressionKind::Return(value) => {
                self.write_str(KeyWord::Return.as_str())?;
                self.write_char(' ')?;
                if let Some(value) = value {
                    self.write_expression(*value)?;
                }
                Ok(())
            }
        }
    }

    fn write_match_pattern(&mut self, arm: &MatchPattern) -> Result<()> {
        match &arm {
            MatchPattern::Wildcard => self.write_str("_"),
            MatchPattern::Literal(literal) => self.write_fmt(format_args!("{literal:?}")),
            MatchPattern::Binding(binding) => self.write_str(binding.ident.as_str()),
            MatchPattern::Array(match_patterns) => {
                self.write_char('[')?;
                let last_index = match_patterns.len().saturating_sub(1);
                for (i, arm) in match_patterns.iter().enumerate() {
                    self.write_match_pattern(arm)?;
                    if i != last_index {
                        self.write_str(", ")?;
                    }
                }
                self.write_char(']')
            }
            MatchPattern::Constructor(ctor) => {
                self.write_fmt(format_args!("{}.{}", ctor.type_name.as_str(), ctor.variant_name.as_str()))?;
                if let Some(binding) = &ctor.binding {
                    self.write_fmt(format_args!("({})", binding.as_str()))?;
                }
                Ok(())
            }
        }
    }

    fn display_branch(&mut self, if_arm: &Option<Box<IfBranch>>) -> Result<()> {
        let mut current = if_arm.as_ref();
        while let Some(arm) = current {
            self.write_depth()?;
            match arm.as_ref() {
                IfBranch::If(elif) => {
                    self.write_fmt(format_args!("{ELSE_STR} {IF_STR} "))?;
                    self.write_expression(elif.condition)?;
                    self.write_block(elif.block)?;
                    current = elif.branch.as_ref();
                }
                IfBranch::Else(el) => {
                    self.write_fmt(format_args!("{ELSE_STR} "))?;
                    self.write_block(*el)?;
                    current = None;
                }
            }
        }

        Ok(())
    }

    fn write_type(&mut self, ty: &SoulType) -> Result<()> {
        match ty {
            SoulType::None => self.write_str(PrimitiveTypes::None.as_str()),
            SoulType::Never => self.write_char('!'),
            SoulType::Primitive(primitive_types) => self.write_str(primitive_types.as_str()),
            SoulType::Array(array) => {
                match array.kind {
                    ArrayKind::StackArray(num) => self.write_fmt(format_args!("[{num}]"))?,
                    ArrayKind::HeapArray => self.write_str("[]")?,
                    ArrayKind::MutSlice => self.write_str("[&mut]")?,
                    ArrayKind::ConstSlice => self.write_str("[&]")?,
                }
                self.write_type(&array.of_type)
            },
            SoulType::Reference(reference) => {
                self.write_char('&')?;
                if let Some(lifetime) = &reference.lifetime {
                    self.write_fmt(format_args!("'{} ", lifetime.as_str()))?;
                }
                if reference.mutable {
                    self.write_str("mut")?;
                }
                self.write_type(&reference.inner)
            }
            SoulType::Pointer(soul_type) => {
                self.write_char('*')?;
                self.write_type(&soul_type)
            }
            SoulType::Optional(soul_type) => {
                self.write_char('?')?;
                self.write_type(&soul_type)
            }
            SoulType::Stub(stub) => {
                self.write_str(&stub.name)?;
                self.write_generic_types(&stub.generics)?;
                Ok(())
            },
            SoulType::NamedVariant { base, variant } => {
                self.write_type(base)?;
                self.write_fmt(format_args!(".{}", variant.as_str()))
            }
        }
    }

    fn write_any_array(&mut self, any_array: &AnyArray) -> Result<()> {
        match any_array {
            AnyArray::Array(array) => {
                if let Some(ty) = &array.collection_type {
                    self.write_type(ty)?;
                    self.write_char('.')?;
                }
                self.write_char('[')?;
                if let Some(ty) = &array.element_type {
                    self.write_type(ty)?;
                    self.write_str(": ")?;
                }
                let last_index = array.values.len().saturating_sub(1);
                for (i, value) in array.values.iter().enumerate() {
                    self.write_expression(*value)?;
                    if i != last_index {
                        self.write_str(", ")?;
                    }
                }
                self.write_char(']')
            }
            AnyArray::ArrayFiller(array) => {
                if let Some(collection) = &array.collection_type {
                    self.write_type(collection)?;
                    self.write_char('.')?;
                }
                self.write_char('[')?;
                if let Some(ty) = &array.element_type {
                    self.write_type(ty)?;
                    self.write_str(": ")?;
                }
                self.write_fmt(format_args!("{FOR_STR} "))?;
                if let Some(name) = &array.for_index {
                    self.write_fmt(format_args!("{} {IN_FOR_LOOP_STR} ", name.as_str()))?;
                }
                self.write_expression(array.amount)?;
                self.write_fmt(format_args!(" {LAMDA_ARROW_STR} "))?;
                self.write_expression(array.element)?;
                self.write_char(']')
            }
        }
    }

    fn write_generic_defines(&mut self, generics: &Vec<Generic>) -> Result<()> {
        if generics.is_empty() {
            return Ok(())
        }
        
        self.write_char('<')?;
        let last_index = generics.len().saturating_sub(1);
        for (i, generic) in generics.iter().enumerate() {
            self.write_str(generic.name.as_str())?;
            if let Some(bound) = &generic.bound {
                self.write_str(": ")?;
                self.write_type(bound)?;
            }
            
            if i != last_index {
                self.write_str(", ")?;
            }
        }
        self.write_char('>')
    }

    fn write_generic_types(&mut self, generics: &Vec<SoulType>) -> Result<()> {
        if generics.is_empty() {
            return Ok(())
        }
        
        self.write_char('<')?;
        let last_index = generics.len().saturating_sub(1);
        for (i, generic) in generics.iter().enumerate() {
            self.write_type(generic)?;
            if i != last_index {
                self.write_str(", ")?;
            }
        }
        self.write_char('>')
    }

    fn write_endln(&mut self) -> Result<()> {
        self.writer.push_char('\n')
    }

    fn write_depth(&mut self) -> Result<()> {
        self.writer.push_str(&self.depth)
    }

    fn push_depth(&mut self) {
        self.depth.push('\t');
    }

    fn pop_depth(&mut self) {
        self.depth.pop();
    }

    fn write_fmt(&mut self, args: Arguments<'_>) -> Result<()> {
        self.writer.push_fmt(args)
    }

    fn write_str(&mut self, str: &str) -> Result<()> {
        self.writer.push_str(str)
    }

    fn write_char(&mut self, ch: char) -> Result<()> {
        self.writer.push_char(ch)
    }
}

trait GetErr<I, V> {
    fn get_err(&self, index: I) -> Result<&V>;
}
impl<I: VecMapIndex + Debug + Clone, V> GetErr<I, V> for VecMap<I, V> {
    fn get_err(&self, index: I) -> Result<&V> {
        self.get(index.clone()).ok_or(anyhow::Error::msg(format!("{index:?} is not found; {}", Backtrace::force_capture().to_string())))
    }
}