use hir::{
    Binary, BlockId, DisplayType, ExpressionId, FunctionBody, HirTree, HirType, LazyTypeId, LocalId, LocalKind, StructId, Unary
};
use soul_utils::{
    ids::{FunctionId, IdAlloc},
    soul_names::KeyWord,
    vec_map::VecMapIndex,
};
use typed_hir::{TypedHir, display_thir::DisplayThirType};
use std::fmt::Write;

pub fn display_hir(hir: &HirTree) -> String {
    let mut displayer = HirDisplayer::new_hir(hir);

    for global in &hir.root.globals {
        displayer.display_global(global);
    }

    displayer.to_string()
}

pub fn display_thir(hir: &HirTree, typed: &TypedHir) -> String {
    let mut displayer = HirDisplayer::new_thir(hir, typed);

    for global in &hir.root.globals {
        displayer.display_global(global);
    }

    displayer.to_string()
}

pub fn display_created_types(hir: &HirTree, typed: &TypedHir) -> String {
    
    let mut sb = String::new();
    for (id, struct_type) in typed.types_map.structs.entries() {
        let name = &hir.info.types.id_to_struct(id).expect("should have struct").name;
        sb.push_str("struct ");
        sb.push_str(name.as_str());
        sb.push_str(" {\n");
        for field in &struct_type.fields {
            let field_name = &hir.nodes.fields[field.id].name;
            sb.push('\t');
            sb.push_str(field_name);
            sb.push_str(": ");
            typed.types_map
                .id_to_type(field.ty)
                .expect(&format!("{:?} not found", field.ty))
                .write_display(&typed.types_map, &mut sb)
                .expect("no fmt error");
            sb.push('\n');
        }
        sb.push_str("}\n");
    }

    sb
}

struct HirDisplayer<'a> {
    sb: String,
    hir: &'a HirTree,
    typed: Option<&'a TypedHir>,

    depth: usize,
    terminate: Option<ExpressionId>,
}
impl<'a> HirDisplayer<'a> {
    fn new_hir(hir: &'a HirTree) -> Self {
        Self {
            hir,
            depth: 0,
            typed: None,
            terminate: None,
            sb: String::new(),
        }
    }

    fn new_thir(hir: &'a HirTree, typed: &'a TypedHir) -> Self {
        Self {
            hir,
            depth: 0,
            terminate: None,
            sb: String::new(),
            typed: Some(typed),
        }
    }

    fn display_global(&mut self, global: &hir::Global) {
        match &global.kind {
            hir::GlobalKind::Function(function) => self.display_function(*function),
            hir::GlobalKind::Variable(variable) => self.display_variable(variable),
            hir::GlobalKind::InternalAssign(assign) => {
                self.push_str("/*internal*/");
                self.display_assign(assign);
            }
            hir::GlobalKind::InternalVariable(variable) => {
                self.push_str("/*internal*/");
                self.display_variable(variable);
            }
        }
        self.push('\n');
    }

    fn display_function(&mut self, function_id: FunctionId) {
        let function = &self.hir.nodes.functions[function_id];
        self.push('\n');
        if let hir::FunctionBody::External(id) = function.body {
            self.push_str("extern \"");
            self.push_str(id.as_str());
            self.push_str("\" ");
        }
        self.push_str(function.name.as_str());
        self.push('(');

        let last_index = function.parameters.len().saturating_sub(1);
        for (i, arg) in function.parameters.iter().enumerate() {
            self.display_local(arg.local);
            self.push_str(": ");
            self.display_type(arg.ty);
            if i != last_index {
                self.push_str(", ");
            }
        }
        self.push_str("): ");

        self.display_type(function.return_type.to_lazy());
        if let FunctionBody::Internal(body) = &function.body {
            self.push(' ');
            self.display_block(body);
        }
    }

    fn display_variable(&mut self, variable: &hir::Variable) {
        let local_info = &self.hir.nodes.locals[variable.local];
        if local_info.is_temp() {
            self.display_temp(variable.local);
        } else {
            self.display_local(variable.local);
        }

        self.push_str(": ");
        let local_type = match self.typed {
            Some(typed) => typed.types_table.locals[variable.local].to_lazy(),
            None => local_info.ty,
        };

        self.display_type(local_type);
        if let LocalKind::Variable(Some(value)) = &local_info.kind {
            self.push_str(" := ");
            self.display_expression(value);
        }
    }

    fn display_assign(&mut self, assign: &hir::Assign) {
        self.display_place(&assign.place);
        self.push_str(" = ");
        self.display_expression(&assign.value);
    }

