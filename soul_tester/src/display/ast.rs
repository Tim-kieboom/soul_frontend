use std::{
    backtrace::Backtrace,
    fmt::{Arguments, Debug},
    path::Path,
};

use crate::display::writer::Writer;
use anyhow::Result;
use ast_model::{
    AstStore, AstTree, FunctionKind, block::BlockId, expression::{AnyArray, ExpressionId, ExpressionKind, ForCondition, ForElementKind, IfBranch, Lambda, MatchPattern, TypeofKind}, soul_type::{ArrayKind, Generic, SoulType, TupleKind}, statements::{
        Assignment, Enum, EnumVariant, ImplBlock, Import, ImportItem, ImportKind, Parameter, StatementId, StatementKind, Struct, Trait, TypeDef, UnionKind, UseBlock, VarPattern, Variable
    },
};
use soul_tokenizer::model::{keyword::KeyWord, types::Types};
use soul_utils::{
    FunctionId, TypeModifier, collections::vec_map::{VecMap, VecMapIndex}, ids::IdAlloc, soul_names::{PrimitiveTypes, Symbol}, span::ModuleId
};

const IF_STR: &str = KeyWord::If.as_str();
const NEW_STR: &str = KeyWord::New.as_str();
const FOR_STR: &str = KeyWord::For.as_str();
const USE_STR: &str = KeyWord::Use.as_str();
const ELSE_STR: &str = KeyWord::Else.as_str();
const IMPL_STR: &str = KeyWord::Impl.as_str();
const TYPE_STR: &str = KeyWord::Type.as_str();
const ENUM_STR: &str = KeyWord::Enum.as_str();
const PASS_STR: &str = KeyWord::Pass.as_str();
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
    ast: &'a AstTree,
}

pub(crate) fn display_ast_tree<'a>(
    ast: &AstTree,
    root_dir: &Path,
    writer: &mut impl Writer,
) -> Result<()> {
    let mut displayer = Displayer::new(ast, root_dir, &ast.store, writer);
    displayer.write_module(ast.root)?;
    writer.writer_flush()
}

