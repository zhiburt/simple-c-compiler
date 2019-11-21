use crate::{ast};
use std::collections::HashMap;

pub fn gen(p: ast::Program, start_point: &str) -> Result<String> {
    let header = format!("\t.globl {}", start_point);
    let mut asm_func = AsmFunc::new();
    Ok(format!("{}\n{}", header, asm_func.gen(&p.0)?))
}

pub type Result<T> = std::result::Result<T, GenError>;

#[derive(Debug)]
pub enum GenError {
    InvalidVariableUsage(String),
}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GenError::InvalidVariableUsage(var) => write!(f, "gen error {}", var),
        }
    }
}

impl std::error::Error for GenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

const PLATFORM_WORD_SIZE: i64 = 8;

struct AsmFunc {
    variable_map: HashMap<String, i64>,
    stack_index: i64,
}

impl AsmFunc {
    fn new() -> Self {
        AsmFunc {
            variable_map: HashMap::new(),
            stack_index: -PLATFORM_WORD_SIZE,
        }
    }

    fn gen(&mut self, st: &ast::Declaration) -> Result<String> {
        match st {
            ast::Declaration::Func{name, statements} => {
                let prologue = vec![
                    "push %rbp".to_owned(),
                    "mov %rsp, %rbp".to_owned(),
                ];
                let epilogue = vec![
                    "mov %rbp, %rsp".to_owned(),
                    "pop %rbp".to_owned(),
                    "ret".to_owned(),
                ];

                let mut code = Vec::new();
                code.extend(prologue);

                
                let return_exists = statements.iter().any(|stat| match stat {
                    ast::Statement::Return{..} => true,
                    _ => false,
                });

                for statement in statements {
                    code.extend(self.gen_statement(statement)?);
                }

                if !return_exists {
                    code.push("ret $0".to_owned());
                }

                code.extend(epilogue);

                let mut pretty_code = code
                    .iter()
                    .map(|c| format!("\t{}", c))
                    .collect::<Vec<String>>();
                let func_name = format!("{}:", name);
                pretty_code.insert(0, func_name);
                Ok(pretty_code.join("\n"))
            }
        }
    }

    fn gen_statement(&mut self, st: &ast::Statement) -> Result<Vec<String>> {
        match st {
            ast::Statement::Return{exp} | ast::Statement::Exp{exp} => self.gen_expr(&exp),
            ast::Statement::Declare{name, exp} => {
                if self.variable_map.contains_key(name) {
                    return Err(GenError::InvalidVariableUsage(name.clone()));
                }

                self.variable_map.insert(name.clone(), self.stack_index);
                self.stack_index -= PLATFORM_WORD_SIZE;

                let code = match exp {
                    Some(exp) => {
                        let mut code = self.gen_expr(&exp)?;
                        code.push("push %rax".to_owned());
                        code
                    }
                    _ => vec!["push $0".to_owned()]
                };

                Ok(code)
            }
        }
    }

    fn gen_expr(&self, expr: &ast::Exp) -> Result<Vec<String>> {
        match expr {
            ast::Exp::Const(c) => Ok(self.gen_const(c)),
            ast::Exp::UnOp(op, exp) => self.gen_unop(op, exp),
            ast::Exp::BinOp(op, exp1, exp2) => self.gen_binop(op, exp1, exp2),
            ast::Exp::Assign(name, exp) => {
                let mut code = self.gen_expr(exp)?;
                
                let offset = self.variable_map.get(name).ok_or(GenError::InvalidVariableUsage(name.clone()))?;
                code.push(format!("mov %rax, {}(%rbp)", offset));

                Ok(code)
            }
            ast::Exp::Var(name) => {
                let offset = self.variable_map.get(name).ok_or(GenError::InvalidVariableUsage(name.clone()))?;
                Ok(vec![format!("mov {}(%rbp), %rax", offset)])
            }
        }
    }

    fn gen_const(&self, c: &ast::Const) -> Vec<String> {
        match c {
            ast::Const::Int(val) => vec![format!("mov    ${}, %rax", val)]
        }
    }

    fn gen_unop(&self, op: &ast::UnOp, exp: &ast::Exp) -> Result<Vec<String>> {
        let mut code = self.gen_expr(exp)?;
        code.extend(
            match op {
                ast::UnOp::Negation => {
                    vec!["neg    %rax".to_owned()]
                }
                ast::UnOp::LogicalNegation => {
                    vec![
                        "cmpl    $0, %eax".to_owned(),
                        "movl    $0, %eax".to_owned(),
                        "sete    %al".to_owned()
                    ]
                }
                ast::UnOp::BitwiseComplement => {
                    vec!["not    %eax".to_owned()]
                }
                ast::UnOp::Increment => {
                    vec!["inc    %eax".to_owned()]
                }
                ast::UnOp::Decrement => {
                    vec!["dec    %eax".to_owned()]
                }
            }
        );
        Ok(code)
    }

