use std::{iter::{self}, sync::LazyLock};
use crate::steps::{parse::parser::{Parser, TryError}, tokenize::token_stream::TokenKind};
use models::{abstract_syntax_tree::{block::Block, expression::{Expression, ExpressionKind, ReturnKind}, operator::{Binary, BinaryOperator, BinaryOperatorKind}, soul_type::{SoulType}, statment::{Assignment, Ident, Statement, StatementKind, Variable}}, error::{SoulError, SoulErrorKind, SoulResult, Span}, scope::scope::ValueSymbol, soul_names::{self, AssignType, KeyWord, TypeModifier}, symbool_kind::SymboolKind};

pub const SQUARE_CLOSE: TokenKind = TokenKind::Symbool(SymboolKind::SquareClose);
pub const CURLY_OPEN: TokenKind = TokenKind::Symbool(SymboolKind::CurlyOpen);
pub const CURLY_CLOSE: TokenKind = TokenKind::Symbool(SymboolKind::CurlyClose);
pub const ROUND_OPEN: TokenKind = TokenKind::Symbool(SymboolKind::RoundOpen);
pub const ROUND_CLOSE: TokenKind = TokenKind::Symbool(SymboolKind::RoundClose);
pub const ARROW_LEFT: TokenKind = TokenKind::Symbool(SymboolKind::LeftArray);
pub const COLON: TokenKind = TokenKind::Symbool(SymboolKind::Colon);
pub const SEMI_COLON: TokenKind = TokenKind::Symbool(SymboolKind::SemiColon);
pub const COMMA: TokenKind = TokenKind::Symbool(SymboolKind::Comma);
pub const ASSIGN: TokenKind = TokenKind::Symbool(SymboolKind::Assign);
pub const COLON_ASSIGN: TokenKind = TokenKind::Symbool(SymboolKind::ColonAssign);
pub const STAMENT_END_TOKENS: &[TokenKind] = &[
    CURLY_CLOSE,
    TokenKind::EndFile,
    TokenKind::EndLine,
    TokenKind::Symbool(SymboolKind::SemiColon),
];

impl<'a> Parser<'a> {

    pub(crate) fn parse_block(&mut self, modifier: TypeModifier) -> SoulResult<Block> {
        const END_TOKENS: &[TokenKind] = &[
            CURLY_CLOSE, 
            TokenKind::EndFile,
        ];

        let mut statments = vec![];

        let scope_id = self.push_scope();

        self.expect(&CURLY_OPEN)?;
        while !self.current_is_any(END_TOKENS) {

            match self.parse_statement() {
                Ok(statment) => statments.push(statment),
                Err(err) => {
                    self.add_error(err);
                    self.skip_over_statement();
                }
            }

            self.skip(&[SEMI_COLON, TokenKind::EndLine]);
        }
        
        self.expect(&CURLY_CLOSE)?;
        Ok(Block{modifier, statments, scope_id})
    }

    pub(crate) fn skip_over_statement(&mut self) {

        while !self.current_is(&TokenKind::EndFile) {
            self.bump();
            
            if self.current_is_any(STAMENT_END_TOKENS) {
                return
            }
        }
    }

    pub(crate) fn parse_statement(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;

        self.skip(STAMENT_END_TOKENS);

        let possible_kind = match &self.token().kind {

            TokenKind::Ident(ident) => {

                if let Some(modifier) = soul_names::TypeModifier::from_str(&ident) {
                    self.parse_statement_modifier(start_span, modifier)?
                }
                else if let Some(keyword) = soul_names::KeyWord::from_str(&ident) {
                    self.parse_stament_keyword(start_span, keyword)?
                }
                else {
                    Some(self.parse_statement_ident(start_span)?)
                }
            },
            &CURLY_OPEN => Some(Statement::new_block(
                self.parse_block(TypeModifier::Mut)?, 
                self.new_span(start_span),
            )),
            TokenKind::Unknown(char) => return Err(
                SoulError::new(
                    format!("unknown char: '{char}'"), 
                    SoulErrorKind::InvalidChar, 
                    Some(start_span),
                ),
            ),
            _ => None
        };

        if let Some(kind) = possible_kind {
            Ok(kind)
        }
        else {

            match self.parse_expression(STAMENT_END_TOKENS) {
                Ok(expression) => Ok(
                    Statement::new(StatementKind::Expression(expression), self.new_span(start_span))
                ),
                Err(err) => {
                    self.skip_over_statement();
                    Err(err)
                },
            }
        }
    }
    
