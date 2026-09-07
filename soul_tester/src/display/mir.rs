use std::{fmt::Debug, iter::Enumerate};

use crate::{
    config,
    display::{vecmap_to_pretty_vec, write_create_file, write_to_file, writer::Writer},
    push_fmt,
};
use anyhow::Result;
use mir_model::{
    AggregateKind, LocalDecl, LocalId, MirFunction, Operand, Place, PlaceElem, Rvalue, Statement, Terminator,
};
use mir_run::MirProgram;
use soul_utils::{TypeModifier, collections::vec_map::{VecMap, VecMapIndex}};

/// Writes a pretty-printed textual dump of every lowered `MirFunction` (mirrors
/// `display::ast::display_ast`'s `tree.soulc`, but for MIR — see
/// `docs/mir-design.md` for the shape this reconstructs), plus a JSON dump of
/// the raw `MirProgram` for programmatic inspection.
pub(crate) fn display_mir(program: &MirProgram) -> Result<()> {
    let mut output_path = config::CONFIG.output_path().join("mir");
    output_path.push("tree.soulc");

    let mut writer = write_create_file(&output_path)?;
    for (_, function) in program.functions.entries() {
        write_function(&mut writer, function)?;
        writer.push_char('\n')?;
    }
    writer.writer_flush()?;

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

fn local_str(id: impl VecMapIndex) -> String {
    format!("_{}", id.index())
}

fn block_str(id: impl VecMapIndex) -> String {
    format!("bb{}", id.index())
}

fn write_function(writer: &mut impl Writer, function: &MirFunction) -> Result<()> {
    let mut locals = function.locals.entries().enumerate();

    push_fmt!(writer, "{:?}(", function.name)?;
    write_parameters(writer, function, &mut locals)?;
    writer.push_char(')')?;
    write_return_local(writer, &mut locals)?;
    writer.push_str(" {\n")?;

    write_locals(writer, function, &mut locals)?;    
    write_block(writer, function)?;

    writer.push_str("}\n")?;
    Ok(())
}

fn write_block(writer: &mut impl Writer, function: &MirFunction) -> Result<()> {
    for (block_id, block) in function.blocks.entries() {
        push_fmt!(writer, "    {}: {{\n", block_str(block_id))?;
        for statement in &block.statements {
            writer.push_str("        ")?;
            write_statement(writer, statement)?;
            writer.push_str(";\n")?;
        }
        writer.push_str("        ")?;
        write_terminator(writer, &block.terminator)?;
        writer.push_str(";\n    }\n")?;
    }
    Ok(())
}

fn write_locals<'a, Iter>(writer: &mut impl Writer, function: &MirFunction, locals: &mut Enumerate<Iter>) -> Result<()> 
where 
    Iter: Iterator<Item = (LocalId, &'a LocalDecl)>
{
    writer.push_str("\tlocals: [\n\t\t")?;
    let last_index = function.locals.len().saturating_sub(1);
    for (i, (id, decl)) in locals {
        write_local(writer, id, decl)?;
        if i != last_index {
            writer.push_str(",\n\t\t")?;
        }
    }
    writer.push_str("\n\t]\n")?;
    Ok(())
}

fn write_parameters<'a, Iter>(writer: &mut impl Writer, function: &MirFunction, locals: &mut Enumerate<Iter>) -> Result<()> 
where 
    Iter: Iterator<Item = (LocalId, &'a LocalDecl)>
{
    for i in 0..function.arg_count {
        if i > 0 {
            writer.push_str(", ")?;
        }
        let Some((_i, (id, decl))) = locals.next() else {
            writer.push_str("<missing parameter local>")?;
            break;
        };
        write_local(writer, id, decl)?;
    }
    Ok(())
}

fn write_return_local<'a, Iter>(writer: &mut impl Writer, locals: &mut Enumerate<Iter>) -> Result<()>
where 
    Iter: Iterator<Item = (LocalId, &'a LocalDecl)>
{
    writer.push_str("-> ")?;

    match locals.next() {
        Some((_i, (id, decl))) => {
            write_local(writer, id, decl)?
        }
        None => writer.push_str("<missing return local>")?,
    };
    Ok(())
}

fn write_local(writer: &mut impl Writer, id: LocalId, decl: &LocalDecl) -> Result<()> {
    let name = local_str(id);
    let ty = &decl.ty;
    match decl.mutability {
        TypeModifier::Mut => writer.push_str("mut ")?,
        TypeModifier::Const => writer.push_str("const ")?,
        TypeModifier::Immut => (),
    }
    push_fmt!(writer, "{name}: {ty:?}")?;
    Ok(())
}

fn write_statement(writer: &mut impl Writer, statement: &Statement) -> Result<()> {
    match statement {
        Statement::Assign(place, rvalue) => {
            write_place(writer, place)?;
            writer.push_str(" = ")?;
            write_rvalue(writer, rvalue)?;
        }
        Statement::MarkMoved(local) => {
            push_fmt!(writer, "MarkMoved({})", local_str(*local))?;
        }
        Statement::SetDropFlag(local, value) => {
            push_fmt!(writer, "SetDropFlag({}, {value})", local_str(*local))?;
        }
        Statement::StorageDead(local) => {
            push_fmt!(writer, "StorageDead({})", local_str(*local))?;
        }
    }
    Ok(())
}

fn write_place(writer: &mut impl Writer, place: &Place) -> Result<()> {
    writer.push_str(&local_str(place.local))?;
    for elem in &place.projection {
        match elem {
            PlaceElem::Field(index) => push_fmt!(writer, ".{index}")?,
            PlaceElem::Index(index_local) => push_fmt!(writer, "[{}]", local_str(*index_local))?,
            PlaceElem::Deref => writer.push_str(".*")?,
        }
    }
    Ok(())
}

fn write_operand(writer: &mut impl Writer, operand: &Operand) -> Result<()> {
    match operand {
        Operand::Copy(place) => write_place(writer, place)?,
        Operand::Move(place) => {
            writer.push_str("move ")?;
            write_place(writer, place)?;
        }
        Operand::Constant(value) => push_fmt!(writer, "{value:?}")?,
    }
    Ok(())
}

fn write_rvalue(writer: &mut impl Writer, rvalue: &Rvalue) -> Result<()> {
    match rvalue {
        Rvalue::Use(operand) => write_operand(writer, operand)?,
        Rvalue::BinaryOp(op, left, right) => {
            write_operand(writer, left)?;
            push_fmt!(writer, " {} ", op.as_str())?;
            write_operand(writer, right)?;
        }
        Rvalue::UnaryOp(op, operand) => {
            writer.push_str(op.as_str())?;
            write_operand(writer, operand)?;
        }
        Rvalue::Ref { mutable, place } => {
            writer.push_str(if *mutable { "&mut " } else { "&" })?;
            write_place(writer, place)?;
        }
        Rvalue::Aggregate(kind, operands) => {
            push_fmt!(writer, "{}(", aggregate_kind_str(kind))?;
            let last_index = operands.len().saturating_sub(1);
            for (i, operand) in operands.iter().enumerate() {
                write_operand(writer, operand)?;
                if i != last_index {
                    writer.push_str(", ")?;
                }
            }
            writer.push_char(')')?;
        }
        Rvalue::Cast(operand, ty) => {
            write_operand(writer, operand)?;
            push_fmt!(writer, " as {ty:?}")?;
        }
    }
    Ok(())
}

fn aggregate_kind_str(kind: &AggregateKind) -> &'static str {
    match kind {
        AggregateKind::Struct => "Struct",
        AggregateKind::Tuple => "Tuple",
        AggregateKind::Array => "Array",
    }
}

fn write_terminator(writer: &mut impl Writer, terminator: &Terminator) -> Result<()> {
    match terminator {
        Terminator::Goto(target) => {
            push_fmt!(writer, "goto -> {}", block_str(*target))?;
        }
        Terminator::SwitchInt {
            discriminant,
            targets,
            otherwise,
        } => {
            writer.push_str("switchInt(")?;
            write_operand(writer, discriminant)?;
            writer.push_str(") -> [")?;
            for (value, target) in targets {
                push_fmt!(writer, "{value:?}: {}, ", block_str(*target))?;
            }
            push_fmt!(writer, "otherwise: {}]", block_str(*otherwise))?;
        }
        Terminator::Call {
            func,
            args,
            destination,
            target,
        } => {
            write_place(writer, destination)?;
            push_fmt!(writer, " = call {func:?}(")?;
            let last_index = args.len().saturating_sub(1);
            for (i, arg) in args.iter().enumerate() {
                write_operand(writer, arg)?;
                if i != last_index {
                    writer.push_str(", ")?;
                }
            }
            writer.push_char(')')?;
            match target {
                Some(target) => push_fmt!(writer, " -> {}", block_str(*target))?,
                None => writer.push_str(" -> !")?,
            }
        }
        Terminator::Drop { place, target } => {
            writer.push_str("drop(")?;
            write_place(writer, place)?;
            push_fmt!(writer, ") -> {}", block_str(*target))?;
        }
        Terminator::Return => {
            writer.push_str("return")?;
        }
        Terminator::Unreachable => {
            writer.push_str("unreachable")?;
        }
    }
    Ok(())
}
