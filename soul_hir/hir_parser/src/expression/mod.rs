use ast::{Argument, NamedTupleElement, NamedTupleType, VarTypeKind, scope::NodeId};
use hir::{Binary, Expression, ExpressionId, HirType, HirTypeKind, LocalId, Place, PlaceKind, TypeId, Unary};
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind},
    ids::IdAlloc,
    soul_error_internal,
    span::Span,
};

use crate::HirContext;
mod array;
mod r#if;

impl<'a> HirContext<'a> {
    pub fn lower_expression(&mut self, expression: &ast::Expression) -> hir::ExpressionId {
        let id = self.alloc_expression(expression.span);

        let span = expression.span;
        let hir_expression = match &expression.node {
            ast::ExpressionKind::Null(_node_id) => hir::Expression {
                id,
                ty: self.null_ty(span),
                kind: hir::ExpressionKind::Null,
            },
            ast::ExpressionKind::Literal((_id, literal)) => hir::Expression {
                id,
                ty: self.type_from_literal(literal),
                kind: hir::ExpressionKind::Literal(literal.clone()),
            },
            ast::ExpressionKind::Index(index) => {
                let place = Place::new(
                    PlaceKind::Index {
                        id: self.id_generator.alloc_place(),
                        base: Box::new(self.lower_place(&index.collection)),
                        index: self.lower_expression(&index.index),
                    },
                    span,
                );

                hir::Expression {
                    id,
                    ty: self.new_infer_type(span),
                    kind: hir::ExpressionKind::Load(place),
                }
            }
            ast::ExpressionKind::FunctionCall(function_call) => self.lower_call(id, function_call),
            ast::ExpressionKind::Variable {
                ident,
                resolved: _,
                id: option_id,
            } => self.lower_expression_variable(id, ident, *option_id),
            ast::ExpressionKind::If(r#if) => self.lower_if(id, r#if, span),
            ast::ExpressionKind::As(cast) => {
                let value = self.lower_expression(&cast.left);
                let cast_to = self.lower_type(&cast.type_cast);
                let ref_type = self.hir.types.insert_ref(cast_to);
                hir::Expression {
                    id,
                    ty: cast_to,
                    kind: hir::ExpressionKind::Cast { value, cast_to: ref_type },
                }
            }
            ast::ExpressionKind::Unary(unary) => {
                let expression = self.lower_expression(&unary.expression);
                let operator = unary.operator.clone();
                hir::Expression {
                    id,
                    ty: self.new_infer_type(span),
                    kind: hir::ExpressionKind::Unary(Unary {
                        operator,
                        expression,
                    }),
                }
            }
            ast::ExpressionKind::Array(array) => self.lower_array(id, array, span),
            ast::ExpressionKind::Block(block) => return self.lower_block_expression(block),
            ast::ExpressionKind::While(r#while) => {
                let condition = r#while
                    .condition
                    .as_ref()
                    .map(|value| self.lower_expression(value));

                let body = self.lower_block(&r#while.block);
                hir::Expression {
                    id,
                    ty: self.add_type(HirType::none_type()),
                    kind: hir::ExpressionKind::While { condition, body },
                }
            }
            ast::ExpressionKind::Binary(binary) => {
                let left = self.lower_expression(&binary.left);
                let operator = binary.operator.clone();
                let right = self.lower_expression(&binary.right);
                hir::Expression {
                    id,
                    ty: self.new_infer_type(span),
                    kind: hir::ExpressionKind::Binary(Binary {
                        left,
                        operator,
                        right,
                    }),
                }
            }
            ast::ExpressionKind::Deref { id: _, inner } => hir::Expression {
                id,
                ty: self.new_infer_type(span),
                kind: hir::ExpressionKind::DeRef(self.lower_expression(inner)),
            },
            ast::ExpressionKind::Ref {
                id: _,
                is_mutable,
                expression,
            } => self.lower_ref(id, expression, is_mutable, span),
            ast::ExpressionKind::Default(_) => {
                todo!("desugar Default")
            }
            ast::ExpressionKind::ExternalExpression(external) => {
                let _module_id = match self.hir.imports.get_id(&external.path) {
                    Some(id) => id,
                    None => self
                        .hir
                        .imports
                        .insert(&mut self.id_generator.module, external.path.clone()),
                };

                todo!("impl externalExpression")
            }
            ast::ExpressionKind::ReturnLike(_) => {
                panic!("return_like should be unreachable")
            }
        };

        self.insert_expression(id, hir_expression)
    }

    fn lower_block_expression(&mut self, block: &ast::Block) -> hir::ExpressionId {
        let body = self.lower_block(block);

        let ty = match &self.hir.blocks[body].terminator {
            Some(value) => self.hir.expressions[*value].ty,
            None => self.add_type(HirType::none_type()),
        };

        let id = self.alloc_expression(block.span);
        let return_value = hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Block(body),
        };

        self.insert_expression(id, return_value)
    }

    fn lower_ref(
        &mut self,
        id: hir::ExpressionId,
        expression: &ast::Expression,
        is_mutable: &bool,
        span: Span,
    ) -> hir::Expression {
        let inner = self.lower_expression(expression);
        let of_type = self.hir.expressions[inner].ty;

        let local = match &expression.node {
            ast::ExpressionKind::Variable { ident, .. } => match self.find_local(ident) {
                Some(val) => val,
                None => {
                    self.log_error(SoulError::new(
                        format!("'{}' not found in scope", ident.as_str()),
                        SoulErrorKind::NotFoundInScope,
                        Some(ident.span),
                    ));
                    LocalId::error()
                }
            },
            _ => {
                let temp_local = self.id_generator.alloc_local();

                let variable = hir::Variable {
                    ty: of_type,
                    is_temp: false,
                    local: temp_local,
                    value: Some(inner),
                };
                self.insert_desugar_variable(variable, span);
                temp_local
            }
        };

        let place = Place::new(
            PlaceKind::Local(local, self.id_generator.alloc_place()),
            span,
        );
        let ty = self.add_type(HirType::new(HirTypeKind::Ref {
            of_type,
            mutable: *is_mutable,
        }));
        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Ref {
                place,
                mutable: *is_mutable,
            },
        }
    }