    fn display_block(&mut self, id: &hir::BlockId) {
        let block = &self.hir.nodes.blocks[*id];

        let prev = self.terminate;
        self.terminate = block.terminator;

        self.display_block_id(*id);
        self.push_str("{\n");

        self.depth += 1;
        for node in &block.statements {
            self.push_str(&"\t".repeat(self.depth));
            self.display_statement(node);
            self.push('\n');
        }
        self.depth -= 1;

        self.push_str(&"\t".repeat(self.depth));
        self.push('}');

        self.terminate = prev;
    }

    fn display_statement(&mut self, node: &hir::Statement) {
        match &node.kind {
            hir::StatementKind::Assign(assign) => self.display_assign(assign),
            hir::StatementKind::Variable(variable) => self.display_variable(variable),
            hir::StatementKind::Fall(expression_id) => {
                self.push_str("fall ");
                if let Some(value) = expression_id {
                    self.display_expression(value);
                }
            }
            hir::StatementKind::Break => {
                self.push_str("break");
            }
            hir::StatementKind::Return(expression_id) => {
                self.push_str("return ");
                if let Some(value) = expression_id {
                    self.display_expression(value);
                }
            }
            hir::StatementKind::Continue => self.push_str(KeyWord::Continue.as_str()),
            hir::StatementKind::Expression {
                value,
                ends_semicolon,
            } => {
                if self.terminate == Some(*value) {
                    self.push_str("/*terminate*/");
                }
                self.display_expression(value);
                if *ends_semicolon {
                    self.push(';');
                }
            }
        }
    }

    fn display_expression(&mut self, id: &hir::ExpressionId) {
        let value = &self.hir.nodes.expressions[*id];

        match &value.kind {
            hir::ExpressionKind::Error => self.push_str("<error>"),
            hir::ExpressionKind::Block(block_id) => self.display_block(block_id),
            hir::ExpressionKind::Null => self.push_str("null"),
            hir::ExpressionKind::Literal(literal) => self.push_str(&literal.value_to_string()),
            hir::ExpressionKind::Local(local_id) => self.display_local(*local_id),
            hir::ExpressionKind::Function(_) => self.push_str("<function>"),
            hir::ExpressionKind::StructConstructor { ty, values, defaults } => {
            
                self.display_struct_name(*ty);
                self.push('{');
                let last_index = values.len().saturating_sub(1);
                for (i, (name, value)) in values.iter().enumerate() {
                    self.push_str(name.as_str());
                    self.push_str(": ");
                    self.display_expression(value);
                    if i != last_index {
                        self.push_str(", ");
                    }
                }
                if *defaults {
                    self.push_str(", ..");
                }
                self.push('}');
            }
            hir::ExpressionKind::Load(place) => {
                self.push_str("/*Load*/");
                self.display_place(place);
            }
            hir::ExpressionKind::Ref { place, mutable } => {
                self.push(if *mutable { '&' } else { '@' });
                self.display_place(place);
            }
            hir::ExpressionKind::DeRef(expression_id) => {
                self.push('*');
                self.display_expression(expression_id);
                self.display_expression_astype(*id, value.ty);
            }
            hir::ExpressionKind::Unary(Unary {
                operator,
                expression,
            }) => {
                self.push_str(operator.node.as_str());
                self.display_expression(expression);
                self.display_expression_astype(*id, value.ty);
            }
            hir::ExpressionKind::Binary(Binary {
                left,
                operator,
                right,
            }) => {
                self.push('(');
                self.display_expression(left);
                self.push_str(operator.node.as_str());
                self.display_expression(right);
                self.push(')');
                self.display_expression_astype(*id, value.ty);
            }
            hir::ExpressionKind::If {
                condition,
                then_block,
                else_block,
                ends_with_else: _,
            } => {
                self.push_str("if ");
                self.display_expression(condition);
                self.push(' ');
                self.display_block(then_block);
                if let Some(arm) = else_block {
                    self.push('\n');
                    self.push_str(&"\t".repeat(self.depth));
                    self.push_str("else ");
                    self.display_block(arm);
                }
                self.display_expression_astype(*id, value.ty);
                self.push('\n');
            }
            hir::ExpressionKind::While { condition, body } => {
                self.push_str("while ");
                if let Some(value) = condition {
                    self.display_expression(value);
                    self.push(' ');
                }
                self.display_block(body);
                self.display_expression_astype(*id, value.ty);
                self.push('\n');
            }
            hir::ExpressionKind::Call {
                function,
                callee,
                generics,
                arguments,
            } => {
                if let Some(value) = callee {
                    self.display_expression(value);
                    self.push('.');
                }

                self.display_call_id(*function);
                if !generics.is_empty() {
                    self.push('<');
                    let last_index = generics.len().saturating_sub(1);
                    for (i, generic) in generics.iter().enumerate() {

                        self.display_type(generic.to_lazy());
                        if i != last_index {
                            self.push_str(", ");
                        }
                    }
                    self.push('>');
                }
                self.push('(');
                let last_index = arguments.len().saturating_sub(1);
                for (i, arg) in arguments.iter().enumerate() {
                    self.display_expression(arg);
                    if i != last_index {
                        self.push_str(", ");
                    }
                }
                self.push(')');
                self.display_expression_astype(*id, value.ty);
            }
            hir::ExpressionKind::Cast { value, cast_to } => {
                self.display_expression(value);
                self.push_str(" as ");
                let cast_to = match self.typed {
                    Some(typed) => typed.types_table.expressions[*id].to_lazy(),
                    None => *cast_to,
                };
                self.display_type(cast_to);
            }
            hir::ExpressionKind::InnerRawStackArray(ty) => {
                self.push_str("/*stack alloc ");
                self.display_expression_astype(*id, *ty);
                self.push_str("*/");
            }
        };
    }