impl<'a, W: Writer> Displayer<'a, W> {
    fn new(
        ast: &'a AstTree,
        root_dir: &'a Path,
        store: &'a AstStore,
        writer: &'a mut W,
    ) -> Self {
        Self {
            writer,
            root_dir,
            depth: String::new(),
            store,
            ast,
        }
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
        self.write_depth()?;
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
        if statement.is_public() {
            self.write_str("pub ")?;
        }

        match &statement.node {
            StatementKind::Enum(enum_) => self.write_enum(enum_),
            StatementKind::Trait(trait_) => self.write_trait(trait_),
            StatementKind::Import(import) => self.write_import(import),
            StatementKind::Struct(struct_) => self.write_struct(struct_),
            StatementKind::TypeDef(type_def) => self.write_typedef(type_def),
            StatementKind::Variable(variable) => self.write_variable(variable),
            StatementKind::UseBlock(use_block) => self.write_use_block(use_block),
            StatementKind::Function(function) => self.write_any_function(*function),
            StatementKind::Assignment(assignment) => self.write_assignment(assignment),
            StatementKind::ExternalFunction(external_function) => {
                self.write_any_function(*external_function)
            }
            StatementKind::Expression {
                expression,
                ends_semicolon,
                ..
            } => {
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
        self.write_str(" {\n")?;
        self.push_depth();
        for id in &use_block.statements {
            self.write_depth()?;
            if self.store.statements[*id].is_public() {
                self.write_str("pub ")?;
            }

            self.write_statement(*id)?;
            self.write_endln()?;
        }

        for method in &use_block.methods {
            self.write_depth()?;
            if method.is_public {
                self.write_str("pub ")?;
            }

            self.write_any_function(method.id)?;
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
        self.write_str(" {\n")?;
        self.push_depth();
        for methode in &impl_.methods {
            self.write_depth()?;
            self.write_any_function(*methode)?;
            self.write_endln()?;
        }
        self.pop_depth();
        self.write_depth()?;
        self.write_char('}')
    }

    fn write_variable(&mut self, variable: &Variable) -> Result<()> {
        self.write_str(variable.modifier.as_str())?;
        self.write_char(' ')?;
        self.write_var_pattern(&variable.pattern)?;

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

    fn write_var_pattern(&mut self, pattern: &VarPattern) -> Result<()> {
        match pattern {
            VarPattern::Discard => self.write_str("_"),
            VarPattern::Simple { binding, modifier } => {
                if *modifier == TypeModifier::Mut {
                    self.write_str("mut ")?;
                }
                self.write_str(binding.ident.as_str())
            }
            VarPattern::Tuple(tuple) => {
                self.write_char('(')?;
                for (i, element) in tuple.elements.iter().enumerate() {
                    if i > 0 {
                        self.write_str(", ")?;
                    }
                    self.write_var_pattern(element)?;
                }
                if tuple.rest {
                    if !tuple.elements.is_empty() {
                        self.write_str(", ")?;
                    }
                    self.write_str("..")?;
                }
                self.write_char(')')
            }
            VarPattern::NamedTuple(named) => {
                self.write_char('{')?;
                for (i, field) in named.fields.iter().enumerate() {
                    if i > 0 {
                        self.write_str(", ")?;
                    }
                    if field.modifier == TypeModifier::Mut {
                        self.write_str("mut ")?;
                    }
                    self.write_str(field.field.as_str())?;
                    match &field.binding {
                        Some(binding) if binding.ident != field.field => {
                            self.write_str(": ")?;
                            self.write_str(binding.ident.as_str())?;
                        }
                        None => {
                            self.write_str(": _")?;
                        }
                        _ => {}
                    }
                }
                if named.rest {
                    if !named.fields.is_empty() {
                        self.write_str(", ")?;
                    }
                    self.write_str("..")?;
                }
                self.write_char('}')
            }
            VarPattern::Constructor(constructor) => {
                self.write_str(constructor.type_name.as_str())?;
                self.write_char('{')?;
                for (i, field) in constructor.fields.iter().enumerate() {
                    if i > 0 {
                        self.write_str(", ")?;
                    }
                    if field.modifier == TypeModifier::Mut {
                        self.write_str("mut ")?;
                    }
                    self.write_str(field.field.as_str())?;
                    match &field.binding {
                        Some(binding) if binding.ident != field.field => {
                            self.write_str(": ")?;
                            self.write_str(binding.ident.as_str())?;
                        }
                        None => {
                            self.write_str(": _")?;
                        }
                        _ => {}
                    }
                }
                if constructor.rest {
                    if !constructor.fields.is_empty() {
                        self.write_str(", ")?;
                    }
                    self.write_str("..")?;
                }
                self.write_char('}')
            }
        }
    }

    fn write_lambda(&mut self, lambda: &Lambda) -> Result<()> {
        if lambda.params.len() == 1 {
            self.write_var_pattern(&lambda.params[0])?;
        } else {
            self.write_char('(')?;
            for (i, param) in lambda.params.iter().enumerate() {
                if i > 0 {
                    self.write_str(", ")?;
                }
                self.write_var_pattern(param)?;
            }
            self.write_char(')')?;
        }
        self.write_fmt(format_args!(" {} ", LAMDA_ARROW_STR))?;
        self.write_expression(lambda.body)
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

        for field in &struct_.fields {
            self.write_depth()?;
            if field.is_public {
                self.write_str("pub ")?;
            }

            self.write_str(field.value.modifier.as_str())?;
            self.write_char(' ')?;
            self.write_var_pattern(&field.value.pattern)?;
            if let Some(ty) = &field.value.ty {
                self.write_str(": ")?;
                self.write_type(ty)?;
            }
            if let Some(value) = &field.value.initialize_value {
                self.write_str(" = ")?;
                self.write_expression(*value)?;
            }
            self.write_endln()?;
        }

        for statement in &struct_.statements {
            self.write_depth()?;
            self.write_statement(*statement)?;
            self.write_endln()?;
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
            self.write_str(&path.module.display(self.root_dir))?;
            match &path.kind {
                ImportKind::This => self.write_str("this")?,
                ImportKind::Glob => self.write_char('*')?,
                ImportKind::Alias(ident) => {
                    self.write_fmt(format_args!(" as {}", ident.as_str()))?
                }
                ImportKind::Module => (),
                ImportKind::Items {
                    this,
                    this_alias,
                    items,
                } => {
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
                            ImportItem::Alias { name, alias } => self.write_fmt(format_args!(
                                "{} as {}",
                                name.as_str(),
                                alias.as_str()
                            ))?,
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
        self.write_fmt(format_args!("{ENUM_STR} {}", enum_.name.as_str()))?;
        if let Some(ty) = &enum_.impl_type {
            self.write_str(": ")?;
            self.write_type(ty)?;
        }

        self.write_str(" {\n")?;
        self.push_depth();
        let last_index = enum_.variants.len().saturating_sub(1);
        for (i, variant) in enum_.variants.iter().enumerate() {
            self.write_depth()?;
            match variant {
                EnumVariant::Normal(ident) => self.write_str(ident.as_str())?,
                EnumVariant::Assigned { name, value } => {
                    self.write_fmt(format_args!("{} = ", name.as_str()))?;
                    self.write_expression(*value)?
                }
                EnumVariant::Union(union) => match union {
                    UnionKind::Tuple { name, parameters } => {
                        self.write_str(name.as_str())?;
                        self.write_char('(')?;
                        let last_index = parameters.len().saturating_sub(1);
                        for (i, ty) in parameters.iter().enumerate() {
                            self.write_type(ty)?;
                            if i != last_index {
                                self.write_str(", ")?;
                            }
                        }
                        self.write_char(')')?;
                    }
                    UnionKind::NamedTuple { name, parameters } => {
                        self.write_str(name.as_str())?;
                        self.write_char('{')?;
                        let last_index = parameters.len().saturating_sub(1);
                        for (i, (ident, ty)) in parameters.iter().enumerate() {
                            self.write_fmt(format_args!("{}: ", ident.as_str()))?;
                            self.write_type(ty)?;
                            if i != last_index {
                                self.write_str(", ")?;
                            }
                        }
                        self.write_char('}')?;
                    }
                }
            }
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
        for ty in &trait_.typedefs {
            self.write_depth()?;
            self.write_fmt(format_args!("{} ", KeyWord::Type.as_str()))?;
            self.write_type(ty)?;
            self.write_endln()?;
        }

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
            self.write_fmt(format_args!(
                "{} \"{}\" ",
                KeyWord::Extern.as_str(),
                external.as_str()
            ))?;
        }

        if signature.modifier != TypeModifier::Mut {
            self.write_fmt(format_args!("{} ", signature.modifier.as_str()))?;
        }
        self.write_str(signature.name.as_str())?;
        self.write_generic_defines(&signature.generics)?;
        self.write_char('(')?;
        if let Some(kind) = signature.function_kind.display() {
            self.write_str(kind)?;
            if !signature.parameters.is_empty() {
                self.write_str(", ")?;
            }
        }
        self.write_parameters(&signature.parameters)?;
        self.write_str("): ")?;
        self.write_type(&signature.return_type)?;
        let block = match function {
            FunctionKind::Normal(function) => function.block,
            FunctionKind::Signature(_) => return Ok(()),
        };
        self.write_char(' ')?;
        self.write_block(block)
    }

    fn write_parameters(&mut self, parameters: &[Parameter]) -> Result<()> {
        let last_index = parameters.len().saturating_sub(1);
        for (i, parameter) in parameters.iter().enumerate() {
            self.write_fmt(format_args!(
                "{} {}: ",
                parameter.modifier.as_str(),
                parameter.name.as_str()
            ))?;
            self.write_type(&parameter.ty)?;
            if let Some(value) = parameter.default {
                self.write_expression(value)?;
            }
            if i != last_index {
                self.write_str(", ")?;
            }
        }

        Ok(())
    }

    fn write_expression(&mut self, id: ExpressionId) -> Result<()> {
        if id == ExpressionId::error() {
            self.write_str("<error>")?;
            return Ok(())
        }
        
        let expression = self.store.expressions.get_err(id)?;
        match &expression.node {
            
            ExpressionKind::Null(_) => self.write_str("null"),
            ExpressionKind::None(_) => self.write_str("()"),
            ExpressionKind::Undefined(_) => self.write_str("undefined"),
            ExpressionKind::Literal((_, literal)) => self.write_fmt(format_args!("{literal:?}")),
            ExpressionKind::Copy(value) => {
                self.write_expression(*value)?;
                self.write_fmt(format_args!(".{}", KeyWord::Copy.as_str()))
            }
            ExpressionKind::Index(index) => {
                self.write_expression(index.collection)?;
                if index.optional_map {
                    self.write_char('?')?;
                }
                self.write_char('[')?;
                self.write_expression(index.index)?;
                self.write_char(']')
            }
            ExpressionKind::FieldAccess(field_access) => {
                self.write_expression(field_access.object)?;
                if field_access.optional_map {
                    self.write_char('?')?;
                }
                self.write_char('.')?;
                self.write_str(field_access.field.as_str())
            }
            ExpressionKind::FunctionCall(function_call) => {
                if let Some(callee) = function_call.callee {
                    self.write_expression(callee.value)?;
                    if callee.optional_map {
                        self.write_char('?')?;
                    }
                    self.write_char('.')?;    
                }

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
                    if i != last_index || ctor.defaults {
                        self.write_str(", ")?;
                    }
                }
                if ctor.defaults {
                    self.write_str("..")?;
                }
                self.write_char('}')
            }
            ExpressionKind::Variable(variable) => self.write_str(variable.name.as_str()),
            ExpressionKind::Array(any_array) => self.write_any_array(any_array),
            ExpressionKind::Sizeof(value) => {
                self.write_expression(*value)?;
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
                self.write_char('(')?;
                self.write_expression(binary.left)?;
                self.write_char(' ')?;
                self.write_str(binary.operator.value.as_str())?;
                self.write_char(' ')?;
                self.write_expression(binary.right)?;
                self.write_char(')')
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
                    self.write_depth()?;
                    self.write_match_pattern(&arm.pattern)?;
                    self.write_fmt(format_args!(" {LAMDA_ARROW_STR} "))?;
                    self.write_block(arm.body)?;
                    self.write_endln()?;
                }
                self.pop_depth();
                self.write_depth()?;
                self.write_char('}')?;
                Ok(())
            }
            ExpressionKind::MatchMethod(match_method) => {
                self.write_expression(match_method.scrutinee)?;
                self.push_depth();
                self.write_endln()?;
                let last_index = match_method.arms.len().saturating_sub(1);
                for (i, arm) in match_method.arms.iter().enumerate() {
                    self.write_depth()?;
                    if match_method.optional_map {
                        self.write_char('?')?;
                    }
                    self.write_char('.')?;
                    self.write_str(arm.variant.as_str())?;
                    if let Some(binding) = &arm.binding {
                        self.write_fmt(format_args!(
                            "{{{} {LAMDA_ARROW_STR} ",
                            binding.ident.as_str()
                        ))?;
                        self.write_block(arm.body)?;
                        self.write_char('}')?;
                    } else {
                        self.write_block(arm.body)?;
                    }

                    if i != last_index {
                        self.write_endln()?;
                    }
                }
                self.pop_depth();
                self.write_depth()?;
                Ok(())
            }
            ExpressionKind::For(for_) => {
                self.write_fmt(format_args!("{FOR_STR} "))?;
                match &for_.condition {
                    ForCondition::Loop => (),
                    ForCondition::While(condition) => self.write_expression(*condition)?,
                    ForCondition::Foreach {
                        index,
                        collection,
                        element_kind,
                    } => {
                        if let Some(index) = index {
                            self.write_fmt(format_args!("{}, ", index.ident.as_str()))?;
                        }
                        match element_kind {
                            ForElementKind::Single([binding]) => self.write_str(binding.ident.as_str())?,
                            ForElementKind::Tuple(bindings) => {
                                self.write_char('(')?;
                                let last_index = bindings.len().saturating_sub(1);
                                for (i, binding) in bindings.iter().enumerate() {
                                    self.write_str(binding.ident.as_str())?;
                                    if i != last_index {
                                        self.write_str(", ")?;
                                    }
                                }
                                self.write_char(')')?
                            }
                        }
                        self.write_fmt(format_args!(" {IN_FOR_LOOP_STR} ", ))?;
                        self.write_expression(*collection)?;
                    }
                }
                self.write_block(for_.block)
            }
            ExpressionKind::Block(block_id) => self.write_block(*block_id),
            ExpressionKind::TypeOf(type_of) => {
                self.write_expression(type_of.value)?;
                self.write_fmt(format_args!(" {TYPEOF_STR} "))?;
                match &type_of.kind {
                    TypeofKind::Null => {
                        self.write_str(KeyWord::Null.as_str())?
                    }
                    TypeofKind::NotNull => {
                        self.write_fmt(format_args!(
                            "{}{}", 
                            Symbol::Not.as_str(), 
                            KeyWord::Null.as_str(),
                        ))?
                    }
                    TypeofKind::Union { type_name, variant_name } => {
                        self.write_fmt(format_args!(
                            "{}.{}", 
                            type_name.as_str(), 
                            variant_name.as_str(),
                        ))?
                    }
                };
                if let Some(binding) = &type_of.binding {
                    self.write_fmt(format_args!("({})", binding.ident.as_str()))?;
                }
                Ok(())
            }
            ExpressionKind::Lambda(lambda) => self.write_lambda(lambda),
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
            ExpressionKind::Pass(expression_id) => {
                self.write_expression(*expression_id)?;
                self.write_fmt(format_args!(".{PASS_STR}"))
            },
            ExpressionKind::StringFormat(fmt) => {
                let tag = if fmt.to_string { "f" } else { "fstr" };
                self.write_fmt(format_args!("{tag}"))?;
                for (text, expr_id) in &fmt.parts {
                    self.write_fmt(format_args!("\"{text}\""))?;
                    self.write_char('{')?;
                    self.write_expression(*expr_id)?;
                    self.write_char('}')?;
                }
                self.write_fmt(format_args!("\"{}\"", fmt.trailing))
            },
            ExpressionKind::Tuple(values) => {
                self.write_str(".(")?;
                let last_index = values.len().saturating_sub(1);
                for (i, value) in values.iter().enumerate() {
                    self.write_expression(*value)?;
                    if i != last_index {
                        self.write_str(", ")?;
                    }
                } 
                self.write_char(')')
            }
            ExpressionKind::NamedTuple(values) => {
                self.write_str(".{")?;
                let last_index = values.len().saturating_sub(1);
                for (i, (name, value)) in values.iter().enumerate() {
                    self.write_fmt(format_args!("{}: ", name.as_str()))?;
                    self.write_expression(*value)?;
                    if i != last_index {
                        self.write_str(", ")?;
                    }
                } 
                self.write_char('}')
            }
        }
    }

    fn write_match_pattern(&mut self, arm: &MatchPattern) -> Result<()> {
        match &arm {
            MatchPattern::If { pattern, if_condition } => {
                self.write_match_pattern(pattern)?;
                self.write_fmt(format_args!(" {} ", KeyWord::If.as_str()))?;
                self.write_expression(*if_condition)?;
                self.write_fmt(format_args!(" {}", Symbol::LambdaArrow.as_str()))
            }
            MatchPattern::Null => self.write_str(KeyWord::Null.as_str()),
            MatchPattern::NotNull(binding) => {
                self.write_fmt(format_args!("{}{}(", Symbol::Not.as_str(), KeyWord::Null.as_str()))?;
                self.write_str(binding.ident.as_str())?;
                self.write_char(')')
            }
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
                self.write_fmt(format_args!(
                    "{}.{}",
                    ctor.type_name.as_str(),
                    ctor.variant_name.as_str()
                ))?;
                if let Some(binding) = &ctor.binding {
                    self.write_fmt(format_args!("({})", binding.ident.as_str()))?;
                }
                Ok(())
            }
            MatchPattern::Tuple(tuple) => {
                self.write_char('(')?;
                for (i, element) in tuple.elements.iter().enumerate() {
                    if i > 0 {
                        self.write_str(", ")?;
                    }
                    self.write_match_pattern(element)?;
                }
                if tuple.rest {
                    if !tuple.elements.is_empty() {
                        self.write_str(", ")?;
                    }
                    self.write_str("..")?;
                }
                self.write_char(')')
            }
            MatchPattern::NamedTuple(named) => {
                self.write_char('{')?;
                for (i, field) in named.fields.iter().enumerate() {
                    if i > 0 {
                        self.write_str(", ")?;
                    }
                    self.write_str(field.field.as_str())?;
                    if let Some(binding) = &field.binding {
                        self.write_str(": ")?;
                        self.write_str(binding.ident.as_str())?;
                    }
                }
                if named.rest {
                    if !named.fields.is_empty() {
                        self.write_str(", ")?;
                    }
                    self.write_str("..")?;
                }
                self.write_char('}')
            }
            MatchPattern::ConstructorStruct(struct_pat) => {
                self.write_str(struct_pat.type_name.as_str())?;
                self.write_char('{')?;
                for (i, field) in struct_pat.fields.iter().enumerate() {
                    if i > 0 {
                        self.write_str(", ")?;
                    }
                    self.write_str(field.field.as_str())?;
                    if let Some(binding) = &field.binding {
                        self.write_str(": ")?;
                        self.write_str(binding.ident.as_str())?;
                    }
                }
                if struct_pat.rest {
                    if !struct_pat.fields.is_empty() {
                        self.write_str(", ")?;
                    }
                    self.write_str("..")?;
                }
                self.write_char('}')
            }
            MatchPattern::Rest => self.write_str(".."),
        }
    }

    fn display_branch(&mut self, if_arm: &Option<Box<IfBranch>>) -> Result<()> {
        let mut current = if_arm.as_ref();
        while let Some(arm) = current {
            self.write_char(' ')?;
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
            SoulType::TupleKind(kind) => {
                self.write_char('(')?;
                let last_index = kind.len().saturating_sub(1);
                match kind {
                    TupleKind::Tuple(types) => for (i, ty) in types.iter().enumerate() {
                        self.write_type(ty)?;
                        if i != last_index {
                            self.write_str(", ")?;
                        }
                    }
                    TupleKind::NamedTuple(items) => for (i, (name, ty)) in items.iter().enumerate() {
                        self.write_fmt(format_args!("{}: ", name.as_str()))?;
                        self.write_type(ty)?;
                        if i != last_index {
                            self.write_str(", ")?;
                        }
                    }
                }
                self.write_char(')')
            }
            SoulType::None => self.write_str(PrimitiveTypes::None.as_str()),
            SoulType::Never => self.write_char('!'),
            SoulType::Primitive(primitive_types) => self.write_str(primitive_types.as_str()),
            SoulType::Array(array) => {
                match array.kind {
                    ArrayKind::StackArrayWildcard => self.write_str("[_]")?,
                    ArrayKind::StackArray(num) => self.write_fmt(format_args!("[{num}]"))?,
                    ArrayKind::HeapArray => self.write_str("[]")?,
                    ArrayKind::MutSlice => self.write_str("[&mut]")?,
                    ArrayKind::ConstSlice => self.write_str("[&]")?,
                }
                self.write_type(&array.of_type)
            }
            SoulType::Reference(reference) | SoulType::Pointer(reference) => {
                if matches!(ty, SoulType::Pointer(_)) {
                    self.write_char('*')?;
                } else {
                    self.write_char('&')?;
                }

                if let Some(lifetime) = &reference.lifetime {
                    self.write_fmt(format_args!("'{} ", lifetime.as_str()))?;
                }
                if reference.mutable {
                    self.write_str("mut ")?;
                }
                self.write_type(&reference.inner)
            }
            SoulType::RawPtr(inner) => {
                self.write_str("RawPtr")?;
                if let Some(inner) = inner {
                    self.write_char('<')?;
                    self.write_type(inner)?;
                    self.write_char('>')?;
                }
                Ok(())
            }
            SoulType::Res { ok, err } => {
                self.write_str("Res")?;
                match (ok, err) {
                    (Some(ok), Some(err)) => {
                        self.write_char('<')?;
                        self.write_type(ok)?;
                        self.write_str(", ")?;
                        self.write_type(err)?;
                        self.write_char('>')?;
                    }
                    (Some(ok), None) => {
                        self.write_char('<')?;
                        self.write_type(ok)?;
                        self.write_char('>')?;
                    }
                    _ => {}
                }
                Ok(())
            }
            SoulType::Optional(soul_type) => {
                self.write_char('?')?;
                self.write_type(&soul_type)
            }
            SoulType::Stub(stub) => {
                self.write_str(&stub.name)?;
                self.write_generic_types(&stub.generics)?;
                Ok(())
            }
            SoulType::NamedVariant { base, variant } => {
                self.write_type(base)?;
                self.write_fmt(format_args!(".{}", variant.as_str()))
            }
            SoulType::String => self.write_str(Types::String.as_str()),
            SoulType::FormatString => self.write_str(Types::FormatString.as_str()),
            SoulType::Any => self.write_str(Types::Any.as_str()),
            SoulType::Error => self.write_str(Types::Error.as_str()),
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
                if let Some(binding) = &array.for_index {
                    self.write_fmt(format_args!("{} {IN_FOR_LOOP_STR} ", binding.ident.as_str()))?;
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
            return Ok(());
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
            return Ok(());
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
        self.get(index.clone()).ok_or(anyhow::Error::msg(format!(
            "{index:?} is not found; {}\n",
            Backtrace::force_capture().to_string()
        )))
    }
}
