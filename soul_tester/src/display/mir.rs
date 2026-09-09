use std::{fmt::Debug, iter::Enumerate};

use crate::{
    config,
    display::{vecmap_to_pretty_vec, write_create_file, write_to_file, writer::Writer},
    push_fmt,
};
use anyhow::Result;
use ast_model::AstStore;
use mir_model::{
    AggregateKind, Function, LocalDecl, LocalId, Operand, Place, PlaceElem, Rvalue, Statement,
    Terminator,
};
use mir_run::MirProgram;
use soul_utils::{
    FunctionId, SharedStr, TypeModifier, collections::vec_map::{VecMap, VecMapIndex},
};

pub(crate) fn display_mir(program: &MirProgram, ast: &AstStore) -> Result<()> {
    
    let mut output_path = config::CONFIG.output_path().join("mir");
    output_path.push("tree.soulc");

    let mut writer = write_create_file(&output_path)?;
    let mut dispayer = Displayer::new(&mut writer, ast);
    for (_, function) in program.functions.entries() {
        dispayer.write_function(function)?;
        dispayer.push_char('\n')?;
    }
    dispayer.writer_flush()?;

    output_path.pop();
    output_path.push("json");
    let functions = vecmap_to_json_str(&program.functions)?;
    write_to_file(&output_path.join("functions.json"), &functions)?;

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

struct Displayer<'a, W: Writer> {
    writer: &'a mut W,
    ast: &'a AstStore
}
impl<'a, W: Writer> Displayer<'a, W> {
    pub fn new(writer: &'a mut W, ast: &'a AstStore) -> Self {
        Self {
            ast,
            writer,
        }
    }

    fn write_function(&mut self, function: &Function) -> Result<()> {
        let mut locals = function.locals.entries().enumerate();

        let name = self.get_function_name(function.id);

        push_fmt!(self, "{name}(")?;
        self.write_parameters(function, &mut locals)?;
        self.push_char(')')?;
        self.write_return_local(function.return_local, &mut locals)?;
        self.push_str(" {\n")?;

        self.write_locals(function, &mut locals)?;
        self.write_block(function)?;

        self.push_str("}\n")?;
        Ok(())
    }

    fn write_block(&mut self, function: &Function) -> Result<()> {
        for (block_id, block) in function.blocks.entries() {
            
            push_fmt!(self, "    {}: {{\n", block_str(block_id))?;
            for statement in &block.statements {
                self.push_str("        ")?;
                self.write_statement(statement)?;
                self.push_str(";\n")?;
            }
            self.push_str("        ")?;
            self.write_terminator(&block.terminator)?;
            self.push_str(";\n    }\n")?;
        }
        Ok(())
    }

