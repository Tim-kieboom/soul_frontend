use std::{
    backtrace::Backtrace,
    fmt::{Arguments, Debug},
    path::Path,
};

use crate::{
    config,
    display::{vecmap_to_pretty_vec, write_create_file, write_to_file, writer::Writer},
    push_fmt,
};
use anyhow::Result;
use ast_model::{
    AstStore, AstTree, ExternalCrateData, FunctionKind,
    block::BlockId,
    expression::{
        AnyArray, ExpressionId, ExpressionKind, ForCondition, FunctionCalleeKind, IfBranch,
        IfCondition, Lambda, MatchPattern, TypeofKind,
    },
    soul_type::{ArrayKind, Generic, SoulType, TupleKind},
    statements::{
        Assignment, Enum, EnumVariant, FunctionModifier, FunctionThisKind, ImplBlock, Import,
        ImportItem, ImportKind, Parameter, Statement, StatementId, StatementKind, Struct, Trait,
        TypeDef, UnionKind, UseBlock, VarPattern, Variable,
    },
};
use soul_tokenizer::model::{
    keyword::KeyWord::{self},
    types::Types,
};
use soul_utils::{
    FunctionId, TypeModifier,
    collections::vec_map::{VecMap, VecMapIndex},
    ids::IdAlloc,
    linkage::Linkage,
    soul_names::{PrimitiveTypes, Symbol},
    span::{Attribute, ModuleId},
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
    add_tags: bool,
}

pub(crate) fn display_ast(tree: &AstTree) -> Result<()> {
    inner_display_ast(tree).map_err(|err| anyhow::anyhow!("in display_ast: {err}"))
}

fn inner_display_ast(tree: &AstTree) -> Result<()> {
    let mut output_path = config::CONFIG.output_path().join("ast");
    output_path.push("tree.soulc");

    let mut writer = write_create_file(&output_path)?;
    display_ast_tree(tree, config::CONFIG.source_path(), &mut writer)?;

    output_path.pop();
    output_path.push("json");

    let modules = vecmap_to_json_str(tree.crates.modules.as_vecmap())?;
    let externals = serde_json::to_string_pretty(&tree.crates.external)?;
    let scope_info = vecmap_to_json_str(tree.scope_info.scopes.as_vecmap())?;

    let blocks = vecmap_to_json_str(&tree.crates.store.blocks)?;
    let functions = vecmap_to_json_str(&tree.crates.store.functions)?;
    let statements = vecmap_to_json_str(&tree.crates.store.statements)?;
    let expressions = vecmap_to_json_str(&tree.crates.store.expressions)?;

    write_to_file(&output_path.join("modules.json"), &modules)?;
    write_to_file(&output_path.join("externals.json"), &externals)?;
    write_to_file(&output_path.join("scope_info.json"), &scope_info)?;

    output_path.push("store");
    write_to_file(&output_path.join("blocks.json"), &blocks)?;
    write_to_file(&output_path.join("functions.json"), &functions)?;
    write_to_file(&output_path.join("statements.json"), &statements)?;
    write_to_file(&output_path.join("expressions.json"), &expressions)?;

    Ok(())
}

fn vecmap_to_json_str<K, V>(map: &VecMap<K, V>) -> Result<String>
where
    K: VecMapIndex + Debug,
    V: serde::Serialize,
{
    let vec = vecmap_to_pretty_vec(map);
    let str = serde_json::to_string_pretty(&vec)?;
    Ok(str)
}

fn display_ast_tree(ast: &AstTree, root_dir: &Path, writer: &mut impl Writer) -> Result<()> {
    let mut displayer = Displayer::new(ast, root_dir, &ast.crates.store, writer);
    displayer.write_crate_overview()?;
    displayer.write_entry(ast.root)?;
    for (name, data) in &ast.crates.external {
        displayer.write_external(name, data)?
    }
    writer.writer_flush()?;
    Ok(())
}

fn linkage_str(linkage: &soul_utils::linkage::Linkage) -> &'static str {
    match linkage {
        Linkage::Dynamic => "dynamic",
        Linkage::Static => "static",
    }
}

impl<'a, W: Writer> Displayer<'a, W> {
    fn new(ast: &'a AstTree, root_dir: &'a Path, store: &'a AstStore, writer: &'a mut W) -> Self {
        Self {
            ast,
            store,
            writer,
            root_dir,
            add_tags: true,
            depth: String::new(),
        }
    }

    fn write_crate_overview(&mut self) -> Result<()> {
        if self.ast.crates.external.is_empty() {
            return Ok(());
        }
        self.push_fmt(format_args!("// === Crate Forest ===\n"))?;
        for (name, data) in &self.ast.crates.external {
            let dep_label = if data.root_id == self.ast.root {
                " (self)"
            } else {
                ""
            };
            self.push_fmt(format_args!(
                "//   {name}: root={:?} linkage={} modules={}{}\n",
                data.root_id,
                linkage_str(&data.linkage),
                data.module_ids.len(),
                dep_label,
            ))?;
        }
        self.push_fmt(format_args!("// =====================\n\n"))?;
        Ok(())
    }

    fn write_entry(&mut self, root: ModuleId) -> Result<()> {
        self.write_depth()?;
        self.push_fmt(format_args!("{} . {{", KeyWord::Crate.as_str()))?;

        self.push_depth();

        self.write_endln()?;
        self.write_depth()?;
        self.write_module(root)?;

        self.pop_depth();

        self.write_endln()?;
        self.write_depth()?;
        self.push_char('}')
    }

