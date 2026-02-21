use hir::{BlockId, ExpressionId, FunctionId, HirTree, LocalId, TypeId};
use hir_typed_context::HirTypedTable;
use soul_utils::{soul_names::KeyWord, vec_map::VecMapIndex};
use std::fmt::Write;

pub fn display_hir(hir: &HirTree) -> String {
    let mut displayer = HirDisplayer::new_hir(hir);

    for global in &hir.root.globals {
        displayer.display_global(global);
    }

    displayer.to_string()
}

pub fn display_typed_hir(hir: &HirTree, type_table: &HirTypedTable) -> String {
    let mut displayer = HirDisplayer::new_typed_hir(hir, type_table);

    for global in &hir.root.globals {
        displayer.display_global(global);
    }

    displayer.to_string()
}

struct HirDisplayer<'a> {
    sb: String,
    hir: &'a HirTree,
    type_table: Option<&'a HirTypedTable>,

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
            type_table: None,
        }
    }

    fn new_typed_hir(hir: &'a HirTree, type_table: &'a HirTypedTable) -> Self {
        Self {
            hir,
            depth: 0,
            sb: String::new(),
            terminate: None,
            type_table: Some(type_table),
        }
    }

    fn display_global(&mut self, global: &hir::Global) {
        match global {
            hir::Global::Function(function, _id) => self.display_function(function),
            hir::Global::Variable(variable, _id) => self.display_variable(variable),
            hir::Global::InternalAssign(assign, _id) => self.display_assign(assign),
        }
        self.push('\n');
    }

    fn display_function(&mut self, function_id: &hir::FunctionId) {
        let function = &self.hir.functions[*function_id];
        self.push('\n');
        self.display_call_id(function.id);
        self.push(' ');
        self.push_str(function.name.as_str());
        self.push('(');

        let last_index = function.parameters.len().saturating_sub(1);
        for (i, arg) in function.parameters.iter().enumerate() {
            self.display_local(arg.local);
            self.push_str(": ");
            let ty = match &self.type_table {
                Some(val) => val.locals[arg.local],
                None => function.return_type,
            };
            self.display_type(ty);
            if i != last_index {
                self.push_str(", ");
            }
        }
        self.push_str("): ");

        let ty = match &self.type_table {
            Some(val) => val.functions[function.id],
            None => function.return_type,
        };
        self.display_type(ty);
        self.push(' ');
        self.display_block(&function.body);
    }

    fn display_variable(&mut self, variable: &hir::Variable) {
        self.display_local(variable.local);
        self.push_str(": ");
        let ty = match self.type_table {
            Some(val) => val.locals[variable.local],
            None => variable.ty,
        };
        self.display_type(ty);
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
        let block = &self.hir.blocks[*id];

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
        match node {
            hir::Statement::Assign(assign, _) => self.display_assign(assign),
            hir::Statement::Variable(variable, _) => self.display_variable(variable),
            hir::Statement::Fall(expression_id, _) => {
                self.push_str("fall ");
                if let Some(value) = expression_id {
                    self.display_expression(value);
                }
            }
            hir::Statement::Break(expression_id, _) => {
                self.push_str("break ");
                if let Some(value) = expression_id {
                    self.display_expression(value);
                }
            }
            hir::Statement::Return(expression_id, _) => {
                self.push_str("return ");
                if let Some(value) = expression_id {
                    self.display_expression(value);
                }
            }
            hir::Statement::Continue(_) => self.push_str(KeyWord::Continue.as_str()),
            hir::Statement::Expression {
                id: _,
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
        let value = &self.hir.expressions[*id];

        match &value.kind {
            hir::ExpressionKind::Block(block_id) => self.display_block(block_id),
            hir::ExpressionKind::Null => self.push_str("null"),
            hir::ExpressionKind::Literal(literal) => self.push_str(&literal.value_to_string()),
            hir::ExpressionKind::Local(local_id) => self.display_local(*local_id),
            hir::ExpressionKind::Function(_) => self.push_str("<function>"),
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
                self.display_expression_astype(value.id, value.ty);
            }
            hir::ExpressionKind::Unary {
                operator,
                expression,
            } => {
                self.push_str(operator.node.as_str());
                self.display_expression(expression);
                self.display_expression_astype(value.id, value.ty);
            }
            hir::ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                self.push('(');
                self.display_expression(left);
                self.push_str(operator.node.as_str());
                self.display_expression(right);
                self.push(')');
                self.display_expression_astype(value.id, value.ty);
            }
            hir::ExpressionKind::If {
                condition,
                then_block,
                else_block,
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
                self.display_expression_astype(value.id, value.ty);
                self.push('\n');
            }
            hir::ExpressionKind::While { condition, body } => {
                self.push_str("while ");
                if let Some(value) = condition {
                    self.display_expression(value);
                }
                self.display_block(body);
                self.display_expression_astype(value.id, value.ty);
                self.push('\n');
            }
            hir::ExpressionKind::Call {
                function,
                callee,
                arguments,
            } => {
                if let Some(value) = callee {
                    self.display_expression(value);
                    self.push('.');
                }

                self.display_call_id(*function);
                self.push('(');
                let last_index = arguments.len().saturating_sub(1);
                for (i, arg) in arguments.iter().enumerate() {
                    self.display_expression(arg);
                    if i != last_index {
                        self.push_str(", ");
                    }
                }
                self.push(')');
                self.display_expression_astype(value.id, value.ty);
            }
            hir::ExpressionKind::Cast { value, cast_to } => {
                self.display_expression(value);
                self.push_str(" as ");
                self.display_type(*cast_to);
            }
            hir::ExpressionKind::InnerRawStackArray { ty, .. } => {
                self.push_str("/*RawAlloced ");
                self.display_expression_astype(value.id, *ty);
                self.push_str("*/");
            }
        };
    }

    fn display_place(&mut self, place: &hir::Place) {
        match &place.node {
            hir::PlaceKind::Local(local_id) => self.display_local(*local_id),
            hir::PlaceKind::Deref(place) => {
                self.push('*');
                self.display_place(place);
            }
            hir::PlaceKind::Index { base, index } => {
                self.display_place(base);
                self.push('[');
                self.display_expression(index);
                self.push(']');
            }
            hir::PlaceKind::Field { base, index } => {
                self.display_place(base);
                self.push('.');
                write!(self.sb, "{:?}", index).expect("no fromat error");
            }
        }
    }

    fn display_block_id(&mut self, id: BlockId) {
        write!(self.sb, "Block_{}", id.index()).expect("no format error")
    }

    fn display_call_id(&mut self, id: FunctionId) {
        write!(self.sb, "Func_{}", id.index()).expect("no format error")
    }

    fn display_local(&mut self, id: LocalId) {
        write!(self.sb, "var_{}", id.index()).expect("no format error")
    }

    fn display_expression_astype(&mut self, value: ExpressionId, id: TypeId) {
        let ty = match &self.type_table {
            Some(val) => val.expressions[value],
            None => id,
        };
        self.display_astype(ty);
    }

    fn display_astype(&mut self, id: TypeId) {
        self.push_str("<as: ");
        self.display_type(id);
        self.push('>');
    }

    fn display_type(&mut self, id: TypeId) {
        let types = match &self.type_table {
            Some(val) => &val.types,
            None => &self.hir.types,
        };

        let ty = types.get_type(id).expect("should have id");
        ty.write_display(types, &mut self.sb)
            .expect("no format error");
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
