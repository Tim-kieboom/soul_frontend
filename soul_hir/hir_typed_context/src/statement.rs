use crate::HirTypedContext;
use hir::{
    Assign, BlockId, ExpressionId, FunctionId, Global, HirType, IdAlloc, Place, PlaceKind, Statement, TypeId, UnifyResult, Variable
};
use soul_utils::error::{SoulError, SoulErrorKind};

impl<'a> HirTypedContext<'a> {
    pub(crate) fn infer_global(&mut self, global: &Global) {
        let ty = match global {
            Global::InternalAssign(assign, _) => self.infer_assign(assign),
            Global::Variable(variable, _) => {
                let ty = self.infer_variable(variable);
                self.type_local(variable.local, ty);
                ty
            }
            Global::Function(function, _) => {
                let ty = self.infer_function(function);
                self.type_function(*function, ty);
                ty
            }
        };

        self.type_statement(global.get_id(), ty);
    }

    pub(crate) fn infer_statement(&mut self, statement: &Statement) {
        let ty = match statement {
            Statement::Assign(assign, _) => self.infer_assign(assign),
            Statement::Continue(_) => self.add_type(HirType::none_type()),
            Statement::Variable(variable, _) => self.infer_variable(variable),
            Statement::Expression { value, .. } => self.infer_expression(*value),

            Statement::Fall(value, _)
            | Statement::Break(value, _)
            | Statement::Return(value, _) => match value {
                Some(val) => self.infer_expression(*val),
                None => self.none_type,
            },
        };

        self.type_statement(statement.get_id(), ty);
    }

    pub(crate) fn infer_block(&mut self, body: BlockId) -> TypeId {
        let mut return_type = None;

        let block = &self.hir.blocks[body];
        for statement in &block.statements {
            self.infer_statement(statement);
            if let Statement::Return(value, id) = statement {
                let span = self.statement_span(*id);
                let got = match value {
                    Some(val) => self.infer_expression(*val),
                    None => self.none_type,
                };

                let value = value.unwrap_or(ExpressionId::error());
                match return_type {
                    Some(ty) => self.unify(value, ty, got, span),
                    None => return_type = Some(got),
                }
            }
        }

        if let Some(terminator) = block.terminator {
            let span = self.expression_span(terminator);
            let got = self.infer_expression(terminator);
            match return_type {
                Some(ty) => self.unify(terminator, ty, got, span),
                None => return_type = Some(got),
            }
        }

        let ty = return_type.unwrap_or(self.none_type);
        self.type_block(body, ty);
        ty
    }

    pub(crate) fn infer_place(&mut self, place: &Place) -> TypeId {
        let span = place.span;
        match &place.node {
            PlaceKind::Local(id) => {
                let ty = self.hir.locals[*id];
                self.type_local(*id, ty);
                ty
            }
            PlaceKind::Deref(place) => {
                let inner = self.infer_place(place);
                let deref = self.get_type(inner).try_deref(&self.hir.types, span);
                match deref {
                    Ok(val) => val,
                    Err(err) => {
                        self.log_error(err);
                        TypeId::error()
                    }
                }
            }
            PlaceKind::Index { base, .. } => {
                let base = self.infer_place(base);
                let base_type = self.get_type(base);
                match &base_type.kind {
                    hir::HirTypeKind::Array { element, .. } => *element,
                    _ => {
                        self.log_error(SoulError::new(
                            "can only index array type",
                            SoulErrorKind::UnifyTypeError,
                            Some(span),
                        ));
                        TypeId::error()
                    }
                }
            }
            PlaceKind::Field { .. } => todo!("field not yet impl"),
        }
    }

    fn infer_variable(&mut self, variable: &Variable) -> TypeId {
        let declared_type = variable.ty;
        if declared_type == TypeId::error() {
            self.type_local(variable.local, declared_type);
            return declared_type;
        }

        let (value, span) = match variable.value {
            Some(val) => (val, self.expression_span(val)),
            None => return declared_type,
        };

        let value_type = self.infer_expression(value);
        self.unify(value, declared_type, value_type, span);
        self.type_local(variable.local, declared_type);
        declared_type
    }

    fn infer_function(&mut self, function_id: &FunctionId) -> TypeId {
        let function = &self.hir.functions[*function_id];
        let span = self.block_span(function.body);
        let block_type = self.infer_block(function.body);
        for parameter in &function.parameters {
            self.type_local(parameter.local, parameter.ty);
        }
        self.unify(
            ExpressionId::error(),
            function.return_type,
            block_type,
            span,
        );
        function.return_type
    }

    fn infer_assign(&mut self, assign: &Assign) -> TypeId {
        let span = self.expression_span(assign.value);
        let expected = self.infer_place(&assign.place);
        let value_type = self.infer_expression(assign.value);
        match self.infer_table.unify_type_type(expected, value_type, span) {
            Ok(UnifyResult::Ok) => (),
            Ok(UnifyResult::NeedsAutoCopy) => self.add_autocopy(assign.value),
            Err(err) => self.log_error(err),
        }
        expected
    }
}