    fn lower_expression_variable(
        &mut self,
        id: hir::ExpressionId,
        ident: &Ident,
        option_id: Option<NodeId>,
    ) -> hir::Expression {
        let node_id = option_id.expect("node_id should be Some(_) in hir");
        let var_type_kind = self.ast_store.get_variable_type(node_id);

        let ty = match var_type_kind {
            None => self.new_infer_type(ident.span),
            Some(VarTypeKind::NonInveredType(ty)) => self.lower_type(ty),
            Some(VarTypeKind::InveredType(modifier)) => {
                let modifier = *modifier;
                self.new_infer_with_modifier(modifier, ident.span)
            }
        };

        let local = match self.find_local(ident) {
            Some(val) => val,
            None => {
                #[cfg(debug_assertions)]
                self.log_error(soul_error_internal!(
                    format!("local('{}') not found", ident.as_str()),
                    Some(ident.span)
                ));

                LocalId::error()
            }
        };

        let place_id = self.id_generator.alloc_place();
        let place_kind = match self.hir.locals.get(local) {
            Some(local_info) if local_info.is_temp() => PlaceKind::Temp(local, place_id),
            _ => PlaceKind::Local(local, place_id),
        };

        let place = Place::new(place_kind, ident.span);

        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Load(place),
        }
    }

    fn lower_call(
        &mut self,
        id: hir::ExpressionId,
        function_call: &ast::FunctionCall,
    ) -> hir::Expression {
        let resolved = match function_call.resolved {
            Some(val) => val,
            None => {
                return hir::Expression::error(id);
            }
        };

        let (signature, ty) = match self.ast_store.get_function(resolved) {
            Some(signature) => (signature, Self::convert_type(&signature.return_type, &mut self.hir.types)),
            None => {
                #[cfg(debug_assertions)]
                self.log_error(soul_error_internal!("could not find function", Some(function_call.name.span)));
                return hir::Expression::error(id)
            }
        };

        let callee = function_call
            .callee
            .as_ref()
            .map(|el| self.lower_expression(el));


        let mut arguments = vec![];
        arguments.resize(signature.parameters.len(), ExpressionId::error());

        for (i, argument) in function_call.arguments.iter().enumerate() {
            let parameter_i = match self.get_parameter_index(i, argument, &signature.parameters, function_call.name.span) {
                Ok(val) => val,
                Err(span) => {
                    self.log_error(
                        SoulError::new(format!("parameters of {} argument not found", i), 
                        SoulErrorKind::InvalidContext, 
                        Some(span),
                    ));
                    continue;
                }
            };
            arguments[parameter_i] = self.lower_expression(&argument.value);
        }

        for (i, argument) in arguments.iter_mut().enumerate() {
            if *argument != ExpressionId::error() {
                continue;
            }

            *argument = match &signature.parameters[i].default {
                Some(val) => self.lower_expression(val),
                None => {
                    let span = function_call.name.span;
                    self.log_error(
                        SoulError::new(format!("argument {} not found in function declation", i+1), 
                        SoulErrorKind::InvalidContext, 
                        Some(span),
                    ));
                    let id = self.alloc_expression(span);
                    let err = Expression::error(id);
                    self.insert_expression(id, err)
                }
            };
        }   

        hir::Expression {
            id,
            ty,
            kind: hir::ExpressionKind::Call {
                callee,
                arguments,
                function: resolved,
            },
        }
    }

    fn get_parameter_index(&mut self, i: usize, argument: &Argument, parameters: &Vec<NamedTupleElement>, span: Span) -> Result<usize, Span> {
        
        if let Some(name) = &argument.name {
            let (parameter_i, parameter) = find_default_parameter(name.as_str(), parameters).ok_or(name.span)?;
            if parameter.default.is_none() {
                self.log_error(
                    SoulError::new(format!("{} is not a default parameter", name.as_str()), 
                    SoulErrorKind::InvalidContext, 
                    Some(argument.value.span),
                ));
            }
            Ok(parameter_i)
        } else {
            let parameter = parameters.get(i).ok_or(span)?;
            if parameter.default.is_some() {
                let name = &parameter.name;
                self.log_error(
                    SoulError::new(format!("argument {} is a default parameter should add name (so '{}: <value>')", i+1, name.as_str()), 
                    SoulErrorKind::InvalidContext, 
                    Some(argument.value.span),
                ));
            }
            Ok(i)
        }
    }

    fn null_ty(&mut self, span: Span) -> TypeId {
        let infer = self.new_infer_type(span);
        self.add_type(HirType {
            kind: HirTypeKind::Optional(infer),
            modifier: None,
        })
    }
}

fn find_default_parameter<'a>(name: &str, parameters: &'a NamedTupleType) -> Option<(usize, &'a NamedTupleElement)> {
    parameters.iter().enumerate().filter(|(_, parameter)| parameter.name.as_str() == name).next()
}