use soul_utils::soul_error_internal;

use crate::{EndBlock, MirContext, mir};

pub(crate) struct StatementResponse {
    /// insert terminator in block
    pub(crate) terminator: Option<mir::Terminator>,
    
    /// operand is Some() when statement is expressionStatement
    pub(crate) expression_operand: Option<mir::Operand>,
}

impl<'a> MirContext<'a> {
    pub(crate) fn lower_statement(&mut self, statement: &hir::Statement) -> EndBlock<StatementResponse> {
        let statement_id = statement.get_id();
        let is_end = &mut false; 

        let mut last_operand = None;

        let terminator = match statement {
            hir::Statement::Variable(variable, _) => {
                self.lower_variable(variable, is_end);
                None
            }
            hir::Statement::Assign(assign, _) => {
                self.lower_assign(assign, is_end);
                None
            }
            hir::Statement::Expression { value, .. } => {

                let kind = &self.hir.expressions[*value].kind;
                let operand = self.lower_operand(*value).pass(is_end);
                
                if is_valid_statement_expression(kind) && !matches!(operand.kind, mir::OperandKind::None) {
                    let statement = mir::Statement::new(mir::StatementKind::Eval(operand.clone()));
                    self.push_statement(statement);
                }

                last_operand = Some(operand);
                None
            }
            hir::Statement::Return(value, _) => {
                let operand = match value {
                    Some(val) => {
                        let operand = self.lower_operand(*val).pass(is_end);
                        Some(operand)
                    }
                    None => None,
                };

                *is_end = true;
                Some(mir::Terminator::Return(operand))
            }
            hir::Statement::Continue(_)
            | hir::Statement::Fall(_, _)
            | hir::Statement::Break(_, _) => {
                let span = self.hir.spans.statements[statement_id];
                self.log_error(soul_error_internal!("statement not yet impl", Some(span)));
                None
            }
        };

        let response = StatementResponse {
            terminator,
            expression_operand: last_operand,
        };
        EndBlock::new(response, is_end)
    }

    pub(crate) fn lower_variable(&mut self, variable: &hir::Variable, is_end: &mut bool) {
        
        let local = if variable.is_temp {
            mir::Place::Temp(match self.temp_remap.get(variable.local) {
                Some(val) => *val,
                None => self.new_temp(self.types.locals[variable.local]),
            })
        } else {
            mir::Place::Local(match self.local_remap.get(variable.local) {
                Some(val) => *val,
                None => self.new_local(variable.local, self.types.locals[variable.local]),
            })
        };
        
        if let Some(value) = variable.value {
            let operand = self.lower_operand(value).pass(is_end);

            let place = self.new_place(local);
            let statement = mir::Statement::new(mir::StatementKind::Assign {
                place,
                value: mir::Rvalue::new(mir::RvalueKind::Use(operand)),
            });

            self.push_statement(statement);
        }
    }

    pub(crate) fn lower_assign(&mut self, assign: &hir::Assign, is_end: &mut bool) {
        let place = self.lower_place(&assign.place).pass(is_end);
        let value = self.lower_operand(assign.value).pass(is_end);
        if matches!(value.kind, mir::OperandKind::None) {
            return
        }

        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place,
            value: mir::Rvalue::new(mir::RvalueKind::Use(value)),
        });

        self.push_statement(statement);
    }
}

fn is_valid_statement_expression(kind: &hir::ExpressionKind) -> bool {
    match kind {
        hir::ExpressionKind::Null 
        | hir::ExpressionKind::Load(_)
        | hir::ExpressionKind::Local(_)
        | hir::ExpressionKind::DeRef(_)
        | hir::ExpressionKind::Literal(_)
        | hir::ExpressionKind::Ref { .. }
        | hir::ExpressionKind::Cast { .. }
        | hir::ExpressionKind::Unary { .. }
        | hir::ExpressionKind::Binary { .. }
        | hir::ExpressionKind::InnerRawStackArray { .. } => false,
        
        hir::ExpressionKind::Block(_)
        | hir::ExpressionKind::Function(_)
        | hir::ExpressionKind::If { .. }
        | hir::ExpressionKind::Call { .. }
        | hir::ExpressionKind::While { .. } => true,
    }
}