    fn write_external(&mut self, name: &str, data: &ExternalCrateData) -> Result<()> {
        self.write_endln()?;
        self.write_depth()?;

        self.push_fmt(format_args!("{} {name} {{", KeyWord::Crate.as_str()))?;

        self.push_depth();
        self.write_endln()?;
        for module_id in data.module_ids.entries() {
            self.write_module(module_id)?;
            self.write_endln()?;
        }

        self.pop_depth();
        self.write_endln()?;
        self.write_depth()?;
        self.push_char('}')
    }

    fn write_module(&mut self, id: ModuleId) -> Result<()> {
        let module = self.ast.crates.modules.as_vecmap().get_err(id)?;
        if self.add_tags {
            self.push_fmt(format_args!("// {id:?}\n"))?;
        }

        self.write_depth()?;
        self.push_fmt(format_args!("mod {} {{\n", module.name))?;
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
        self.push_char('}')?;
        Ok(())
    }

    fn write_block(&mut self, id: BlockId) -> Result<()> {
        let block = self.store.blocks.get_err(id)?;
        if block.is_const {
            self.push_str(KeyWord::Const.as_str())?;
            self.push_char(' ')?;
        }
        self.push_str("{\n")?;
        self.push_depth();
        for id in &block.statements {
            self.write_depth()?;
            self.write_statement(*id)?;
            self.write_endln()?;
        }

        self.pop_depth();
        self.write_depth()?;
        self.push_char('}')?;
        Ok(())
    }

