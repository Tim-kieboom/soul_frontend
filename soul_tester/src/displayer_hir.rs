use hir::{
    Binary, BlockId, DisplayType, ExpressionId, FunctionBody, HirTree, HirType, LocalId, PossibleTypeId, Unary
};
use soul_utils::{
    ids::{FunctionId, IdAlloc},
    soul_names::KeyWord,
    vec_map::VecMapIndex,
};
use std::fmt::Write;

pub fn display_hir(hir: &HirTree) -> String {
    let mut displayer = HirDisplayer::new_hir(hir);

    for global in &hir.root.globals {
        displayer.display_global(global);
    }

    displayer.to_string()
}

struct HirDisplayer<'a> {
    sb: String,
    hir: &'a HirTree,

    depth: usize,
    terminate: Option<ExpressionId>,
}
impl<'a> HirDisplayer<'a> {
    fn new_hir(hir: &'a HirTree) -> Self {
        Self {
            hir,
            depth: 0,
            sb: String::new(),
            terminate: None,
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
            self.display_type(function.return_type);
            if i != last_index {
                self.push_str(", ");
            }
        }
        self.push_str("): ");

        self.display_type(function.return_type);
        if let FunctionBody::Internal(body) = &function.body {
            self.push(' ');
            self.display_block(body);
        }
    }

    fn display_variable(&mut self, variable: &hir::Variable) {
        if variable.is_temp {
            self.display_temp(variable.local);
        } else {
            self.display_local(variable.local);
        }

        self.push_str(": ");
        self.display_type(variable.ty);
        if let Some(value) = &variable.value {
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
            
                let types = &self.hir.info.types;
                self.push_str(types.id_to_struct(*ty).expect("should have struct").name.as_str());
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
                self.display_expression_astype(value.ty);
            }
            hir::ExpressionKind::Unary(Unary {
                operator,
                expression,
            }) => {
                self.push_str(operator.node.as_str());
                self.display_expression(expression);
                self.display_expression_astype(value.ty);
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
                self.display_expression_astype(value.ty);
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
                self.display_expression_astype(value.ty);
                self.push('\n');
            }
            hir::ExpressionKind::While { condition, body } => {
                self.push_str("while ");
                if let Some(value) = condition {
                    self.display_expression(value);
                    self.push(' ');
                }
                self.display_block(body);
                self.display_expression_astype(value.ty);
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
                        self.display_type(PossibleTypeId::Known(*generic));
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
                self.display_expression_astype(value.ty);
            }
            hir::ExpressionKind::Cast { value, cast_to } => {
                self.display_expression(value);
                self.push_str(" as ");
                self.display_type(*cast_to);
            }
            hir::ExpressionKind::InnerRawStackArray(ty) => {
                self.push_str("/*stack alloc ");
                self.display_expression_astype(*ty);
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

    fn display_expression_astype(&mut self, id: PossibleTypeId) {
        self.display_astype(id);
    }

    fn display_astype(&mut self, id: PossibleTypeId) {
        self.push_str("<as: ");
        self.display_type(id);
        self.push('>');
    }

    fn display_type(&mut self, id: PossibleTypeId) {

        if let None = self.inner_type(id) {
            HirType::error_type().write_display(&self.hir.info.types, &self.hir.info.infers, &mut self.sb).expect("no format error")
        }
    }

    fn inner_type(&mut self, id: PossibleTypeId) -> Option<()> {
        let types = &self.hir.info.types;
        let infers = &self.hir.info.infers;
        match id {
            PossibleTypeId::Known(type_id) => types.id_to_type(type_id)?.write_display(types, infers, &mut self.sb).expect("no fmt error"),
            PossibleTypeId::Infer(infer_type_id) => infers.get_infer(infer_type_id)?.write_display(types, infers, &mut self.sb).expect("no fmt error"),
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