    fn parse_statement_ident(&mut self, start_span: Span) -> SoulResult<Statement> {
        const FUNCTION_IDS: &[TokenKind] = &[ROUND_OPEN, ARROW_LEFT];
        const DECLARATION_IDS: &[TokenKind] = &[COLON, COLON_ASSIGN];
            
        let peek = self.peek();

        if FUNCTION_IDS.contains(&peek.kind) {
            let ident_token = self.bump_consume();
            let ident = match ident_token.kind {
                TokenKind::Ident(val) => val,
                _ => unreachable!(),
            };

            return match self.parse_function_declaration(start_span, TypeModifier::Mut, None, ident) {
                Ok(val) => Ok(val),
                Err(TryError::IsErr(err)) => Err(err),
                Err(TryError::IsNotValue(ident)) => self.parse_function_call(start_span, None, ident).map(|expression| Statement::from_expression(expression)),
            }
        }
        else if DECLARATION_IDS.contains(&peek.kind) {
            let ident_token = self.bump_consume();
            let ident = match ident_token.kind {
                TokenKind::Ident(val) => val,
                _ => unreachable!(),
            };

            let (ty, assign_type) = if self.current_is(&COLON) {
                self.bump();
                let ty = self.parse_type()?;
                self.expect_any(&[COLON_ASSIGN, ASSIGN])?;
                (Some(ty), AssignType::Assign)
            }
            else {
                (None, AssignType::Assign)
            };

            let variable = self.parse_variable_declaration(start_span, ident.clone(), assign_type, ty)?;
            self.add_scope_value(ident.clone(), ValueSymbol::Variable(variable));
            return Ok(Statement::new(
                StatementKind::Variable(ident), 
                self.new_span(start_span),
            ));
        }
        else {
            return self.parse_assignment_or_expression(start_span);
        }
    }

    fn parse_statement_modifier(&mut self, start_span: Span, modifier: TypeModifier) -> SoulResult<Option<Statement>> {
        self.bump();

        if self.token().kind == CURLY_OPEN {
            let block = self.parse_block(modifier)?;
            return Ok(Some(
                Statement::new_block(block, self.new_span(start_span))
            ))
        }

        if let Some(name) = self.try_consume_name(start_span)? {
            
            if self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
                todo!("function call/decl")
            }
            
            let mut ty = None;
            if self.current_is(&COLON) {
                self.bump();
                ty = Some(self.parse_type()?);
            }

            if self.current_is_any(STAMENT_END_TOKENS) {

                self.add_scope_value(
                    name.clone(), 
                    ValueSymbol::new_variable(
                        name.clone(),
                        ty.unwrap_or(SoulType::none()),
                        None,
                    ),
                );

                return Ok(Some(
                    Statement::new(StatementKind::Variable(name), self.new_span(start_span))
                ))
            }

            if let Some(assign_type) = try_get_assign_type(&self.token().kind) {
                let variable = self.parse_variable_declaration(start_span, name.clone(), assign_type, ty)?;
                self.add_scope_value(name.clone(), ValueSymbol::Variable(variable));

                return Ok(Some(
                    Statement::new(StatementKind::Variable(name), self.new_span(start_span))
                )) 
            }
            else {

                return Err(
                    SoulError::new(
                        format!("'{}' should be '=' or ':='", self.token().kind.display()),
                        SoulErrorKind::InvalidAssignType,
                        Some(self.new_span(start_span)),
                    )
                )
            } 
        }
        
