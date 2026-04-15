use std::fmt::{Arguments, Debug, Write};

use ast::{
    Assignment, AstContext, Block, ElseKind, Expression, Function, FunctionSignature, Generic, IfArm, Import, SoulType, Statement, StatementKind, Struct, TypeKind, UseBlock, Variable, scope::{NodeId, ScopeId}
};
use soul_utils::{
    ids::FunctionId, sementic_level::CompilerContext, soul_names::{KeyWord, Operator, TypeModifier}, span::ModuleId, vec_map::VecMapIndex
};

pub fn display_ast(root: ModuleId, context: &CompilerContext, ast_context: &AstContext) -> String {
    let mut displayer = AstDisplayer::new_ast(context, ast_context);
    displayer.display_module(root);   
    for (_id, path) in context.module_store.entries() {
        displayer.push_fmt(format_args!("\nmod {:?}", path));
    }

    displayer.consume_to_string()
}

pub fn display_ast_name_resolved(root: ModuleId, context: &CompilerContext, ast_context: &AstContext) -> String {
    let mut displayer = AstDisplayer::new_name_resolved(context, ast_context);
    displayer.display_module(root);    
    displayer.consume_to_string()
}

const MUT: bool = true;
const CONST: bool = false;

struct AstDisplayer<'a> {
    sb: String,
    depth: String,
    is_name_resolved: bool,
    ast_context: &'a AstContext,
    context: &'a CompilerContext,
}
impl<'a> AstDisplayer<'a> {
    fn new_ast(context: &'a CompilerContext, ast_context: &'a AstContext) -> Self {
        Self {
            context,
            ast_context,
            sb: String::new(),
            is_name_resolved: false,
            depth: String::with_capacity(10),
        }
    }

    fn new_name_resolved(context: &'a CompilerContext, ast_context: &'a AstContext) -> Self {
        Self {
            context,
            ast_context,
            sb: String::new(),
            is_name_resolved: true,
            depth: String::with_capacity(10),
        }
    }

    fn push(&mut self, ch: char) {
        self.sb.push(ch);
    }

    fn push_str(&mut self, text: &str) {
        self.sb.push_str(text);
    }

