use hir::{FieldId, FunctionId, HirType};
use hir_typed_context::HirTypedTable;
use mir_parser::mir::{
    self, BlockId, LocalId, MirTree, Operand, Place, PlaceId, Rvalue, StatementId, TempId
};
use soul_utils::{soul_names::TypeModifier, vec_map::VecMapIndex};
use std::fmt::Write;

pub fn display_mir(mir: &MirTree, types: &HirTypedTable) -> String {
    let mut displayer = MirDisplayer::new(mir, types);

    for function in mir.functions.keys() {
        displayer.display_function(function);
        displayer.push('\n');
    }

    displayer.to_string()
}

struct MirDisplayer<'a> {
    sb: String,
    mir: &'a MirTree,
    types: &'a HirTypedTable,
}
impl<'a> MirDisplayer<'a> {
    fn new(mir: &'a MirTree, types: &'a HirTypedTable) -> Self {
        Self {
            sb: String::new(),
            mir,
            types,
        }
    }

    fn push(&mut self, ch: char) {
        self.sb.push(ch);
    }

    fn push_str(&mut self, str: &str) {
        self.sb.push_str(str);
    }

    fn to_string(self) -> String {
        self.sb
    }

    fn display_function(&mut self, function_id: FunctionId) {
        let function = &self.mir.functions[function_id];
        self.push_str(function.name.as_str());
        self.push('(');

        let last_index = function.parameters.len().saturating_sub(1);
        for (i, local) in function.parameters.iter().enumerate() {
            self.display_local_name(*local);
            if i != last_index {
                self.push_str(", ");
            }
        }
        self.push_str(") [\n");
        for local_id in &function.locals {
            let local = &self.mir.locals[*local_id];

            let mut hir_type = self
                .types
                .types
                .get_type(local.ty)
                .unwrap_or(&HirType::error_type())
                .clone();
            let modifier = hir_type.modifier.unwrap_or(TypeModifier::Const);
            hir_type.modifier = None;

            self.push('\t');
            self.push_str(modifier.as_str());
            self.push(' ');
            self.display_local_name(local.id);
            self.push_str(": ");
            hir_type
                .write_display(&self.types.types, &mut self.sb)
                .expect("no fmt error");
            self.push('\n');
        }

        self.push('\n');
        for block in &function.blocks {
            self.display_block(*block);
        }
        self.push_str("\n]\n");
    }

    fn display_block(&mut self, block_id: BlockId) {
        let block = &self.mir.blocks[block_id];

        self.push('\t');
        self.display_block_name(block_id);
        self.push_str(": {\n");
        for statement in &block.statements {
            self.push_str("\t\t");
            self.display_statement(*statement);
            self.push('\n');
        }

        self.push_str("\t\t");
        match &block.terminator {
            mir::Terminator::Goto(block_id) => {
                self.push_str("goto ");
                self.display_block_name(*block_id);
            }
            mir::Terminator::Return(operand) => {
                self.push_str("return ");
                if let Some(value) = operand {
                    self.display_operand(value);
                }
            }
            mir::Terminator::If { .. } => todo!(),
            mir::Terminator::Call { .. } => todo!(),
            mir::Terminator::Unreachable => self.push_str("// unreachable"),
        }
        self.push_str("\n\t}");
    }

    fn display_statement(&mut self, statement_id: StatementId) {
        let statement = &self.mir.statements[statement_id];

        match &statement.kind {
            mir::StatementKind::StorageStart(locals) => {
                self.push_str("StorageLives([");
                let last_index = locals.len().saturating_sub(1);
                for (i, local) in locals.iter().enumerate() {
                    self.display_local_name(*local);
                    if i != last_index {
                        self.push_str(", ");
                    }
                }
                self.push_str("])");
            }
            mir::StatementKind::Eval(operand) => self.display_operand(operand),
            mir::StatementKind::Assign { place, value } => {
                self.display_place(place);
                self.push_str(" = ");
                self.display_rvalue(value);
            }
            mir::StatementKind::StorageDead(local_id) => {
                self.push_str("StorageDead(");
                self.display_local_name(*local_id);
                self.push(')');
            }
        }
    }

    fn display_rvalue(&mut self, value: &Rvalue) {
        match &value.kind {
            mir::RvalueKind::StackAlloc { ty: _, len } => {
                self.push_str("/*stack alloc ");
                self.display_operand(len);
                self.push_str("*/");
            }
            mir::RvalueKind::Use(operand) => self.display_operand(operand),
            mir::RvalueKind::Binary {
                left,
                operator,
                right,
            } => {
                self.display_operand(left);
                self.push(' ');
                self.push_str(operator.node.as_str());
                self.push(' ');
                self.display_operand(right);
            }
            mir::RvalueKind::Unary { operator, value } => {
                self.push_str(operator.node.as_str());
                self.display_operand(value);
            }
        }
    }

    fn display_operand(&mut self, operand: &Operand) {
        match &operand.kind {
            mir::OperandKind::Temp(temp_id) => self.display_temp_name(*temp_id),
            mir::OperandKind::Local(local_id) => self.display_local_name(*local_id),
            mir::OperandKind::Comptime(literal) => {
                write!(self.sb, "{:?}", literal).expect("no fmt error");
            }
            mir::OperandKind::None => self.push_str("<none>"),
        }
    }

    fn display_place(&mut self, place_id: &PlaceId) {
        let place = &self.mir.places[*place_id];
        match place {
            Place::Temp(temp_id) => self.display_temp_name(*temp_id),
            Place::Deref(operand) => {
                self.push('*');
                self.display_operand(operand);
            }
            Place::Local(local_id) => self.display_local_name(*local_id),
            Place::Index(place_id, operand) => {
                self.display_place(place_id);
                self.push('[');
                self.display_operand(operand);
                self.push(']');
            }
            Place::Field(place_id, field_id) => {
                self.display_place(place_id);
                self.push('.');
                self.display_field_name(*field_id);
            }
        }
    }

    fn display_field_name(&mut self, field: FieldId) {
        write!(self.sb, "field{}", field.index()).expect("not fmt error");
    }

    fn display_local_name(&mut self, local: LocalId) {
        write!(self.sb, "_{}", local.index()).expect("not fmt error");
    }

    fn display_temp_name(&mut self, temp: TempId) {
        write!(self.sb, "temp{}", temp.index()).expect("not fmt error");
    }

    fn display_block_name(&mut self, block_id: BlockId) {
        write!(self.sb, "bb_{}", block_id.index()).expect("not fmt error");
    }
}