    fn gen_binop(&self, op: &ast::BinOp, exp1: &ast::Exp, exp2: &ast::Exp) -> Result<Vec<String>> {
        let exp1 = self.gen_expr(exp1)?;
        let exp2 = self.gen_expr(exp2)?;

        let code_with = |exp1: Vec<String>, exp2: Vec<String>, exp: &[String]| {
            let mut code = Vec::with_capacity(exp1.len() + exp2.len());
            code.extend(exp1);
            code.push("push %rax".to_owned());
            code.extend(exp2);
            code.push("pop %rcx".to_owned());
            code.extend_from_slice(exp);
            code
        };

        Ok(match op {
            ast::BinOp::BitwiseXor => {
                code_with(exp1, exp2, &["xor %rcx, %rax".to_owned()])
            },
            ast::BinOp::BitwiseOr => {
                code_with(exp1, exp2, &["or %rcx, %rax".to_owned()])
            },
            ast::BinOp::BitwiseAnd => {
                code_with(exp1, exp2, &["and %rcx, %rax".to_owned()])
            },
            ast::BinOp::Addition => {
                code_with(exp1, exp2, &["add %rcx, %rax".to_owned()])
            },
            ast::BinOp::Sub => {
                code_with(exp1, exp2, &[
                    "sub %rax, %rcx".to_owned(),
                    "mov %rcx, %rax".to_owned()
                ])
            },
            ast::BinOp::Multiplication => {
                code_with(exp1, exp2, &["imul %rcx, %rax".to_owned()])
            },
            ast::BinOp::Division => {
                code_with(exp1, exp2, &[
                    "mov %rax, %rbx".to_owned(),
                    "mov %rcx, %rax".to_owned(),
                    "mov %rbx, %rcx".to_owned(),
                    "cqo".to_owned(),
                    "idiv %rcx".to_owned()
                ])
            },
            ast::BinOp::Modulo => {
                code_with(exp1, exp2, &[
                    "mov %rax, %rbx".to_owned(),
                    "mov %rcx, %rax".to_owned(),
                    "mov %rbx, %rcx".to_owned(),
                    "cqo".to_owned(),
                    "idiv %rcx".to_owned(),
                    "mov %rdx, %rax".to_owned()
                ])
            },
            ast::BinOp::And => {
                let label = AsmFunc::unique_label("");
                let end_label = AsmFunc::unique_label("end");
                let mut code = Vec::new();
                code.extend(exp1);
                code.push("cmp    $0, %rax".to_owned());
                code.push(format!("jne    {}", label));
                code.push(format!("jmp    {}", end_label));
                code.push(format!("{}:", label));
                code.extend(exp2);
                code.push("cmp    $0, %rax".to_owned());
                code.push("mov    $0, %rax".to_owned());
                code.push("setne    %al".to_owned());
                code.push(format!("{}:", end_label));
                code
            },
            ast::BinOp::Or => {
                let label = AsmFunc::unique_label("");
                let end_label = AsmFunc::unique_label("end");
                let mut code = Vec::new();
                code.extend(exp1);
                code.push("cmp    $0, %rax".to_owned());
                code.push(format!("je    {}", label));
                code.push("mov    $1, %rax".to_owned());
                code.push(format!("jmp    {}", end_label));
                code.push(format!("{}:", label));
                code.extend(exp2);
                code.push("cmp    $0, %rax".to_owned());
                code.push("mov    $0, %rax".to_owned());
                code.push("setne    %al".to_owned());
                code.push(format!("{}:", end_label));
                code
            },
            ast::BinOp::Equal => {
                code_with(exp1, exp2, &[
                    "cmp    %rax, %rcx".to_owned(),
                    "mov    $0, %eax".to_owned(),
                    "sete    %al".to_owned()
                ])
            },
            ast::BinOp::NotEqual => {
                code_with(exp1, exp2, &[
                    "cmp    %rax, %rcx".to_owned(),
                    "mov    $0, %eax".to_owned(),
                    "setne    %al".to_owned()
                ])
            },
            ast::BinOp::LessThan => {
                code_with(exp1, exp2, &[
                    "cmp    %rax, %rcx".to_owned(),
                    "mov    $0, %eax".to_owned(),
                    "setl    %al".to_owned()
                ])
            },
            ast::BinOp::LessThanOrEqual => {
                code_with(exp1, exp2, &[
                    "cmp    %rax, %rcx".to_owned(),
                    "mov    $0, %eax".to_owned(),
                    "setle    %al".to_owned()
                ])
            },
            ast::BinOp::GreaterThan => {
                code_with(exp1, exp2, &[
                    "cmp    %rax, %rcx".to_owned(),
                    "mov    $0, %eax".to_owned(),
                    "setg    %al".to_owned()
                ])
            },
            ast::BinOp::GreaterThanOrEqual => {
                code_with(exp1, exp2, &[
                    "cmp    %rax, %rcx".to_owned(),
                    "mov    $0, %eax".to_owned(),
                    "setge    %al".to_owned()
                ])
            },
            ast::BinOp::BitwiseLeftShift => {
                code_with(exp1, exp2, &["sal %rcx, %rax".to_owned()])
            },
            ast::BinOp::BitwiseRightShift => {
                code_with(exp1, exp2, &["sar %rcx, %rax".to_owned()])
            },
        })
    }

    fn unique_label(prefix: &str) -> String {
        static mut LABEL_COUNTER: usize = 0;
        unsafe {
            LABEL_COUNTER += 1;
            if prefix.is_empty() {
                format!("_clause{}", LABEL_COUNTER)
            } else {
                format!("_{}{}", prefix, LABEL_COUNTER)
            }
        }
    }
}