    fn display_place(&mut self, place: &hir::PlaceId) {
        match &self.hir.nodes.places[*place].kind {
            hir::PlaceKind::Temp(local_id) => self.display_temp(*local_id),
            hir::PlaceKind::Local(local_id) => self.display_local(*local_id),
            hir::PlaceKind::Deref(place) => {
                self.push('*');
                self.display_place(place);
            }
            hir::PlaceKind::Index { base, index, .. } => {
                self.display_place(base);
                self.push('[');
                self.display_expression(index);
                self.push(']');
            }
            hir::PlaceKind::Field { base, field: index, .. } => {
                self.display_place(base);
                self.push('.');
                self.push_str(index.as_str());
            }
        }
    }

    fn display_block_id(&mut self, id: BlockId) {
        write!(self.sb, "Block_{}", id.index()).expect("no format error")
    }

    fn display_call_id(&mut self, id: FunctionId) {
        let name = if id == FunctionId::error() {
            "<error>"
        } else {
            self.hir.nodes.functions[id].name.as_str()
        };

        self.push_str(name);
    }

    fn display_local(&mut self, id: LocalId) {
        write!(self.sb, "_{}", id.index()).expect("no format error")
    }

    fn display_temp(&mut self, id: LocalId) {
        write!(self.sb, "temp{}", id.index()).expect("no format error")
    }

    fn display_expression_astype(&mut self, value: ExpressionId, id: LazyTypeId) {
        let id = match self.typed {
            Some(typed) => typed.types_table.expressions[value].to_lazy(),
            None => id,
        };
        self.display_astype(id);
    }

    fn display_astype(&mut self, id: LazyTypeId) {
        self.push_str("<as: ");
        self.display_type(id);
        self.push('>');
    }

    fn display_struct_name(&mut self, id: StructId) {
        let name = self.hir.info.types
            .id_to_struct(id)
            .map(|s| s.name.as_str())
            .unwrap_or("<error>");
        
        self.push_str(name);
    }

    fn display_type(&mut self, id: LazyTypeId) {

        if let None = self.inner_type(id) {
            HirType::error_type().write_display(&self.hir.info.types, &self.hir.info.infers, &mut self.sb).expect("no format error")
        }
    }

    fn inner_type(&mut self, id: LazyTypeId) -> Option<()> {
        
        match (self.typed, id) {
            (Some(typed), LazyTypeId::Known(ty)) => {
                typed.types_map.id_to_type(ty)?.write_display(&typed.types_map, &mut self.sb).expect("no fmt error");
                return Some(());
            }
            (Some(_), LazyTypeId::Infer(_)) => panic!("should not have infer in thir"),
            _ => (),
        }
        
        let types = &self.hir.info.types;
        let infers = &self.hir.info.infers;
        match id {
            LazyTypeId::Known(type_id) => types.id_to_type(type_id)?.write_display(types, infers, &mut self.sb).expect("no fmt error"),
            LazyTypeId::Infer(infer_type_id) => infers.get_infer(infer_type_id)?.write_display(types, infers, &mut self.sb).expect("no fmt error"),
        }

        Some(())
    }

    fn to_string(self) -> String {
        self.sb
    }

    fn push_str(&mut self, str: &str) {
        self.sb.push_str(str);
    }

    fn push(&mut self, ch: char) {
        self.sb.push(ch);
    }
}