        Err(
            SoulError::new(
                format!("'{}' invalid after modifier (could be ['{{' or <name>])", self.token().kind.display()),
                SoulErrorKind::UnexpecedToken,
                Some(self.new_span(start_span)),
            )
        )
    }

    fn parse_stament_keyword(&mut self, start_span: Span, keyword: KeyWord) -> SoulResult<Option<Statement>> {
        
        let kind = match keyword {
                
            KeyWord::Use => todo!("use decl"),
            KeyWord::Enum => todo!("enum decl"),
            KeyWord::Class => todo!("class decl"),
            KeyWord::Trait => todo!("trait decl"),
            KeyWord::Union => todo!("union decl"),
            KeyWord::Struct => todo!("struct decl"),
            
            KeyWord::If => Some(
                Statement::from_expression(self.parse_if()?)
            ), 
            KeyWord::For => Some(
                Statement::from_expression(self.parse_for()?)
            ), 
            KeyWord::Match => Some(
                Statement::from_expression(self.parse_match()?)
            ), 
            KeyWord::While => Some(
                Statement::from_expression(self.parse_while()?)
            ), 
            KeyWord::Return => Some(
                Statement::from_expression(self.parse_return_like(ReturnKind::Return)?)
            ), 
            KeyWord::Continue => Some(
                Statement::from_expression(self.parse_return_like(ReturnKind::Continue)?)
            ), 
            KeyWord::Break => Some(
                Statement::from_expression(self.parse_return_like(ReturnKind::Break)?)
            ), 
            
            KeyWord::Else => return Err(
                SoulError::new(
                    format!("can not have '{}' without first '{}'", KeyWord::Else.as_str(), KeyWord::If.as_str()),
                    SoulErrorKind::InvalidExpression,
                    Some(self.new_span(start_span)),
                )
            ),

            KeyWord::Copy |
            KeyWord::Await |
            KeyWord::InForLoop |
            KeyWord::GenericWhere => return Err(
                SoulError::new(
                    format!("keyword: '{}' invalid start to statment", keyword.as_str()), 
                    SoulErrorKind::UnexpecedStatmentStart,
                    Some(self.new_span(start_span)), 
                ),
            )
        };

        Ok(kind)
    }

    fn parse_assignment_or_expression(&mut self, start_span: Span) -> SoulResult<Statement> {
        static ASSIGNMENT_TOKENS: LazyLock<Vec<TokenKind>> = LazyLock::new(|| 
            AssignType::SYMBOLS
                .iter()
                .map(|sym| TokenKind::Symbool(*sym))
                .chain(iter::once(TokenKind::EndLine))
                .chain(iter::once(SEMI_COLON))
                .collect()
        );
        
        let lvalue = self.parse_expression(ASSIGNMENT_TOKENS.as_ref())?;
        if self.current_is_any(STAMENT_END_TOKENS)  {

            return Ok(
                Statement::new(
                    StatementKind::Expression(lvalue),
                    self.new_span(start_span), 
                )
            )
        }

        let assign_token = self.bump_consume();
        let assign = try_get_assign_type(&assign_token.kind)
            .ok_or(
                SoulError::new(
                    format!("'{}' should be an assign symbool", assign_token.kind.display()),
                    SoulErrorKind::InvalidAssignType,
                    Some(self.new_span(start_span)),
                )
            )?;

        let rvalue = self.parse_expression(STAMENT_END_TOKENS)?;
        let resolved_rvalue = resolve_assign_type(&lvalue, assign, assign_token.span, rvalue);

        self.bump();

        Ok(
            Statement::new(
                StatementKind::Assignment(Assignment{left: lvalue, right: resolved_rvalue}),
                self.new_span(start_span), 
            )
        )
    }

    fn parse_variable_declaration(&mut self, start_span: Span, variable_name: Ident, assign_type: AssignType, ty: Option<SoulType>) -> SoulResult<Variable> {
        
        let expression = match assign_type {
            AssignType::Declaration |
            AssignType::Assign => {
                self.bump();
                self.parse_expression(STAMENT_END_TOKENS)?
            },
            other => {
                return Err(SoulError::new(
                    format!("'{}' is not valid for variable declaration (can use ['=', ':='])", other.as_str()),
                    SoulErrorKind::InvalidAssignType,
                    Some(self.new_span(start_span)),
                ))
            },
        };

        Ok(Variable{
            ty: ty.unwrap_or(SoulType::none()), 
            name: variable_name, 
            initialize_value: Some(expression),
        })
    }
}

fn try_get_assign_type(token: &TokenKind) -> Option<AssignType> {
    
    if let TokenKind::Symbool(symbool) = token {
        AssignType::from_symbool(*symbool)
    }
    else {
        None
    }
}

fn resolve_assign_type(lvalue: &Expression, assign_type: AssignType, assign_span: Span, rvalue: Expression) -> Expression {
    
    let full_span = assign_span.combine(rvalue.span);

    let operator = match assign_type {
        AssignType::AddAssign => BinaryOperator::new(BinaryOperatorKind::Add, assign_span),
        AssignType::SubAssign => BinaryOperator::new(BinaryOperatorKind::Sub, assign_span),
        AssignType::MulAssign => BinaryOperator::new(BinaryOperatorKind::Mul, assign_span),
        AssignType::DivAssign => BinaryOperator::new(BinaryOperatorKind::Div, assign_span),
        AssignType::ModAssign => BinaryOperator::new(BinaryOperatorKind::Mod, assign_span),
        AssignType::BitOrAssign => BinaryOperator::new(BinaryOperatorKind::BitOr, assign_span),
        AssignType::BitAndAssign => BinaryOperator::new(BinaryOperatorKind::BitAnd, assign_span),
        AssignType::BitXorAssign => BinaryOperator::new(BinaryOperatorKind::BitXor, assign_span),

        AssignType::Assign |
        AssignType::Declaration => return rvalue,
    };

    Expression::new(
        ExpressionKind::Binary(
            Binary::new(
                lvalue.clone(), 
                operator, 
                rvalue)
        ), 
        full_span
    )
}