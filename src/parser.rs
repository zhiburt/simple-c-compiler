/// Parse package does all stuff to create AST
/// 
/// TODO: should we have rejected logic when we remove(0) from tokens
/// might be better to check it and if something wrong fail?
/// but not effect original vector, but it's not very crucial now, until we return tokens even in error,
/// or take &tokens not move them

use crate::{Token, TokenType, ast};

use std::error;
use std::fmt;

pub type Result<T> = std::result::Result<T, CompilerError>;

#[derive(Debug)]
pub enum CompilerError {
    ParsingError,
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "syntax_err")
    }
}

impl error::Error for CompilerError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

fn tokens_to_types(tokens: &Vec<Token>) -> Vec<TokenType> {
    tokens.iter().map(|t| t.token_type).collect()
}

fn parse_expr<ParsExpFunc>(parse: ParsExpFunc, opt_tokens: &[TokenType], tokens: Vec<Token>)
    -> Result<(ast::Exp, Vec<Token>)>
    where ParsExpFunc : Fn(Vec<Token>) -> Result<(ast::Exp, Vec<Token>)>
{
    let (mut exp, mut tokens) = parse(tokens)?;
    while let Some(tok) = tokens.get(0)  {
            if !opt_tokens.contains(&tok.token_type) {
                break;
            }
            
            let tok_type = tokens.remove(0).token_type;
            let (right, stashed_tokens) = parse(tokens)?;
            let op = map_token_to_ast(tok_type).unwrap();
            exp = ast::Exp::BinOp(op, Box::new(exp), Box::new(right));
            tokens = stashed_tokens;
    }

    Ok((exp, tokens))
}

fn map_token_to_ast(t: TokenType) -> Option<ast::BinOp> {
    match t {
            TokenType::BitwiseXor => Some(ast::BinOp::BitwiseXor),
            TokenType::BitwiseOr => Some(ast::BinOp::BitwiseOr),
            TokenType::BitwiseAnd => Some(ast::BinOp::BitwiseAnd),
            TokenType::Addition => Some(ast::BinOp::Addition),
            TokenType::Negation => Some(ast::BinOp::Sub),
            TokenType::Multiplication => Some(ast::BinOp::Multiplication),
            TokenType::Division => Some(ast::BinOp::Division),
            TokenType::Modulo => Some(ast::BinOp::Modulo),
            TokenType::And => Some(ast::BinOp::And),
            TokenType::Or => Some(ast::BinOp::Or),
            TokenType::Equal => Some(ast::BinOp::Equal),
            TokenType::NotEqual => Some(ast::BinOp::NotEqual),
            TokenType::LessThan => Some(ast::BinOp::LessThan),
            TokenType::LessThanOrEqual => Some(ast::BinOp::LessThanOrEqual),
            TokenType::GreaterThan => Some(ast::BinOp::GreaterThan),
            TokenType::GreaterThanOrEqual => Some(ast::BinOp::GreaterThanOrEqual),
            TokenType::BitwiseLeftShift => Some(ast::BinOp::BitwiseLeftShift),
            TokenType::BitwiseRightShift => Some(ast::BinOp::BitwiseRightShift),
            _ => None,
    }
}

fn map_token_to_unop(t: TokenType) -> Option<ast::UnOp> {
    match t {
        TokenType::BitwiseComplement => Some(ast::UnOp::BitwiseComplement),
        TokenType::LogicalNegation => Some(ast::UnOp::LogicalNegation),
        TokenType::Negation => Some(ast::UnOp::Negation),
        _ => None,
    }
}

fn map_inc_dec_token_to_unop(t: TokenType, postfix: bool) -> Option<ast::UnOp> {
    if postfix {
        match t {
            TokenType::Increment => Some(ast::UnOp::IncrementPostfix),
            TokenType::Decrement => Some(ast::UnOp::DecrementPostfix),
            _ => None,
        }
    } else {
        match t {
            TokenType::Increment => Some(ast::UnOp::IncrementPrefix),
            TokenType::Decrement => Some(ast::UnOp::DecrementPrefix),
            _ => None,
        }
    }
}

pub fn is_operators(t: &[Token], operators: &[TokenType]) -> bool {
    for (i, op) in operators.iter().enumerate() {
        match t.get(i) {
            Some(tok) if tok.token_type != *op => return false,
            None => return false,
            _ => (),
        };
    }

    true
}

