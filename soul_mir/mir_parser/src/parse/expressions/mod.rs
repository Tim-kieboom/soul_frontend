use hir::{Binary, ExpressionId, StructId, TypeId, Unary};
use soul_utils::{
    Ident, ids::{FunctionId, IdAlloc}, soul_error_internal, span::Span
};
use typed_hir::ThirTypeKind;

use crate::{EndBlock, MirContext, mir::{self, Operand}};

mod conditionals;

impl<'a> MirContext<'a> {
    pub(crate) fn lower_operand(&mut self, value_id: hir::ExpressionId) -> EndBlock<mir::Operand> {
        let value = &self.hir_response.hir.nodes.expressions[value_id];
        let span = self.expression_span(value_id);
        let value_type = self.expression_type(value_id);
        let is_end = &mut false;

        if let Some(literal) = self.get_expression_literal(value_id) {
            let operand = mir::Operand::new(value_type, mir::OperandKind::Comptime(literal.clone()));
            return EndBlock::new(operand, is_end);
        }

        let operand = match &value.kind {
            hir::ExpressionKind::StructConstructor { ty, values, defaults:_ } => {
                self.lower_struct_constructor(values, *ty, value_type).pass(is_end)
            }
            hir::ExpressionKind::Literal(literal) => {
                mir::Operand::new(value_type, mir::OperandKind::Comptime(literal.clone()))
            }
            hir::ExpressionKind::Local(local_id) => {
                let local_type = self.local_type(*local_id);
                let id = match self.local_remap.get(*local_id) {
                    Some(val) => *val,
                    None => {
                        self.log_error(soul_error_internal!(
                            format!("local_remap could not find {:?}", local_id),
                            Some(span)
                        ));
                        mir::LocalId::error()
                    }
                };
                mir::Operand::new(local_type, mir::OperandKind::Local(id))
            }
            hir::ExpressionKind::Unary(Unary {
                operator,
                expression,
            }) => {
                let inner = self.lower_operand(*expression).pass(is_end);

                let temp = self.new_temp(value_type);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), value_type)),
                    value: mir::Rvalue::new(mir::RvalueKind::Unary {
                        operator: operator.clone(),
                        value: inner,
                    }),
                });
                self.push_statement(statement);
                mir::Operand::new(value_type, mir::OperandKind::Temp(temp))
            }
            hir::ExpressionKind::Binary(Binary {
                left,
                operator,
                right,
            }) => {
                let left = self.lower_operand(*left).pass(is_end);
                let right = self.lower_operand(*right).pass(is_end);

                let temp = self.new_temp(value_type);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), value_type)),
                    value: mir::Rvalue::new(mir::RvalueKind::Binary {
                        left,
                        operator: operator.clone(),
                        right,
                    }),
                });
                self.push_statement(statement);
                mir::Operand::new(value_type, mir::OperandKind::Temp(temp))
            }
            hir::ExpressionKind::Call {
                function,
                callee,
                generics,
                arguments: hir_arguments,
            } => self
                .lower_call(*function, callee, generics, hir_arguments, value_type, span)
                .pass(is_end),
            hir::ExpressionKind::Block(block_id) => {
                let main_body = self.expect_current_block();
                self.lower_block(*block_id, main_body).pass(is_end);

                let operand = match self.hir_response.hir.nodes.blocks[*block_id].terminator {
                    Some(terminator) => {
                        let inner = self.lower_operand(terminator).pass(is_end);
                        let terminator_type = self.expression_type(terminator);
                        let temp = self.new_temp(terminator_type);

                        let place = self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), value_type));
                        self.push_statement(mir::Statement::new(mir::StatementKind::Assign {
                            place,
                            value: mir::Rvalue::new(mir::RvalueKind::Use(inner)),
                        }));

                        mir::Operand::new(value_type, mir::OperandKind::Temp(temp))
                    }
                    None => mir::Operand::new(value_type, mir::OperandKind::None),
                };

                operand
            }

            hir::ExpressionKind::Null => {
                self.log_error(soul_error_internal!(
                    "ExpressionKind::Null  not yet impl in mir",
                    Some(span)
                ));
                mir::Operand::new(value_type, mir::OperandKind::None)
            }
            hir::ExpressionKind::Function(_) => {
                self.log_error(soul_error_internal!(
                    "ExpressionKind::Function not yet impl in mir",
                    Some(span)
                ));
                mir::Operand::new(value_type, mir::OperandKind::None)
            }

            hir::ExpressionKind::Load(place) => self.lower_load(value_type, *place, is_end),

            hir::ExpressionKind::DeRef(inner) => {
                let ptr = self.lower_operand(*inner).pass(is_end);
                let temp = self.new_temp(value_type);

                let statement = mir::Statement::new(mir::StatementKind::Assign {
                    place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), value_type)),
                    value: mir::Rvalue::new(mir::RvalueKind::Use(ptr)),
                });

                self.push_statement(statement);
                mir::Operand::new(value_type, mir::OperandKind::Temp(temp))
            }

            hir::ExpressionKind::Ref { place, mutable } => {
                let ty = self.hir_response.typed.types_table.places[*place];

                let place_id = self.lower_place(*place).pass(is_end);

                mir::Operand::new(
                    ty,
                    mir::OperandKind::Ref {
                        place: place_id,
                        mutable: *mutable,
                    },
                )
            }

            hir::ExpressionKind::Cast { value, cast_to:_ } => {

                let cast_to = self.hir_response.typed.types_table.expressions[value_id];
                let inner_type = self.expression_type(*value);
                if inner_type == cast_to {
                    self.lower_operand(*value).pass(is_end)
                } else {
                    let value = self.lower_operand(*value).pass(is_end);
                    let temp = self.new_temp(cast_to);

                    let statement = mir::Statement::new(mir::StatementKind::Assign {
                        place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), value_type)),
                        value: mir::Rvalue::new(mir::RvalueKind::CastUse {
                            value,
                            cast_to,
                        }),
                    });

                    self.push_statement(statement);

                    mir::Operand::new(value_type, mir::OperandKind::Temp(temp))
                }
            }

            hir::ExpressionKind::InnerRawStackArray { .. } => {
                mir::Operand::new(self.hir_response.typed.types_table.none_type, mir::OperandKind::None)
            }

            hir::ExpressionKind::If {
                condition,
                then_block,
                else_block,
                ends_with_else: _,
            } => self.lower_if(*condition, *then_block, *else_block, self.hir_response.typed.types_table.expressions[value_id], is_end),
            hir::ExpressionKind::While { condition, body } => {
                self.lower_while(*condition, *body, is_end)
            }
            hir::ExpressionKind::Error => {
                mir::Operand::new(self.hir_response.typed.types_table.none_type, mir::OperandKind::None)
            }
        };

        EndBlock::new(operand, is_end)
    }

    pub(crate) fn lower_call(
        &mut self,
        function_id: FunctionId,
        callee: &Option<hir::ExpressionId>,
        hir_generics: &Vec<TypeId>,
        hir_arguments: &Vec<hir::ExpressionId>,
        ty: hir::TypeId,
        span: Span,
    ) -> EndBlock<mir::Operand> {
        let is_end = &mut false;

        if callee.is_some() {
            self.log_error(soul_error_internal!(
                "function call callee not yet impl",
                Some(span)
            ));
        }

        let function = &self.hir_response.hir.nodes.functions[function_id];
        let parameters = &function.parameters;
        let mut arguments = vec![];
        for (i, parameter) in parameters.iter().enumerate() {
            let arg = match hir_arguments.get(i) {
                Some(val) => *val,
                None => match parameter.default {
                    Some(val) => val,
                    None => continue,
                },
            };

            let expression = &self.hir_response.hir.nodes.expressions[arg];
            let mut value = self.lower_operand(arg).pass(is_end);

            let ty = self.local_type(parameters[i].local);

            let param_type = self.id_to_type(ty).clone();
            let arg_type = self.id_to_type(self.expression_type(arg)).clone();
            
            let warning = "check for primitive catablility";
            if expression.is_literal() {
                let temp = self.new_temp(ty);
                let place = self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), ty));
                let rvalue = mir::Rvalue::new(mir::RvalueKind::CastUse { value, cast_to: ty });
                let cast = mir::Statement::new(mir::StatementKind::Assign {
                    place,
                    value: rvalue,
                });
                self.push_statement(cast);

                value = mir::Operand::new(ty, mir::OperandKind::Temp(temp));
            }
            arguments.push(value);
        }

        let temp = if self.id_to_type(ty).kind == ThirTypeKind::None {
            None
        } else {
            Some(self.new_temp(ty))
        };
        let return_place = temp.map(|val| self.new_place(mir::Place::new(mir::PlaceKind::Temp(val), ty)));

        let statement = mir::Statement::new(mir::StatementKind::Call {
            id: function_id,
            type_args: hir_generics.clone(),
            arguments,
            return_place,
        });
        self.push_statement(statement);

        let operand = match temp {
            Some(val) => mir::Operand::new(ty, mir::OperandKind::Temp(val)),
            None => mir::Operand::new(ty, mir::OperandKind::None),
        };

        EndBlock::new(operand, is_end)
    }

    fn lower_struct_constructor(&mut self, values: &Vec<(Ident, ExpressionId)>, struct_id: StructId, struct_type: TypeId) -> EndBlock<Operand> {
        let r#struct = self.hir_response
            .typed
            .types_map
            .id_to_struct(struct_id)
            .expect("should have struct");

        let dummy = Operand::new(TypeId::error(), mir::OperandKind::None);
        let is_end = &mut false;
        
        let mut runtime = false;
        let mut fields = Vec::new();

        fields.resize(r#struct.fields.len(), dummy);

        for (name, value) in values {

            
            let (i, _) = match r#struct.fields.iter().enumerate().find(|(_i, field)| self.hir_response.hir.nodes.fields[field.id].name == name.as_str()) {
                Some(val) => val,
                None => continue,
            };

            let value_type = self.expression_type(*value);
            let operand = match self.get_expression_literal(*value) {
                Some(literal) => Operand::new(value_type, mir::OperandKind::Comptime(literal.clone())),
                None => {
                    runtime = true;
                    self.lower_operand(*value).pass(is_end)
                }
            };
            fields[i] = operand;
        }
        
        let body = if runtime {
            mir::AggregateBody::Runtime(fields)
        } else {
            let literals = fields
                .into_iter()
                .enumerate()
                .map(|(i, op)| {
                    let ty = r#struct.fields[i].ty;
                    match op.kind {
                        mir::OperandKind::Comptime(literal) => (literal, ty),
                        _ => unreachable!(),
                    }
                }).collect();
            
            mir::AggregateBody::Comptime(literals)
        };

        let ctor = mir::RvalueKind::Aggregate { struct_type: struct_id, body };
        let temp = self.new_temp(struct_type);

        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), struct_type)),
            value: mir::Rvalue::new(ctor),
        });
        self.push_statement(statement);
        let operand = mir::Operand::new(struct_type, mir::OperandKind::Temp(temp));
        EndBlock::new(operand, is_end)
    }

    fn lower_load(&mut self, ty: TypeId, place: hir::PlaceId, is_end: &mut bool) -> mir::Operand {
        let place_id = self.lower_place(place).pass(is_end);
        let operand = match &self.tree.places[place_id].kind {
            mir::PlaceKind::Local(local) => {
                return mir::Operand::new(ty, mir::OperandKind::Local(*local));
            }
            _ => mir::Operand::new(ty, mir::OperandKind::Temp(self.place_to_temp(place_id, ty))),
        };

        let temp = self.new_temp(ty);

        let statement = mir::Statement::new(mir::StatementKind::Assign {
            place: self.new_place(mir::Place::new(mir::PlaceKind::Temp(temp), ty)),
            value: mir::Rvalue::new(mir::RvalueKind::Use(operand)),
        });

        self.push_statement(statement);
        mir::Operand::new(ty, mir::OperandKind::Temp(temp))
    }
}