    fn push_fmt<'b>(&mut self, args: Arguments<'b>) {
        self.sb.write_fmt(args).expect("no ftm error")
    }

    fn display_depth(&mut self) {
        self.sb.push_str(&self.depth);
    }

    fn push_scope(&mut self) {
        self.depth.push('\t');
    }

    fn pop_scope(&mut self) {
        let res = self.depth.pop();
        debug_assert!(res.is_some());
    }

    fn display_module(&mut self, module_id: ModuleId) {
        let module = &self.ast_context.modules[module_id];
        
        self.display_depth();
        self.push_fmt(format_args!("mod {} {{\n", module.name));
        self.push_scope();
        for statement in &module.global.statements {
            self.display_statement(statement);
            self.push('\n');
        }

        for module in &module.modules {
            self.display_module(*module);
        }

        self.pop_scope();
        self.display_depth();
        self.push_str("}\n");
    }

    fn display_block(&mut self, block: &Block) {
        self.try_display_node_id(block.node_id);
        if block.modifier != TypeModifier::Mut {
            self.push(' ');
            self.push_str(block.modifier.as_str());
        }
        self.push_str(" {");
        self.try_display_scope_id(block.scope_id);
        self.push('\n');

        self.push_scope();
        for statement in &block.statements {
            self.display_statement(statement);
            self.push('\n');
        }
        self.pop_scope();
        self.display_depth();
        self.push_str("}\n");
    }

    fn display_statement(&mut self, statement: &Statement) {
        self.display_tag(statement.display_variant(), statement.node.get_id());

        match &statement.node {
            ast::StatementKind::Struct(obj) => self.display_struct(obj),
            ast::StatementKind::Import(import) => self.display_import(import),
            ast::StatementKind::Variable(variable) => self.display_variable(variable),
            ast::StatementKind::UseBlock(use_block) => self.display_use_block(use_block),
            ast::StatementKind::Assignment(assignment) => self.display_assignment(assignment),
            ast::StatementKind::Function(function)
            | ast::StatementKind::ExternalFunction(function) => self.display_function(function),
            ast::StatementKind::Expression {
                expression,
                ends_semicolon,
                ..
            } => {
                self.display_expression(expression);
                if *ends_semicolon {
                    self.push(';');
                }
            }
        }
    }

    fn display_assignment(&mut self, assignment: &Assignment) {
        self.display_expression(&assignment.left);
        self.push_str(" = ");
        self.display_expression(&assignment.right);
    }

    fn display_use_block(&mut self, use_block: &UseBlock) {
        self.push_str(KeyWord::Use.as_str());
        self.display_generic_declare(&use_block.generics);
        self.push(' ');
        self.display_type(&use_block.use_type);
        self.push_str(" {\n");
        self.push_scope();
        for methode in &use_block.methodes {
            self.display_tag("Function", methode.signature.node.id);
            self.display_function(methode);
            self.push('\n');
        }
        for impl_block in &use_block.impls {
            self.display_depth();
            self.push_str(KeyWord::Impl.as_str());
            self.display_type(&impl_block.impl_trait);
            for methode in &impl_block.methodes {
                self.display_tag("Function", methode.signature.node.id);
                self.display_function(methode);
                self.push('\n');
            }
        }
        self.pop_scope();
        self.display_depth();
        self.push_str("}\n");
    }

    fn display_function(&mut self, function: &Function) {
        self.display_function_signature(&function.signature.node);
        if function.signature.node.external.is_some() {
            self.push('\n');
            return;
        }
        self.display_block(&function.block);
    }

    fn display_function_signature(&mut self, signature: &FunctionSignature) {
        if let Some(language) = &signature.external {
            let keyword = KeyWord::Extern.as_str();
            self.push_fmt(format_args!("{keyword} \"{}\" ", language.as_str()));
        }

        let methode_type = &signature.methode_type;
        if methode_type
            .modifier
            .is_some_and(|modifier| modifier != TypeModifier::Mut)
        {
            self.push_str(methode_type.modifier.unwrap().as_str());
            self.push(' ');
        }

        if methode_type.kind != TypeKind::None {
            self.display_typekind(&methode_type.kind);
            self.push(' ');
        }

        self.push_str(signature.name.as_str());
        self.display_generic_declare(&signature.generics);
        self.push('(');
        match signature.function_kind {
            ast::FunctionKind::Static => (),
            ast::FunctionKind::MutRef => self.push_str("&this"),
            ast::FunctionKind::Consume => self.push_str("this"),
            ast::FunctionKind::ConstRef => self.push_str("@this"),
        };
        let has_this = signature.function_kind != ast::FunctionKind::Static;
        if has_this && !signature.parameters.is_empty() {
            self.push_str(", ");
        }

        let last_index = signature.parameters.len().saturating_sub(1);
        for (i, param) in signature.parameters.iter().enumerate() {
            self.try_display_node_id(param.node_id);
            self.push_str(param.ty.modifier.unwrap_or(TypeModifier::Const).as_str());
            self.push(' ');
            self.push_str(param.name.as_str());
            self.push_str(": ");
            self.display_typekind(&param.ty.kind);
            if let Some(value) = &param.default {
                self.push_str(" = ");
                self.display_expression(value);
            }
            if i != last_index {
                self.push_str(", ");
            }
        }

        self.push(')');

        if signature.return_type.kind != TypeKind::None {
            self.push_str(": ");
            self.display_type(&signature.return_type);
        }
    }

    fn display_if_arm(&mut self, if_arm: &Option<IfArm>) {
        let mut current = if_arm.as_ref();
        while let Some(arm) = current {
            self.display_depth();
            match &arm.node {
                ElseKind::ElseIf(elif) => {
                    self.push_str(KeyWord::Else.as_str());
                    self.push(' ');
                    self.push_str(KeyWord::If.as_str());
                    self.push(' ');
                    self.display_expression(&elif.node.condition);
                    self.display_block(&elif.node.block);
                    current = elif.node.else_branchs.as_ref();
                }
                ElseKind::Else(el) => {
                    self.push_str(KeyWord::Else.as_str());
                    self.push(' ');
                    self.display_block(&el.node);
                    current = None;
                }
            }
        }
    }

    fn display_variable(&mut self, variable: &Variable) {
        let modifier = match &variable.ty {
            ast::VarTypeKind::InveredType(modifier) => *modifier,
            ast::VarTypeKind::NonInveredType(ty) => ty.modifier.unwrap_or(TypeModifier::Const),
        };
        self.push_str(modifier.as_str());
        self.push(' ');
        self.push_str(variable.name.as_str());
        self.push_str(": ");
        match &variable.ty {
            ast::VarTypeKind::InveredType(_) => self.push_str("/*?type*/"),
            ast::VarTypeKind::NonInveredType(ty) => self.display_typekind(&ty.kind),
        }

        if let Some(value) = &variable.initialize_value {
            self.push_str(" = ");
            self.display_expression(value);
        }
    }

    fn display_import(&mut self, import: &Import) {
        const SEPERATOR: &str = ".";

        self.push_str(KeyWord::Import.as_str());
        for path in &import.paths {
            self.push(' ');
            if let Err(_) = path.module.write_display(&self.context.source_folder, &mut self.sb) {
                self.push_str("<error>");
            }
            match &path.kind {
                ast::ImportKind::All => self.push('*'),
                ast::ImportKind::This => self.push_fmt(format_args!("{SEPERATOR}this")),
                ast::ImportKind::Items(items) => {
                    self.push_str(".{");
                    let last_index = items.len().saturating_sub(1);
                    for (i, item) in items.iter().enumerate() {
                        self.push_str(item.as_str());
                        if i != last_index {
                            self.push_str(", ");
                        }
                    }
                    self.push('}');
                }
                ast::ImportKind::Glob => self.push('*'),
                ast::ImportKind::Alias(alias) => {
                    self.push_fmt(format_args!(" as {}", alias.as_str()));
                }
            }
        }
    }

    fn display_struct(&mut self, obj: &Struct) {
        self.push_str(KeyWord::Struct.as_str());
        self.push(' ');
        self.push_str(obj.name.as_str());
        self.display_generic_declare(&obj.generics);
        self.push_str(" {\n");
        self.push_scope();
        for field in &obj.fields {
            self.display_tag("Field", field.id);
            self.push_str(field.ty.modifier.unwrap_or(TypeModifier::Const).as_str());
            self.push(' ');
            self.push_str(field.name.as_str());
            self.push_str(": ");
            self.display_typekind(&field.ty.kind);
            self.push('\n');
        }
        self.pop_scope();
        self.display_depth();
        self.push_str("}\n");
    }

    fn display_expression(&mut self, expression: &Expression) {
        match &expression.node {
            ast::ExpressionKind::If(r#if) => {
                self.try_display_node_id(r#if.id);
                self.push_str(KeyWord::If.as_str());
                self.push(' ');
                self.display_expression(&r#if.condition);
                self.display_block(&r#if.block);
                self.display_if_arm(&r#if.else_branchs);
            }
            ast::ExpressionKind::Index(index) => {
                self.try_display_node_id(index.id);
                self.display_expression(&index.collection);
                self.push('[');
                self.display_expression(&index.index);
                self.push(']');
            }
            ast::ExpressionKind::Unary(unary) => {
                self.push_str(unary.operator.node.as_str());
                self.display_expression(&unary.expression);
            }
            ast::ExpressionKind::Array(array) => {
                self.try_display_node_id(array.id);
                if let Some(collection) = &array.collection_type {
                    self.display_type(collection);
                    self.push(':');
                }
                self.push('[');
                if let Some(element) = &array.element_type {
                    self.display_type(element);
                    self.push(':');
                }

                let last_index = array.values.len().saturating_sub(1);
                for (i, value) in array.values.iter().enumerate() {
                    self.display_expression(value);
                    if i != last_index {
                        self.push_str(", ");
                    }
                }
                self.push(']');
            }
            ast::ExpressionKind::Block(block) => self.display_block(block),
            ast::ExpressionKind::Null(_) => self.push_str("null"),
            ast::ExpressionKind::While(r#while) => {
                self.try_display_node_id(r#while.id);
                self.push_str(KeyWord::While.as_str());
                self.push(' ');
                if let Some(condition) = &r#while.condition {
                    self.display_expression(condition);
                }
                self.push_str("{\n");
            }
            ast::ExpressionKind::Binary(binary) => {
                self.push('(');
                self.display_expression(&binary.left);
                self.push(' ');
                self.push_str(binary.operator.node.as_str());
                self.push(' ');
                self.display_expression(&binary.right);
                self.push(')');
            }
            ast::ExpressionKind::Literal((_, literal)) => {
                self.push_str(&literal.value_to_string());
            }
            ast::ExpressionKind::As(cast) => {
                self.display_expression(&cast.left);
                self.push_str(" as ");
                self.display_type(&cast.type_cast);
            }
            ast::ExpressionKind::Default(_) => self.push_str("()"),
            ast::ExpressionKind::Sizeof(soul_type) => {
                self.display_type(soul_type);
                self.push_str(".sizeof");
            }
            ast::ExpressionKind::Deref { inner, .. } => {
                self.push('*');
                self.display_expression(inner);
            }
            ast::ExpressionKind::ReturnLike(return_like) => {
                self.push_str(return_like.kind.as_keyword().as_str());
                if let Some(value) = &return_like.value {
                    self.push(' ');
                    self.display_expression(value);
                }
            }
            ast::ExpressionKind::FieldAccess(field_access) => {
                self.try_display_node_id(field_access.id);
                self.display_expression(&field_access.object);
                self.push('.');
                self.push_str(field_access.field.as_str());
            }
            ast::ExpressionKind::FunctionCall(function_call) => {
                if let Some(callee) = &function_call.callee {
                    self.display_expression(callee);
                    self.push('.');
                }
                self.push_str(function_call.name.as_str());
                self.display_generic_define(&function_call.generics);
                self.push('(');
                let last_index = function_call.arguments.len().saturating_sub(1);
                for (i, arg) in function_call.arguments.iter().enumerate() {
                    if let Some(name) = &arg.name {
                        self.push_str(name.as_str());
                        self.push_str(": ");
                    }
                    self.display_expression(&arg.value);
                    if i != last_index {
                        self.push_str(", ");
                    }
                }
                self.push(')');
            }
            ast::ExpressionKind::Variable {
                ident, resolved, ..
            } => {
                self.try_display_node_id(*resolved);
                self.push_str(ident.as_str());
            }
            ast::ExpressionKind::ArrayContructor(ctor) => {
                if let Some(collection) = &ctor.collection_type {
                    self.display_type(collection);
                    self.push_str(": ");
                }
                self.push('[');
                if let Some(element) = &ctor.element_type {
                    self.display_type(element);
                    self.push_str(": ");
                }
                self.push_str(KeyWord::For.as_str());
                self.push(' ');
                self.display_expression(&ctor.amount);
                self.push_str(" => ");
                self.display_expression(&ctor.element);
                self.push(']');
            }
            ast::ExpressionKind::Ref {
                expression,
                is_mutable,
                ..
            } => {
                let str = match *is_mutable {
                    MUT => Operator::ConstRef.as_str(),
                    CONST => Operator::BitAnd.as_str(),
                };
                self.push_str(str);
                self.display_expression(expression);
            }
            ast::ExpressionKind::StructConstructor(ctor) => {
                self.display_type(&ctor.struct_type);
                self.push('{');
                let last_index = ctor.values.len().saturating_sub(1);
                for (i, (name, value)) in ctor.values.iter().enumerate() {
                    self.push_str(name.as_str());
                    self.push_str(": ");
                    self.display_expression(value);
                    if ctor.defaults || i != last_index {
                        self.push_str(", ");
                    }
                }
                if ctor.defaults {
                    self.push_str("..");
                }
                self.push('}');
            }
            ast::ExpressionKind::ExternalExpression(external_expression) => {
                self.push_str(
                    external_expression
                        .path
                        .as_path()
                        .to_str()
                        .unwrap_or("<error>"),
                );
                self.push('.');
                self.display_expression(&external_expression.expr);
            }
        }
    }

    fn display_type(&mut self, ty: &SoulType) {
        if let Some(modifier) = ty.modifier {
            self.push_str(modifier.as_str());
            self.push(' ');
        }

        self.display_typekind(&ty.kind);
    }

    fn display_typekind(&mut self, kind: &TypeKind) {
        match kind {
            ast::TypeKind::None => self.push_str("none"),
            ast::TypeKind::Type => self.push_str("type"),
            ast::TypeKind::Stub(stub) => self.push_str(&stub.name),
            ast::TypeKind::Primitive(primitive_types) => self.push_str(primitive_types.as_str()),
            ast::TypeKind::Array(array_type) => {
                match array_type.kind {
                    ast::ArrayKind::MutSlice => self.push_str("[&]"),
                    ast::ArrayKind::HeapArray => self.push_str("[*]"),
                    ast::ArrayKind::ConstSlice => self.push_str("[@]"),
                    ast::ArrayKind::StackArray(len) => self.push_fmt(format_args!("[{len}]")),
                }
                self.display_type(&array_type.of_type);
            }
            ast::TypeKind::Pointer(soul_type) => {
                self.push('*');
                self.display_type(soul_type);
            }
            ast::TypeKind::Optional(soul_type) => {
                self.push('?');
                self.display_type(soul_type);
            }
            ast::TypeKind::Reference(reference_type) => {
                match reference_type.mutable {
                    MUT => self.push('&'),
                    CONST => self.push('@'),
                };
                if let Some(name) = &reference_type.lifetime {
                    self.push_fmt(format_args!("'{}", name.as_str()));
                }
                self.display_type(&reference_type.inner);
            }
        }
    }

    fn display_tag<ID: VecMapIndex + Debug>(&mut self, str: &str, node_id: Option<ID>) {
        self.display_depth();
        if !self.is_name_resolved {
            self.push_fmt(format_args!("/*{str}*/"));
            self.push('\n');
            self.display_depth();
            return;
        }

        match node_id {
            Some(id) => self.push_fmt(format_args!("/*{str}: {:?}*/", id)),
            None => self.push_fmt(format_args!("/*{str}*/")),
        }
        self.push('\n');
        self.display_depth();
    }

    fn try_display_node_id(&mut self, node_id: Option<NodeId>) {
        if !self.is_name_resolved {
            return;
        }

        if let Some(id) = node_id {
            self.push_fmt(format_args!("/*{:?}*/", id))
        }
    }

    fn display_generic_declare(&mut self, generics: &[Generic]) {
        if !generics.is_empty() {
            self.push('<');
            let last_index = generics.len().saturating_sub(1);
            for (i, generic) in generics.iter().enumerate() {
                self.push_str(generic.name.as_str());
                if i != last_index {
                    self.push_str(", ");
                }
            }
            self.push('>');
        }
    }

    fn display_generic_define(&mut self, generics: &[SoulType]) {
        if !generics.is_empty() {
            self.push('<');
            let last_index = generics.len().saturating_sub(1);
            for (i, generic) in generics.iter().enumerate() {
                self.display_type(generic);
                if i != last_index {
                    self.push_str(", ");
                }
            }
            self.push('>');
        }
    }

    fn try_display_scope_id(&mut self, scope_id: Option<ScopeId>) {
        if !self.is_name_resolved {
            return;
        }

        if let Some(id) = scope_id {
            self.push_fmt(format_args!("/*{:?}*/", id));
        }
    }

    fn consume_to_string(self) -> String {
        self.sb
    }
}

trait StatementKindHelper {
    fn get_id(&self) -> Option<StatementIdKind>;
}
impl StatementKindHelper for StatementKind {
    fn get_id(&self) -> Option<StatementIdKind> {
        match self {
            StatementKind::Struct(obj) => obj.id.to_statement_kind(),
            StatementKind::Import(import) => import.id.to_statement_kind(),
            StatementKind::Variable(variable) => variable.node_id.to_statement_kind(),
            StatementKind::Expression { id, expression, .. } => match &expression.node {
                ast::ExpressionKind::FunctionCall(call) => call.resolved.to_statement_kind(),
                _ => (*id).to_statement_kind(),
            },
            StatementKind::Assignment(assignment) => assignment.node_id.to_statement_kind(),

            StatementKind::ExternalFunction(func) | StatementKind::Function(func) => {
                func.signature.node.id.to_statement_kind()
            }
            StatementKind::UseBlock(_) => None,
        }
    }
}
/// mainly just for displayer
pub enum StatementIdKind {
    NodeId(NodeId),
    FunctionId(FunctionId),
}
impl VecMapIndex for StatementIdKind {
    fn new_index(_value: usize) -> Self {
        panic!("stub impl")
    }

    fn index(&self) -> usize {
        panic!("stub impl")
    }
}
impl Debug for StatementIdKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeId(id) => id.fmt(f),
            Self::FunctionId(id) => id.fmt(f),
        }
    }
}
trait ToStatementKind {
    fn to_statement_kind(self) -> Option<StatementIdKind>;
}
impl ToStatementKind for Option<NodeId> {
    fn to_statement_kind(self) -> Option<StatementIdKind> {
        self.map(StatementIdKind::NodeId)
    }
}
impl ToStatementKind for Option<FunctionId> {
    fn to_statement_kind(self) -> Option<StatementIdKind> {
        self.map(StatementIdKind::FunctionId)
    }
}

trait StatementHelper {
    fn display_variant(&self) -> &str;
}
impl StatementHelper for Statement {
    fn display_variant(&self) -> &str {
        if let StatementKind::Expression { expression, .. } = &self.node {
            expression.node.variant_str()
        } else {
            self.node.variant_name()
        }
    }
}
