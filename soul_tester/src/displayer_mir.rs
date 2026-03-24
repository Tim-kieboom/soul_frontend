use hir::{HirType, TypeId};
use hir_typed_context::HirTypedTable;
use mir_parser::mir::{
    self, BlockId, FunctionBody, Local, LocalId, MirTree, Operand, Place, PlaceId, Rvalue, StatementId, TempId
};
use soul_utils::{
    ids::{FunctionId, IdAlloc},
    soul_names::{TypeModifier, TypeWrapper},
    vec_map::VecMapIndex,
};
use std::fmt::Write;

pub fn display_mir(mir: &MirTree, types: &HirTypedTable) -> String {
    let mut displayer = MirDisplayer::new(mir, types);

    for global in mir.globals.values() {
        displayer.display_global(global);
    }
    if !mir.globals.is_empty() {
        displayer.push('\n');
    }

    if let Some(main) = mir.functions.get(mir.entry_function).map(|f| f.id) {
        displayer.display_function(main);
        displayer.push('\n');
    }

    if let Some(init_globals) = mir.functions.get(mir.init_global_function).map(|f| f.id) {
        displayer.display_function(init_globals);
        displayer.push('\n');
    }

    let keys = mir
        .functions
        .keys()
        .filter(|id| *id != mir.init_global_function && *id != mir.entry_function);
    for function in keys {
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
            mir,
            types,
            sb: String::new(),
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

    fn display_global(&mut self, global: &mir::Global) {
        self.display_local_declare(global.local);
        if let Some(literal) = &global.literal {
            self.push_str(" = ");
            self.push_str(&literal.value_to_string());
        }
        self.push('\n');
    }

    fn display_function(&mut self, function_id: FunctionId) {
        let function = &self.mir.functions[function_id];

        if let FunctionBody::External(language) = function.body {
            self.push_str("extern \"");
            self.push_str(language.as_str());
            self.push_str("\" ");
        }

        self.push_str(function.name.as_str());
        self.push('(');

        let last_index = function.parameters.len().saturating_sub(1);
        for (i, local) in function.parameters.iter().enumerate() {
            self.display_local_declare(*local);
            if i != last_index {
                self.push_str(", ");
            }
        }
        self.push_str("): ");
        self.get_type(function.return_type)
            .write_display(&self.types.types, &mut self.sb)
            .expect("no fmt error");
        self.push(' ');

        let (entry_block, locals, blocks) = match &function.body {
            FunctionBody::External(_) => return,
            FunctionBody::Internal {
                entry_block,
                locals,
                blocks,
            } => (*entry_block, locals, blocks),
        };

        self.push_str(" {\n\t");
        self.display_goto(entry_block);
        self.push('\n');

        for local_id in locals {
            self.push('\t');
            self.display_local_declare(*local_id);
            self.push('\n');
        }

        self.push('\n');
        for block in blocks {
            self.display_block(*block);
        }
        self.push_str("\n}\n");
    }

    fn display_local_declare(&mut self, local_id: LocalId) {
        let local = &self.mir.locals[local_id];

        let mut hir_type = self.get_type(local.ty());
        let modifier = hir_type.modifier.unwrap_or(TypeModifier::Const);
        hir_type.modifier = None;

        self.push_str(modifier.as_str());
        self.push(' ');
        self.display_local_name(local.id());
        self.push_str(": ");
        hir_type
            .write_display(&self.types.types, &mut self.sb)
            .expect("no fmt error");

        match local {
            Local::Comptime { value, .. } => {
                self.push_str(" = ");
                self.push_str(&value.value_to_string());
            }
            _ => (),
        }
    }

    fn display_block(&mut self, block_id: BlockId) {
        let block = &self.mir.blocks[block_id];

        self.display_block_name(block_id);
        self.push_str(": \n");
        for statement in &block.statements {
            self.push_str("\t");
            self.display_statement(*statement);
            self.push('\n');
        }

        self.push_str("\t");
        match &block.terminator {
            mir::Terminator::Exit => self.push_str("// exit"),
            mir::Terminator::Goto(block_id) => {
                self.display_goto(*block_id);
            }
            mir::Terminator::Return(operand) => {
                self.push_str("return ");
                if let Some(value) = operand {
                    self.display_operand(value);
                }
            }
            mir::Terminator::If {
                condition,
                then,
                arm,
            } => {
                self.push_str("if(");
                self.display_operand(condition);
                self.push_str(") ");
                self.display_goto(*then);
                self.push_str("\n\telse ");
                self.display_goto(*arm);
            }
            mir::Terminator::Unreachable => self.push_str("// unreachable"),
        }
    }

    fn display_statement(&mut self, statement_id: StatementId) {
        let statement = &self.mir.statements[statement_id];

        match &statement.kind {
            mir::StatementKind::Call {
                id,
                type_args,
                arguments,
                return_place,
            } => {
                if let Some(place) = return_place {
                    self.display_place(place);
                    self.push_str(" = ");
                }

                self.display_function_name(*id);
                if !type_args.is_empty() {
                    self.push('<');
                    let last_index = type_args.len().saturating_sub(1);
                    for (i, generic) in type_args.iter().enumerate() {
                        self.display_type(*generic);
                        if i != last_index {
                            self.push_str(", ");
                        }
                    }
                    self.push('>');
                }
                self.push('(');
                let last_index = arguments.len().saturating_sub(1);
                for (i, arg) in arguments.iter().enumerate() {
                    self.display_operand(arg);
                    if i != last_index {
                        self.push_str(", ");
                    }
                }
                self.push_str(") ");
            }
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
            mir::RvalueKind::StackAlloc(ty) => {
                self.push_str("/*stack alloc ");
                self.display_type(*ty);
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
            mir::RvalueKind::CastUse { value, cast_to } => {
                self.display_operand(value);
                self.push_str(" as ");
                self.display_type(*cast_to);
            }
        }
    }

    fn display_operand(&mut self, operand: &Operand) {
        const MUT: bool = true;
        const CONST: bool = false;

        match &operand.kind {
            mir::OperandKind::Ref { place, mutable } => {
                match *mutable {
                    MUT => self.push_str(TypeWrapper::MutRef.as_str()),
                    CONST => self.push_str(TypeWrapper::ConstRef.as_str()),
                };
                self.display_place(place);
            }
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
            Place::Temp(temp_id) => {
                self.display_temp_name(*temp_id);
            }
            Place::Deref(operand) => {
                self.push('*');
                self.display_operand(operand);
            }
            Place::Local(local_id) => self.display_local_name(*local_id),
        }
    }

    fn display_goto(&mut self, block_id: BlockId) {
        self.push_str("goto -> ");
        self.display_block_name(block_id);
    }

    fn display_function_name(&mut self, function: FunctionId) {
        let name = if function == FunctionId::error() {
            "<error>"
        } else {
            self.mir.functions[function].name.as_str()
        };

        self.push_str(name);
    }

    fn display_local_name(&mut self, local: LocalId) {
        if local == LocalId::error() {
            self.push_str("_error");
        } else {
            write!(self.sb, "_{}", local.index()).expect("not fmt error");
        }
    }

    fn display_temp_name(&mut self, temp: TempId) {
        write!(self.sb, "temp{}", temp.index()).expect("no fmt error");
    }

    fn display_block_name(&mut self, block_id: BlockId) {
        write!(self.sb, "bb_{}", block_id.index()).expect("no fmt error");
    }

    fn display_type(&mut self, ty: TypeId) {
        self.get_type(ty)
            .write_display(&self.types.types, &mut self.sb)
            .expect("no fmt error");
    }

    fn get_type(&self, ty: TypeId) -> HirType {
        self.types
            .types
            .id_to_type(ty)
            .cloned()
            .unwrap_or(HirType::error_type())
    }
}