pub fn parse_exp(mut tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::Assignment) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::Assign(var.val.unwrap().to_owned(), Box::new(exp)), tokens))
    } else if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::AssignmentPlus) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::AssignOp(var.val.unwrap().to_owned(), ast::AssignmentOp::Plus, Box::new(exp)), tokens))
    } else if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::AssignmentSub) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::AssignOp(var.val.unwrap().to_owned(), ast::AssignmentOp::Sub, Box::new(exp)), tokens))
    } else if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::AssignmentMul) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::AssignOp(var.val.unwrap().to_owned(), ast::AssignmentOp::Mul, Box::new(exp)), tokens))
    } else if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::AssignmentDiv) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::AssignOp(var.val.unwrap().to_owned(), ast::AssignmentOp::Div, Box::new(exp)), tokens))
    } else if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::AssignmentMod) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::AssignOp(var.val.unwrap().to_owned(), ast::AssignmentOp::Mod, Box::new(exp)), tokens))
    } else if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::AssignmentBitLeftShift) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::AssignOp(var.val.unwrap().to_owned(), ast::AssignmentOp::BitLeftShift, Box::new(exp)), tokens))
    } else if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::AssignmentBitRightShift) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::AssignOp(var.val.unwrap().to_owned(), ast::AssignmentOp::BitRightShift, Box::new(exp)), tokens))
    } else if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::AssignmentBitAnd) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::AssignOp(var.val.unwrap().to_owned(), ast::AssignmentOp::BitAnd, Box::new(exp)), tokens))
    } else if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::AssignmentBitOr) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::AssignOp(var.val.unwrap().to_owned(), ast::AssignmentOp::BitOr, Box::new(exp)), tokens))
    } else if tokens[0].is_type(TokenType::Identifier)
        && tokens[1].is_type(TokenType::AssignmentBitXor) {
        let var = tokens.remove(0);
        tokens.remove(0);
        let (exp, tokens) = parse_exp(tokens)?;

        Ok((ast::Exp::AssignOp(var.val.unwrap().to_owned(), ast::AssignmentOp::BitXor, Box::new(exp)), tokens))
    } else {
        parse_expr(parse_or_expr, &[TokenType::Or], tokens)
    }
}

pub fn parse_or_expr(tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    parse_expr(parse_and_expr, &[TokenType::Or], tokens)
}

pub fn parse_and_expr(tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    parse_expr(parse_equality_expr, &[TokenType::And], tokens)
}

pub fn parse_equality_expr(tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    parse_expr(parse_relational_expr, &[TokenType::Equal, TokenType::NotEqual], tokens)
}

pub fn parse_relational_expr(tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    parse_expr(parse_addictive_expr, &[TokenType::GreaterThan, TokenType::GreaterThanOrEqual, TokenType::LessThan, TokenType::LessThanOrEqual], tokens)
}

pub fn parse_addictive_expr(tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    parse_expr(parse_bitwise_expr, &[TokenType::Addition, TokenType::Negation], tokens)
}

pub fn parse_bitwise_expr(tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    parse_expr(parse_un_op_term, &[TokenType::BitwiseLeftShift, TokenType::BitwiseRightShift], tokens)
}

pub fn parse_un_op_term(tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    parse_expr(parse_term, &[TokenType::BitwiseAnd, TokenType::BitwiseOr, TokenType::BitwiseXor], tokens)
}

pub fn parse_term(tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    parse_expr(parse_factor, &[TokenType::Multiplication, TokenType::Modulo, TokenType::Division], tokens)
}

pub fn parse_factor(mut tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    let picked_token = tokens.get(0).unwrap();
    match picked_token.token_type {
        TokenType::OpenParenthesis => {
            let mut token = tokens.remove(0);
            let (expr, mut tokens) = parse_exp(tokens).unwrap();
            token = tokens.remove(0);
            if token.token_type != TokenType::CloseParenthesis {
                return Err(CompilerError::ParsingError);
            }
            Ok((expr, tokens))
        }
        TokenType::Identifier => {
            let token = tokens.remove(0);
            let var = ast::Exp::Var(token.val.unwrap().to_owned());
            match tokens.get(0) {
                Some(tok) if tok.is_type(TokenType::Decrement) || tok.is_type(TokenType::Increment) => {
                    let tok_type = tok.token_type;
                    tokens.remove(0);
                    Ok((ast::Exp::UnOp(map_inc_dec_token_to_unop(tok_type, true).unwrap(), Box::new(var)), tokens))
                }
                _ => Ok((var, tokens)),
            }
        }
        TokenType::IntegerLiteral => {
            let token = tokens.remove(0);
            Ok((ast::Exp::Const(ast::Const::Int(token.val.as_ref().unwrap().parse().unwrap())), tokens))
        }
        TokenType::Negation | TokenType::LogicalNegation | TokenType::BitwiseComplement => {
            let token = tokens.remove(0);
            let (expr, tokens) = parse_expr(parse_factor, &[TokenType::Or], tokens).unwrap();
            Ok((ast::Exp::UnOp(map_token_to_unop(token.token_type).unwrap(), Box::new(expr)), tokens))
        }
        _ => parse_inc_dec_expr(tokens),
    }
}