    fn write_statement(&mut self, id: StatementId) -> Result<()> {
        let statement = self.store.statements.get_err(id)?;
        self.write_attributes(&statement.meta_data.attributes)?;
        if self.add_tags {
            self.write_statement_tag(statement)?;
        }

        if statement.is_public() {
            self.push_str("pub ")?;
        }

        match &statement.node {
            StatementKind::Enum(enum_) => self.write_enum(enum_),
            StatementKind::Union(union_) => self.write_enum(union_),
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
                    self.push_char(';')?;
                }
                Ok(())
            }
        }
    }

    fn write_attributes(&mut self, attributes: &[Attribute]) -> Result<()> {
        for attribute in attributes {
            self.push_char('#')?;
            self.push_char('[')?;
            self.push_str(attribute.name.as_str())?;
            if !attribute.values.is_empty() {
                self.push_char('(')?;
                let last_index = attribute.values.len().saturating_sub(1);
                for (i, value) in attribute.values.iter().enumerate() {
                    self.push_str(value.as_str())?;
                    if i != last_index {
                        self.push_str(", ")?;
                    }
                }
                self.push_char(')')?;
            }
            self.push_char(']')?;
            self.write_endln()?;
            self.write_depth()?;
        }
        Ok(())
    }

    fn write_statement_tag(&mut self, statement: &Statement) -> Result<()> {
        self.push_str("// ")?;
        match &statement.node {
            StatementKind::Expression { expression, .. } => {
                self.write_expression_tag(*expression)?
            }
            _ => self.push_str(statement.node.variant_name())?,
        }

        if let Some(id_string) = statement.node.get_any_id_string() {
            self.push_fmt(format_args!(": {id_string} "))?;
        }

        match &statement.node {
            StatementKind::Variable(variable) => {
                if let Some((modifier, ty, _)) = self.ast.declares.get_variable_type(variable.id) {
                    self.push_fmt(format_args!("{modifier:?}"))?;
                    if let Some(ty) = ty {
                        self.push_str(": ")?;
                        self.write_type(ty)?;
                    }
                    self.push_char(' ')?;
                }
                if let Some(value) = variable.initialize_value {
                    self.push_str("= ")?;
                    self.write_expression_tag(value)?
                }
            }
            StatementKind::Assignment(assignment) => {
                self.write_expression_tag(assignment.left)?;
                self.push_str(" = ")?;
                self.write_expression_tag(assignment.right)?
            }
            StatementKind::Expression { expression, .. } => {
                let value = self.store.expressions.get_err(*expression)?;
                match &value.node {
                    ExpressionKind::FunctionCall(function_call) => {
                        if let Some(resolved) = self.ast.declares.get_call_resolve(function_call.id)
                        {
                            self.push_fmt(format_args!(" resolved = {resolved:?}"))?
                        } else {
                            self.push_str(" resolved = null")?
                        }
                    }
                    ExpressionKind::Variable(variable) => {
                        if let Some(resolved) = self.ast.declares.get_variable_resolve(variable.id)
                        {
                            self.push_fmt(format_args!(" resolved = {resolved:?}"))?
                        } else {
                            self.push_str(" resolved = null")?
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        }

        self.write_endln()?;
        self.write_depth()
    }

    fn write_expression_tag(&mut self, id: ExpressionId) -> Result<()> {
        let expression = self.store.expressions.get_err(id)?;
        self.push_str(expression.node.variant_name())?;
        if let Some(ty) = self.ast.declares.get_expression_type(id) {
            self.push_str(": ")?;
            self.write_type(ty)?;
        }
        Ok(())
    }

    fn write_assignment(&mut self, assignment: &Assignment) -> Result<()> {
        self.write_expression(assignment.left)?;
        self.push_str(" = ")?;
        self.write_expression(assignment.right)
    }

    fn write_use_block(&mut self, use_block: &UseBlock) -> Result<()> {
        self.push_str(USE_STR)?;
        self.write_generic_defines(&use_block.use_generics)?;
        self.push_char(' ')?;
        self.write_type(&use_block.ty)?;
        self.push_str(" {\n")?;
        self.push_depth();
        for id in &use_block.statements {
            self.write_depth()?;
            if self.store.statements[*id].is_public() {
                self.push_str("pub ")?;
            }

            self.write_statement(*id)?;
            self.write_endln()?;
        }

        for method in &use_block.methods {
            self.write_depth()?;
            if method.is_public {
                self.push_str("pub ")?;
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
        self.push_char('}')
    }

    fn write_impl(&mut self, impl_: &ImplBlock) -> Result<()> {
        self.push_str(IMPL_STR)?;
        self.push_char(' ')?;
        self.write_type(&impl_.impl_trait)?;
        self.push_str(" {\n")?;
        self.push_depth();
        for methode in &impl_.methods {
            self.write_depth()?;
            self.write_any_function(*methode)?;
            self.write_endln()?;
        }
        self.pop_depth();
        self.write_depth()?;
        self.push_char('}')
    }

    fn write_variable(&mut self, variable: &Variable) -> Result<()> {
        if variable.modifier == TypeModifier::Mut {
            self.push_str(KeyWord::Mut.as_str())?;
            self.push_char(' ')?;
        }
        self.write_var_pattern(&variable.pattern)?;

        if let Some(ty) = &variable.ty {
            self.push_str(": ")?;
            self.write_type(ty)?;
        }

        if let Some(value) = variable.initialize_value {
            let assign_str = if variable.modifier == TypeModifier::Const {
                " :: "
            } else if variable.ty.is_some() {
                " = "
            } else {
                " := "
            };

            self.push_str(assign_str)?;
            self.write_expression(value)?;
        }

        Ok(())
    }

    fn write_var_pattern(&mut self, pattern: &VarPattern) -> Result<()> {
        match pattern {
            VarPattern::Discard => self.push_str("_"),
            VarPattern::Simple { binding, modifier } => {
                if *modifier == TypeModifier::Mut {
                    self.push_str("mut ")?;
                }
                self.push_str(binding.ident.as_str())
            }
            VarPattern::Tuple(tuple) => {
                self.push_char('(')?;
                for (i, element) in tuple.elements.iter().enumerate() {
                    if i > 0 {
                        self.push_str(", ")?;
                    }
                    self.write_var_pattern(element)?;
                }
                if tuple.rest {
                    if !tuple.elements.is_empty() {
                        self.push_str(", ")?;
                    }
                    self.push_str("..")?;
                }
                self.push_char(')')
            }
            VarPattern::NamedTuple(named) => {
                self.push_char('{')?;
                for (i, field) in named.fields.iter().enumerate() {
                    if i > 0 {
                        self.push_str(", ")?;
                    }
                    if field.modifier == TypeModifier::Mut {
                        self.push_str("mut ")?;
                    }
                    self.push_str(field.field.as_str())?;
                    match &field.binding {
                        Some(binding) if binding.ident != field.field => {
                            self.push_str(": ")?;
                            self.push_str(binding.ident.as_str())?;
                        }
                        None => {
                            self.push_str(": _")?;
                        }
                        _ => {}
                    }
                }
                if named.rest {
                    if !named.fields.is_empty() {
                        self.push_str(", ")?;
                    }
                    self.push_str("..")?;
                }
                self.push_char('}')
            }
            VarPattern::Constructor(constructor) => {
                self.push_str(constructor.type_name.as_str())?;
                self.push_char('{')?;
                for (i, field) in constructor.fields.iter().enumerate() {
                    if i > 0 {
                        self.push_str(", ")?;
                    }
                    if field.modifier == TypeModifier::Mut {
                        self.push_str("mut ")?;
                    }
                    self.push_str(field.field.as_str())?;
                    match &field.binding {
                        Some(binding) if binding.ident != field.field => {
                            self.push_str(": ")?;
                            self.push_str(binding.ident.as_str())?;
                        }
                        None => {
                            self.push_str(": _")?;
                        }
                        _ => {}
                    }
                }
                if constructor.rest {
                    if !constructor.fields.is_empty() {
                        self.push_str(", ")?;
                    }
                    self.push_str("..")?;
                }
                self.push_char('}')
            }
        }
    }

    fn write_lambda(&mut self, lambda: &Lambda) -> Result<()> {
        if lambda.parameters.len() == 1 {
            self.write_var_pattern(&lambda.parameters[0])?;
        } else {
            self.push_char('(')?;
            for (i, param) in lambda.parameters.iter().enumerate() {
                if i > 0 {
                    self.push_str(", ")?;
                }
                self.write_var_pattern(param)?;
            }
            self.push_char(')')?;
        }
        push_fmt!(self, " {} ", LAMDA_ARROW_STR)?;
        self.write_block(lambda.body)
    }

    fn write_typedef(&mut self, type_def: &TypeDef) -> Result<()> {
        self.push_str(TYPE_STR)?;
        self.push_char(' ')?;
        self.write_type(&type_def.new_type)?;
        self.push_str(" = ")?;
        if type_def.is_distinct {
            push_fmt!(self, "{DISTINCT_STR} ")?;
        }
        self.write_type(&type_def.old_type)
    }

    fn write_struct(&mut self, struct_: &Struct) -> Result<()> {
        push_fmt!(self, "{STRUCT_STR} {}", struct_.name)?;
        self.write_generic_defines(&struct_.generics)?;
        self.push_str("{\n")?;
        self.push_depth();

        for field in &struct_.fields {
            self.write_depth()?;
            if field.is_public {
                self.push_str("pub ")?;
            }

            self.write_var_pattern(&field.value.pattern)?;
            if let Some(ty) = &field.value.ty {
                self.push_str(": ")?;
                self.write_type(ty)?;
            }
            if let Some(value) = &field.value.initialize_value {
                self.push_str(" = ")?;
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
        self.push_char('}')?;
        Ok(())
    }

    fn write_import(&mut self, import: &Import) -> Result<()> {
        push_fmt!(self, "{IMPORT_STR} (\n")?;
        self.push_depth();
        for path in import.paths.iter() {
            self.write_depth()?;
            self.push_str(&path.module.display(self.root_dir))?;
            match &path.kind {
                ImportKind::This => self.push_str("this")?,
                ImportKind::Glob => self.push_char('*')?,
                ImportKind::Alias(ident) => push_fmt!(self, " as {ident}")?,
                ImportKind::Module => (),
                ImportKind::Items {
                    has_this,
                    this_alias,
                    items,
                } => {
                    self.push_char('{')?;
                    if *has_this {
                        self.push_str("this")?;
                        if let Some(alias) = &this_alias {
                            push_fmt!(self, " as {alias}")?;
                        }

                        if !items.is_empty() {
                            self.push_str(", ")?;
                        }
                    }

                    let last_index = items.len().saturating_sub(1);
                    for (i, item) in items.iter().enumerate() {
                        match item {
                            ImportItem::Normal(ident) => self.push_str(ident.as_str())?,
                            ImportItem::Alias { name, alias } => self.push_fmt(format_args!(
                                "{} as {}",
                                name.as_str(),
                                alias.as_str()
                            ))?,
                        }
                        if i != last_index {
                            self.push_str(", ")?;
                        }
                    }
                    self.push_char('}')?;
                }
            }
            self.write_endln()?;
        }
        self.pop_depth();
        self.write_depth()?;
        self.push_char(')')?;
        Ok(())
    }

    fn write_enum(&mut self, enum_: &Enum) -> Result<()> {
        push_fmt!(self, "{ENUM_STR} {}", enum_.name)?;
        if let Some(ty) = &enum_.impl_type {
            self.push_str(": ")?;
            self.write_type(ty)?;
        }

        self.push_str(" {\n")?;
        self.push_depth();
        let last_index = enum_.variants.len().saturating_sub(1);
        for (i, variant) in enum_.variants.iter().enumerate() {
            self.write_depth()?;
            match variant {
                EnumVariant::Normal(ident) => self.push_str(ident.as_str())?,
                EnumVariant::Assigned { name, value } => {
                    push_fmt!(self, "{name} = ")?;
                    self.write_expression(*value)?
                }
                EnumVariant::Union(union) => match union {
                    UnionKind::Tuple { name, parameters } => {
                        self.push_str(name.as_str())?;
                        self.push_char('(')?;
                        let last_index = parameters.len().saturating_sub(1);
                        for (i, ty) in parameters.iter().enumerate() {
                            self.write_type(ty)?;
                            if i != last_index {
                                self.push_str(", ")?;
                            }
                        }
                        self.push_char(')')?;
                    }
                    UnionKind::NamedTuple { name, parameters } => {
                        self.push_str(name.as_str())?;
                        self.push_char('{')?;
                        let last_index = parameters.len().saturating_sub(1);
                        for (i, (ident, ty)) in parameters.iter().enumerate() {
                            push_fmt!(self, "{ident}: ")?;
                            self.write_type(ty)?;
                            if i != last_index {
                                self.push_str(", ")?;
                            }
                        }
                        self.push_char('}')?;
                    }
                },
            }
            if i != last_index {
                self.push_char(',')?;
            }
            self.write_endln()?;
        }
        self.pop_depth();
        self.write_depth()?;
        self.push_str("}\n")
    }

    fn write_trait(&mut self, trait_: &Trait) -> Result<()> {
        push_fmt!(self, "{TRAIT_STR} {} {{\n", trait_.name)?;
        self.push_depth();
        for ty in &trait_.typedefs {
            self.write_depth()?;
            push_fmt!(self, "{} ", KeyWord::Type.as_str())?;
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
        self.push_str("}\n")
    }

    fn write_any_function(&mut self, id: FunctionId) -> Result<()> {
        let function = self.store.functions.get_err(id)?;
        let signature = &function.signature();

        if let Some(external) = signature.external {
            push_fmt!(
                self,
                "{} \"{}\" ",
                KeyWord::Extern.as_str(),
                external.as_str()
            )?;
        }

        if signature.modifier.contains(FunctionModifier::CONST) {
            push_fmt!(self, "{} ", KeyWord::Const.as_str())?;
        }
        self.push_str(signature.name.as_str())?;
        self.write_generic_defines(&signature.generics)?;
        self.push_char('(')?;

        match signature.function_kind {
            FunctionThisKind::Ctor => self.push_str("/*ctor*/")?,
            FunctionThisKind::ArrayCtor => self.push_str("/*arrayCtor*/")?,
            _ => {
                if let Some(kind) = signature.function_kind.display() {
                    self.push_str(kind)?;
                    if !signature.parameters.is_empty() {
                        self.push_str(", ")?;
                    }
                }
            }
        }

        self.write_parameters(&signature.parameters)?;
        self.push_str("): ")?;
        self.write_type(&signature.return_type)?;
        let block = match function {
            FunctionKind::Normal(function) => function.block,
            FunctionKind::Signature(_) => return Ok(()),
        };
        self.push_char(' ')?;
        self.write_block(block)
    }

    fn write_parameters(&mut self, parameters: &[Parameter]) -> Result<()> {
        let last_index = parameters.len().saturating_sub(1);
        for (i, parameter) in parameters.iter().enumerate() {
            push_fmt!(
                self,
                "{}{}: ",
                if parameter.is_mut { "mut " } else { "" },
                parameter.name.as_str()
            )?;
            self.write_type(&parameter.ty)?;
            if let Some(value) = parameter.default {
                self.write_expression(value)?;
            }
            if i != last_index {
                self.push_str(", ")?;
            }
        }

        Ok(())
    }

    fn write_expression(&mut self, id: ExpressionId) -> Result<()> {
        if id == ExpressionId::error() {
            self.push_str("<error>")?;
            return Ok(());
        }

        let expression = self.store.expressions.get_err(id)?;
        match &expression.node {
            ExpressionKind::Null(_) => self.push_str("null"),
            ExpressionKind::None(_) => self.push_str("()"),
            ExpressionKind::Undefined(_) => self.push_str("undefined"),
            ExpressionKind::Literal((_, literal)) => push_fmt!(self, "{literal:?}"),
            ExpressionKind::Copy(value) => {
                self.write_expression(*value)?;
                push_fmt!(self, ".{}", KeyWord::Copy.as_str())
            }
            ExpressionKind::Index(index) => {
                self.write_expression(index.collection)?;
                if index.optional_map {
                    self.push_char('?')?;
                }
                self.push_char('[')?;
                self.write_expression(index.index)?;
                self.push_char(']')
            }
            ExpressionKind::FieldAccess(field_access) => {
                self.write_expression(field_access.object)?;
                if field_access.optional_map {
                    self.push_char('?')?;
                }
                self.push_char('.')?;
                self.push_str(field_access.field.as_str())
            }
            ExpressionKind::FunctionCall(function_call) => {
                if let Some(callee) = &function_call.callee {
                    match &callee.kind {
                        FunctionCalleeKind::Type(soul_type) => self.write_type(soul_type)?,
                        FunctionCalleeKind::Expression(expression_id) => {
                            self.write_expression(*expression_id)?
                        }
                    }

                    if callee.optional_map {
                        self.push_char('?')?;
                    }
                    self.push_char('.')?;
                }

                self.push_str(function_call.name.as_str())?;
                self.write_generic_types(&function_call.generics)?;
                self.push_char('(')?;
                let last_index = function_call.arguments.len().saturating_sub(1);
                for (i, arg) in function_call.arguments.iter().enumerate() {
                    if let Some(name) = &arg.name {
                        push_fmt!(self, "{name}: ")?;
                    }
                    self.write_expression(arg.value)?;
                    if i != last_index {
                        self.push_str(", ")?;
                    }
                }
                self.push_char(')')
            }
            ExpressionKind::Constructor(constructor) => {
                self.write_type(&constructor.ty)?;
                self.push_str(".(")?;
                let last_index = constructor.arguments.len().saturating_sub(1);
                for (i, arg) in constructor.arguments.iter().enumerate() {
                    if let Some(name) = &arg.name {
                        push_fmt!(self, "{name}: ")?;
                    }
                    self.write_expression(arg.value)?;
                    if i != last_index {
                        self.push_str(", ")?;
                    }
                }
                self.push_char(')')
            }
            ExpressionKind::StructConstructor(ctor) => {
                self.write_type(&ctor.struct_type)?;
                let last_index = ctor.values.len().saturating_sub(1);
                self.push_char('{')?;
                for (i, arg) in ctor.values.iter().enumerate() {
                    push_fmt!(self, "{}: ", arg.0)?;
                    self.write_expression(arg.1)?;
                    if i != last_index || ctor.defaults {
                        self.push_str(", ")?;
                    }
                }
                if ctor.defaults {
                    self.push_str("..")?;
                }
                self.push_char('}')
            }
            ExpressionKind::Variable(variable) => self.push_str(variable.name.as_str()),
            ExpressionKind::Array(any_array) => self.write_any_array(any_array),
            ExpressionKind::Sizeof(value) => {
                self.write_expression(*value)?;
                push_fmt!(self, ".{SIZEOF_STR}")
            }
            ExpressionKind::New(expression_id) => {
                push_fmt!(self, "{NEW_STR}(")?;
                self.write_expression(*expression_id)?;
                self.push_char(')')
            }
            ExpressionKind::NewArray(any_array) => {
                self.push_str(NEW_STR)?;
                self.write_any_array(any_array)
            }
            ExpressionKind::Unary(unary) => {
                self.push_str(unary.operator.value.as_str())?;
                self.write_expression(unary.value)
            }
            ExpressionKind::Binary(binary) => {
                self.push_char('(')?;
                self.write_expression(binary.left)?;
                self.push_char(' ')?;
                self.push_str(binary.operator.value.as_str())?;
                self.push_char(' ')?;
                self.write_expression(binary.right)?;
                self.push_char(')')
            }
            ExpressionKind::Ref(ref_) => {
                self.push_char('&')?;
                if ref_.is_mutable {
                    self.push_str("mut ")?;
                }
                self.write_expression(ref_.value)
            }
            ExpressionKind::Deref(deref) => {
                self.push_char('*')?;
                self.write_expression(deref.value)
            }
            ExpressionKind::If(if_) => {
                self.push_str(IF_STR)?;
                self.push_char(' ')?;
                self.write_if_condition(&if_.condition)?;
                self.write_block(if_.block)?;
                self.display_branch(&if_.branch)
            }
            ExpressionKind::Match(match_) => {
                push_fmt!(self, "{MATCH_STR} ")?;
                self.write_expression(match_.scrutinee)?;
                self.push_str(" {\n")?;
                self.push_depth();
                for arm in &match_.arms {
                    self.write_depth()?;
                    self.write_match_pattern(&arm.pattern)?;
                    push_fmt!(self, " {LAMDA_ARROW_STR} ")?;
                    self.write_block(arm.body)?;
                    self.write_endln()?;
                }
                self.pop_depth();
                self.write_depth()?;
                self.push_char('}')?;
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
                        self.push_char('?')?;
                    }
                    self.push_char('.')?;
                    self.push_str(arm.variant.as_str())?;
                    if let Some(binding) = &arm.binding {
                        push_fmt!(self, "{{{} {LAMDA_ARROW_STR} ", binding.ident.as_str())?;
                        self.write_block(arm.body)?;
                        self.push_char('}')?;
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
                push_fmt!(self, "{FOR_STR} ")?;
                match &for_.condition {
                    ForCondition::Loop => (),
                    ForCondition::While(condition) => self.write_expression(*condition)?,
                    ForCondition::Foreach {
                        index,
                        collection,
                        element_kind,
                    } => {
                        if let Some(index) = index {
                            push_fmt!(self, "{}, ", index.ident.as_str())?;
                        }
                        self.write_var_pattern(element_kind)?;
                        push_fmt!(self, " {IN_FOR_LOOP_STR} ",)?;
                        self.write_expression(*collection)?;
                    }
                }
                self.write_block(for_.block)
            }
            ExpressionKind::Block(block_id) => self.write_block(*block_id),
            ExpressionKind::TypeOf(type_of) => {
                if matches!(type_of.kind, TypeofKind::Value) {
                    self.write_expression(type_of.value)?;
                    self.push_str(Symbol::Dot.as_str())?;
                    self.push_str(KeyWord::Typeof.as_str())?;
                    return Ok(());
                }
                self.write_expression(type_of.value)?;
                push_fmt!(self, " {TYPEOF_STR} ")?;
                match &type_of.kind {
                    TypeofKind::Null => self.push_str(KeyWord::Null.as_str())?,
                    TypeofKind::NotNull => {
                        push_fmt!(self, "{}{}", Symbol::Not.as_str(), KeyWord::Null.as_str(),)?
                    }
                    TypeofKind::Value => unreachable!(),
                    TypeofKind::Union {
                        type_name,
                        variant_name,
                    } => push_fmt!(self, "{}.{}", type_name.as_str(), variant_name.as_str(),)?,
                };
                Ok(())
            }
            ExpressionKind::Lambda(lambda) => self.write_lambda(lambda),
            ExpressionKind::Break => self.push_str(KeyWord::Break.as_str()),
            ExpressionKind::Continue => self.push_str(KeyWord::Continue.as_str()),
            ExpressionKind::Return(value) => {
                self.push_str(KeyWord::Return.as_str())?;
                self.push_char(' ')?;
                if let Some(value) = value {
                    self.write_expression(*value)?;
                }
                Ok(())
            }
            ExpressionKind::Pass(expression_id) => {
                self.write_expression(*expression_id)?;
                push_fmt!(self, ".{PASS_STR}")
            }
            ExpressionKind::StringFormat(fmt) => {
                let tag = if fmt.to_string { "f" } else { "fstr" };
                push_fmt!(self, "{tag}")?;
                for (text, expr_id) in &fmt.parts {
                    push_fmt!(self, "\"{text}\"")?;
                    self.push_char('{')?;
                    self.write_expression(*expr_id)?;
                    self.push_char('}')?;
                }
                push_fmt!(self, "\"{}\"", fmt.trailing)
            }
            ExpressionKind::Tuple(values) => {
                self.push_str(".(")?;
                let last_index = values.len().saturating_sub(1);
                for (i, value) in values.iter().enumerate() {
                    self.write_expression(*value)?;
                    if i != last_index {
                        self.push_str(", ")?;
                    }
                }
                self.push_char(')')
            }
            ExpressionKind::NamedTuple(values) => {
                self.push_str(".{")?;
                let last_index = values.len().saturating_sub(1);
                for (i, (name, value)) in values.iter().enumerate() {
                    push_fmt!(self, "{}: ", name.as_str())?;
                    self.write_expression(*value)?;
                    if i != last_index {
                        self.push_str(", ")?;
                    }
                }
                self.push_char('}')
            }
        }
    }

    fn write_if_condition(&mut self, condition: &IfCondition) -> Result<()> {
        match condition {
            IfCondition::Expression(value) => self.write_expression(*value),
            IfCondition::CastType {
                binding,
                ty,
                scrutinee,
            } => {
                push_fmt!(self, "type {binding}: ")?;
                self.write_type(ty)?;
                self.push_str(" := ")?;
                self.write_expression(*scrutinee)
            }
            IfCondition::MatchType { pattern, scrutinee } => {
                self.push_str("type ")?;
                self.write_match_pattern(pattern)?;
                self.push_str(" := ")?;
                self.write_expression(*scrutinee)
            }
        }
    }

    fn write_match_pattern(&mut self, arm: &MatchPattern) -> Result<()> {
        match &arm {
            MatchPattern::Fallthrough(chain) => {
                let last_index = chain.len().saturating_sub(1);
                for (i, pattern) in chain.iter().enumerate() {
                    self.write_match_pattern(pattern)?;
                    if i != last_index {
                        self.write_endln()?;
                        self.write_depth()?;
                        self.push_str("| ")?;
                    }
                }
                Ok(())
            }
            MatchPattern::If {
                pattern,
                if_condition,
            } => {
                self.write_match_pattern(pattern)?;
                push_fmt!(self, " {} ", KeyWord::If.as_str())?;
                self.write_expression(*if_condition)?;
                push_fmt!(self, " {}", Symbol::LambdaArrow.as_str())
            }
            MatchPattern::Null => self.push_str(KeyWord::Null.as_str()),
            MatchPattern::NotNull(binding) => {
                push_fmt!(self, "{}{}(", Symbol::Not.as_str(), KeyWord::Null.as_str())?;
                self.push_str(binding.ident.as_str())?;
                self.push_char(')')
            }
            MatchPattern::Wildcard => self.push_str("_"),
            MatchPattern::Literal(literal) => push_fmt!(self, "{literal:?}"),
            MatchPattern::Binding(binding) => self.push_str(binding.ident.as_str()),
            MatchPattern::Array(match_patterns) => {
                self.push_char('[')?;
                let last_index = match_patterns.len().saturating_sub(1);
                for (i, arm) in match_patterns.iter().enumerate() {
                    self.write_match_pattern(arm)?;
                    if i != last_index {
                        self.push_str(", ")?;
                    }
                }
                self.push_char(']')
            }
            MatchPattern::Constructor(ctor) => {
                push_fmt!(self, "{}.{}", ctor.type_name, ctor.variant_name)?;
                if let Some(binding) = &ctor.binding {
                    push_fmt!(self, "({binding})")?;
                }
                Ok(())
            }
            MatchPattern::Tuple(tuple) => {
                self.push_char('(')?;
                for (i, element) in tuple.elements.iter().enumerate() {
                    if i > 0 {
                        self.push_str(", ")?;
                    }
                    self.write_match_pattern(element)?;
                }
                if tuple.rest {
                    if !tuple.elements.is_empty() {
                        self.push_str(", ")?;
                    }
                    self.push_str("..")?;
                }
                self.push_char(')')
            }
            MatchPattern::NamedTuple(named) => {
                self.push_char('{')?;
                for (i, field) in named.fields.iter().enumerate() {
                    if i > 0 {
                        self.push_str(", ")?;
                    }
                    self.push_str(field.field.as_str())?;
                    if let Some(binding) = &field.binding {
                        self.push_str(": ")?;
                        self.push_str(binding.ident.as_str())?;
                    }
                }
                if named.rest {
                    if !named.fields.is_empty() {
                        self.push_str(", ")?;
                    }
                    self.push_str("..")?;
                }
                self.push_char('}')
            }
            MatchPattern::ConstructorStruct(struct_pat) => {
                self.push_str(struct_pat.type_name.as_str())?;
                self.push_char('{')?;
                for (i, field) in struct_pat.fields.iter().enumerate() {
                    if i > 0 {
                        self.push_str(", ")?;
                    }
                    self.push_str(field.field.as_str())?;
                    if let Some(binding) = &field.binding {
                        self.push_str(": ")?;
                        self.push_str(binding.ident.as_str())?;
                    }
                }
                if struct_pat.rest {
                    if !struct_pat.fields.is_empty() {
                        self.push_str(", ")?;
                    }
                    self.push_str("..")?;
                }
                self.push_char('}')
            }
            MatchPattern::Rest => self.push_str(".."),
        }
    }

    fn display_branch(&mut self, if_arm: &Option<IfBranch>) -> Result<()> {
        let mut current = if_arm.as_ref();
        while let Some(arm) = current {
            self.push_char(' ')?;
            match arm {
                IfBranch::If(elif) => {
                    push_fmt!(self, "{ELSE_STR} {IF_STR} ")?;
                    self.write_if_condition(&elif.condition)?;
                    self.write_block(elif.block)?;
                    current = elif.branch.as_ref();
                }
                IfBranch::Else(el) => {
                    push_fmt!(self, "{ELSE_STR} ")?;
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
                self.push_char('(')?;
                let last_index = kind.len().saturating_sub(1);
                match kind {
                    TupleKind::Tuple(types) => {
                        for (i, ty) in types.iter().enumerate() {
                            self.write_type(ty)?;
                            if i != last_index {
                                self.push_str(", ")?;
                            }
                        }
                    }
                    TupleKind::NamedTuple(items) => {
                        for (i, (name, ty)) in items.iter().enumerate() {
                            push_fmt!(self, "{}: ", name.as_str())?;
                            self.write_type(ty)?;
                            if i != last_index {
                                self.push_str(", ")?;
                            }
                        }
                    }
                }
                self.push_char(')')
            }
            SoulType::None => self.push_str(PrimitiveTypes::None.as_str()),
            SoulType::Never => self.push_char('!'),
            SoulType::Primitive(primitive_types) => self.push_str(primitive_types.as_str()),
            SoulType::Array(array) => {
                match array.kind {
                    ArrayKind::StackArrayWildcard => self.push_str("[_]")?,
                    ArrayKind::StackArray(num) => push_fmt!(self, "[{num}]")?,
                    ArrayKind::HeapArray => self.push_str("[]")?,
                    ArrayKind::MutSlice => self.push_str("[&mut]")?,
                    ArrayKind::ConstSlice => self.push_str("[&]")?,
                }
                self.write_type(&array.of_type)
            }
            SoulType::Reference(reference) | SoulType::Pointer(reference) => {
                if matches!(ty, SoulType::Pointer(_)) {
                    self.push_char('*')?;
                } else {
                    self.push_char('&')?;
                }

                if let Some(lifetime) = &reference.lifetime {
                    push_fmt!(self, "'{} ", lifetime.as_str())?;
                }
                if reference.mutable {
                    self.push_str("mut ")?;
                }
                self.write_type(&reference.inner)
            }
            SoulType::RawPtr(inner) => {
                self.push_str("RawPtr")?;
                if let Some(inner) = inner {
                    self.push_char('<')?;
                    self.write_type(inner)?;
                    self.push_char('>')?;
                }
                Ok(())
            }
            SoulType::Res { ok, err } => {
                self.push_str("Res")?;
                match (ok, err) {
                    (Some(ok), Some(err)) => {
                        self.push_char('<')?;
                        self.write_type(ok)?;
                        self.push_str(", ")?;
                        self.write_type(err)?;
                        self.push_char('>')?;
                    }
                    (Some(ok), None) => {
                        self.push_char('<')?;
                        self.write_type(ok)?;
                        self.push_char('>')?;
                    }
                    _ => {}
                }
                Ok(())
            }
            SoulType::Optional(soul_type) => {
                self.push_char('?')?;
                self.write_type(soul_type)
            }
            SoulType::ImplTrait(inner) => {
                self.push_str("impl ")?;
                self.write_type(inner)
            }
            SoulType::Stub(stub) => {
                self.push_str(&stub.name)?;
                self.write_generic_types(&stub.generics)?;
                Ok(())
            }
            SoulType::NamedVariant { base, variant } => {
                self.write_type(base)?;
                push_fmt!(self, ".{}", variant.as_str())
            }
            SoulType::String => self.push_str(Types::String.as_str()),
            SoulType::FormatString => self.push_str(Types::FormatString.as_str()),
            SoulType::Any => self.push_str(Types::Any.as_str()),
            SoulType::Error => self.push_str(Types::Error.as_str()),
        }
    }

    fn write_any_array(&mut self, any_array: &AnyArray) -> Result<()> {
        match any_array {
            AnyArray::Array(array) => {
                if let Some(ty) = &array.collection_type {
                    self.write_type(ty)?;
                    self.push_char('.')?;
                }
                self.push_char('[')?;
                if let Some(ty) = &array.element_type {
                    self.write_type(ty)?;
                    self.push_str(": ")?;
                }
                let last_index = array.values.len().saturating_sub(1);
                for (i, value) in array.values.iter().enumerate() {
                    self.write_expression(*value)?;
                    if i != last_index {
                        self.push_str(", ")?;
                    }
                }
                self.push_char(']')
            }
            AnyArray::ArrayFiller(array) => {
                if let Some(collection) = &array.collection_type {
                    self.write_type(collection)?;
                    self.push_char('.')?;
                }
                self.push_char('[')?;
                if let Some(ty) = &array.element_type {
                    self.write_type(ty)?;
                    self.push_str(": ")?;
                }
                push_fmt!(self, "{FOR_STR} ")?;
                if let Some(binding) = &array.for_index {
                    push_fmt!(self, "{} {IN_FOR_LOOP_STR} ", binding.ident.as_str())?;
                }
                self.write_expression(array.amount)?;
                push_fmt!(self, " {LAMDA_ARROW_STR} ")?;
                self.write_expression(array.element)?;
                self.push_char(']')
            }
        }
    }

    fn write_generic_defines(&mut self, generics: &[Generic]) -> Result<()> {
        if generics.is_empty() {
            return Ok(());
        }

        self.push_char('<')?;
        let last_index = generics.len().saturating_sub(1);
        for (i, generic) in generics.iter().enumerate() {
            self.push_str(generic.name.as_str())?;
            if let Some(bound) = &generic.bound {
                self.push_str(": ")?;
                self.write_type(bound)?;
            }

            if i != last_index {
                self.push_str(", ")?;
            }
        }
        self.push_char('>')
    }

    fn write_generic_types(&mut self, generics: &[SoulType]) -> Result<()> {
        if generics.is_empty() {
            return Ok(());
        }

        self.push_char('<')?;
        let last_index = generics.len().saturating_sub(1);
        for (i, generic) in generics.iter().enumerate() {
            self.write_type(generic)?;
            if i != last_index {
                self.push_str(", ")?;
            }
        }
        self.push_char('>')
    }

    fn write_endln(&mut self) -> Result<()> {
        self.writer.push_char('\n')?;
        Ok(())
    }

    fn write_depth(&mut self) -> Result<()> {
        self.writer.push_str(&self.depth)?;
        Ok(())
    }

    fn push_depth(&mut self) {
        self.depth.push('\t');
    }

    fn pop_depth(&mut self) {
        self.depth.pop();
    }

    fn push_fmt(&mut self, args: Arguments<'_>) -> Result<()> {
        self.writer.push_fmt(args)?;
        Ok(())
    }

    fn push_str(&mut self, str: &str) -> Result<()> {
        self.writer.push_str(str)?;
        Ok(())
    }

    fn push_char(&mut self, ch: char) -> Result<()> {
        self.writer.push_char(ch)?;
        Ok(())
    }
}

trait GetErr<I, V> {
    fn get_err(&self, index: I) -> Result<&V>;
}
impl<I: VecMapIndex + Debug + Clone, V> GetErr<I, V> for VecMap<I, V> {
    fn get_err(&self, index: I) -> Result<&V> {
        self.get(index.clone()).ok_or(anyhow::Error::msg(format!(
            "{index:?} is not found; {}\n",
            Backtrace::force_capture()
        )))
    }
}

trait AnyIdString {
    fn get_any_id_string(&self) -> Option<String>;
}

impl AnyIdString for StatementKind {
    fn get_any_id_string(&self) -> Option<String> {
        Some(match self {
            StatementKind::Variable(variable) => format!("{:?}", variable.id),
            StatementKind::Expression { expression, .. } => format!("{:?}", expression),
            StatementKind::Function(function_id) | StatementKind::ExternalFunction(function_id) => {
                format!("{:?}", function_id)
            }
            StatementKind::Enum(enum_) => format!("{:?}", enum_.id),
            StatementKind::Trait(trait_) => format!("{:?}", trait_.id),
            StatementKind::Struct(struct_) => format!("{:?}", struct_.id),
            _ => return None,
        })
    }
}