    fn write_locals<'f, Iter>(
        &mut self,
        function: &Function,
        locals: &mut Enumerate<Iter>,
    ) -> Result<()>
    where
        Iter: Iterator<Item = (LocalId, &'f LocalDecl)>,
    {
        self.push_str("\tlocals.[")?;
        let Some((_i, local)) = locals.next() else {
            self.push_str("]\n")?;
            return Ok(());
        };
        self.push_str("\n\t\t")?;
        self.write_local(local)?;
        self.push_str("\n\t\t")?;

        let last_index = function.locals.len().saturating_sub(1);
        for (i, local) in locals {
            self.write_local(local)?;
            if i != last_index {
                self.push_str(",\n\t\t")?;
            }
        }
        self.push_str("\n\t]\n")?;
        Ok(())
    }

    fn write_parameters<'f, Iter>(
        &mut self,
        function: &Function,
        locals: &mut Enumerate<Iter>,
    ) -> Result<()>
    where
        Iter: Iterator<Item = (LocalId, &'f LocalDecl)>,
    {
        for i in 0..function.arg_count {
            if i > 0 {
                self.push_str(", ")?;
            }
            let Some((_i, local)) = locals.next() else {
                self.push_str("<missing parameter local>")?;
                break;
            };
            self.write_local(local)?;
        }
        Ok(())
    }

    fn write_return_local<'f, Iter>(
        &mut self,
        return_local: Option<LocalId>,
        locals: &mut Enumerate<Iter>,
    ) -> Result<()>
    where
        Iter: Iterator<Item = (LocalId, &'f LocalDecl)>,
    {
        self.push_str(" -> ")?;

        // A `none`-returning function has no return local at all (see
        // `mir_model::Function::return_local`) — don't consume from `locals` in
        // that case, there's nothing there for it.
        match return_local {
            None => self.push_str("none")?,
            Some(_) => match locals.next() {
                Some((_i, local)) => self.write_local(local)?,
                None => self.push_str("<missing return local>")?,
            },
        };
        Ok(())
    }

    fn write_local(&mut self, local: (LocalId, &LocalDecl)) -> Result<()> {
        let (id, decl) = local;
        let name = local_str(id);
        let ty = &decl.ty;
        match decl.mutability {
            TypeModifier::Mut => self.push_str("mut ")?,
            TypeModifier::Comptime => self.push_str("const ")?,
            TypeModifier::Immut => (),
        }
        push_fmt!(self, "{name}: {ty:?}")?;
        Ok(())
    }

    fn write_statement(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::Assign(place, rvalue) => {
                self.write_place(place)?;
                self.push_str(" = ")?;
                self.write_rvalue(rvalue)?;
            }
            Statement::MarkMoved(local) => {
                push_fmt!(self, "MarkMoved({})", local_str(*local))?;
            }
            Statement::SetDropFlag(local, value) => {
                push_fmt!(self, "SetDropFlag({}, {value})", local_str(*local))?;
            }
            Statement::StorageDead(local) => {
                push_fmt!(self, "StorageDead({})", local_str(*local))?;
            }
        }
        Ok(())
    }

    fn write_place(&mut self, place: &Place) -> Result<()> {
        self.push_str(&local_str(place.local))?;
        for elem in &place.projection {
            match elem {
                PlaceElem::Field(index) => push_fmt!(self, ".{index}")?,
                PlaceElem::Index(index_local) => push_fmt!(self, "[{}]", local_str(*index_local))?,
                PlaceElem::Deref => self.push_str(".*")?,
            }
        }
        Ok(())
    }

    fn write_operand(&mut self, operand: &Operand) -> Result<()> {
        match operand {
            Operand::Copy(place) => self.write_place(place)?,
            Operand::Move(place) => {
                self.push_str("move ")?;
                self.write_place(place)?;
            }
            Operand::Constant(value) => push_fmt!(self, "{value:?}")?,
        }
        Ok(())
    }

    fn write_rvalue(&mut self, rvalue: &Rvalue) -> Result<()> {
        match rvalue {
            Rvalue::Use(operand) => self.write_operand(operand)?,
            Rvalue::BinaryOp(op, left, right) => {
                self.write_operand(left)?;
                push_fmt!(self, " {} ", op.as_str())?;
                self.write_operand(right)?;
            }
            Rvalue::UnaryOp(op, operand) => {
                self.push_str(op.as_str())?;
                self.write_operand(operand)?;
            }
            Rvalue::Ref { mutable, place } => {
                self.push_str(if *mutable { "&mut " } else { "&" })?;
                self.write_place(place)?;
            }
            Rvalue::Aggregate(kind, operands) => {
                push_fmt!(self, "{}(", aggregate_kind_str(kind))?;
                let last_index = operands.len().saturating_sub(1);
                for (i, operand) in operands.iter().enumerate() {
                    self.write_operand(operand)?;
                    if i != last_index {
                        self.push_str(", ")?;
                    }
                }
                self.push_char(')')?;
            }
            Rvalue::Cast(operand, ty) => {
                self.write_operand(operand)?;
                push_fmt!(self, " as {ty:?}")?;
            }
        }
        Ok(())
    }

    fn write_terminator(&mut self, terminator: &Terminator) -> Result<()> {
        match terminator {
            Terminator::Goto(target) => {
                push_fmt!(self, "goto -> {}", block_str(*target))?;
            }
            Terminator::SwitchInt {
                discriminant,
                targets,
                otherwise,
            } => {
                self.push_str("switchInt(")?;
                self.write_operand(discriminant)?;
                self.push_str(") -> [")?;
                for (value, target) in targets {
                    push_fmt!(self, "{value:?}: {}, ", block_str(*target))?;
                }
                push_fmt!(self, "otherwise: {}]", block_str(*otherwise))?;
            }
            Terminator::Call {
                id,
                arguments,
                destination,
                target,
            } => {
                if let Some(place) = destination {
                    self.write_place(place)?;
                    self.push_str(" = ")?;
                }
                push_fmt!(self, "/*call*/ {}(", self.get_function_name(*id))?;
                let last_index = arguments.len().saturating_sub(1);
                for (i, arg) in arguments.iter().enumerate() {
                    self.write_operand(arg)?;
                    if i != last_index {
                        self.push_str(", ")?;
                    }
                }
                self.push_char(')')?;
                match target {
                    Some(target) => push_fmt!(self, " -> {}", block_str(*target))?,
                    None => self.push_str(" -> !")?,
                }
            }
            Terminator::Drop { place, target } => {
                self.push_str("drop(")?;
                self.write_place(place)?;
                push_fmt!(self, ") -> {}", block_str(*target))?;
            }
            Terminator::Return => {
                self.push_str("return")?;
            }
            Terminator::Unreachable => {
                self.push_str("unreachable")?;
            }
        }
        Ok(())
    }

    fn get_function_name(&self, id: FunctionId) -> SharedStr {
        self.ast
            .functions
            .get(id)
            .map(|kind| kind.signature().name.as_shared_str())
            .unwrap_or(format!("{id:?}").into())
    }
}
impl<'a, W: Writer> Writer for Displayer<'a, W> {
    type Error = W::Error;

    fn push_fmt(&mut self, args: std::fmt::Arguments<'_>) -> Result<(), Self::Error> {
        self.writer.push_fmt(args)
    }

    fn writer_flush(&mut self) -> std::prelude::v1::Result<(), Self::Error> {
        self.writer.writer_flush()
    }
}

fn aggregate_kind_str(kind: &AggregateKind) -> &'static str {
    match kind {
        AggregateKind::Struct => "Struct",
        AggregateKind::Tuple => "Tuple",
        AggregateKind::Array => "Array",
    }
}

fn local_str(id: impl VecMapIndex) -> String {
    format!("_{}", id.index())
}

fn block_str(id: impl VecMapIndex) -> String {
    format!("bb{}", id.index())
}