pub fn parse_inc_dec_expr(mut tokens: Vec<Token>) -> Result<(ast::Exp, Vec<Token>)> {
    let mut token = tokens.remove(0);
    match token.token_type {
        TokenType::Increment => {
            let (expr, tokens) = parse_expr(parse_factor, &[TokenType::Or], tokens).unwrap();
            Ok((ast::Exp::UnOp(map_inc_dec_token_to_unop(token.token_type, false).unwrap(), Box::new(expr)), tokens))
        }
        TokenType::Decrement => {
            let (expr, tokens) = parse_expr(parse_factor, &[TokenType::Or], tokens).unwrap();
            Ok((ast::Exp::UnOp(map_inc_dec_token_to_unop(token.token_type, false).unwrap(), Box::new(expr)), tokens))
        }
        _ => Err(CompilerError::ParsingError),
    }
}

pub fn parse_statement(mut tokens: Vec<Token>) -> Result<(ast::Statement, Vec<Token>)> {
    let (stat, mut tokens) = match tokens.get(0).unwrap().token_type {
        TokenType::Return => {
            tokens.remove(0);
            let (exp, mut tokens) = parse_exp(tokens).unwrap();
            (ast::Statement::Return{exp: exp}, tokens)
        },
        _ => {
            let (exp, tokens) = parse_exp(tokens)?;
            (ast::Statement::Exp{exp: exp}, tokens)
        }
    };
    compare_token(tokens.remove(0), TokenType::Semicolon).unwrap();

    Ok((stat, tokens))
}

/// TODO: should we take off the parte with parse_decl?
/// currently we check is it decl if it's we parse it.
/// New function is not created since it dublication of code some kinda
pub fn parse_block_item(mut tokens: Vec<Token>) -> Result<(ast::BlockItem, Vec<Token>)> {
    match tokens.get(0) {
        Some(tok) if tok.token_type == TokenType::Int => {
            tokens.remove(0);
            let var = compare_token(tokens.remove(0), TokenType::Identifier)?;
            let exp = match tokens.get(0) {
                Some(tok) if tok.is_type(TokenType::Assignment) => {
                    tokens.remove(0);
                    let (exp, toks) = parse_exp(tokens)?;
                    tokens = toks;
                    Some(exp)
                } ,
                _ => None,
            };
            compare_token(tokens.remove(0), TokenType::Semicolon).unwrap();

            Ok((ast::BlockItem::Declaration(ast::Declaration::Declare{name: var.val.unwrap().to_owned(), exp: exp}), tokens))
        },
        _ =>  {
            let (state, tokens) = parse_statement(tokens)?;
            Ok((ast::BlockItem::Statement(state), tokens))
        },
    }
}

pub fn parse_func(mut tokens: Vec<Token>) -> Result<(ast::FuncDecl, Vec<Token>)> {
    compare_token(tokens.remove(0), TokenType::Int).unwrap();
    let func_name = compare_token(tokens.remove(0), TokenType::Identifier).unwrap();
    compare_token(tokens.remove(0), TokenType::OpenParenthesis).unwrap();
    compare_token(tokens.remove(0), TokenType::CloseParenthesis).unwrap();
    compare_token(tokens.remove(0), TokenType::OpenBrace).unwrap();

    let mut blocks = Vec::new();
    while tokens.get(0).unwrap().token_type != TokenType::CloseBrace {
        let (block, toks) = parse_block_item(tokens).unwrap();
        blocks.push(block);
        tokens = toks;
    } 
    tokens.remove(0);

    Ok((ast::FuncDecl{name: func_name.val.unwrap().clone(), blocks: blocks}, tokens))
}

pub fn parse(tokens: Vec<Token>) -> Result<ast::Program> {
    let (decl, _) = parse_func(tokens)?;
    Ok(ast::Program(decl))
}

fn compare_token(tok: Token, tok_type: TokenType) -> Result<Token> {
    if tok.token_type == tok_type {
        Ok(tok)
    } else {
        Err(CompilerError::ParsingError)
    }
}
