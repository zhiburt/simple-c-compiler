use crate::ast;
use std::collections::{HashMap, HashSet};

pub fn il(p: &ast::Program) -> Vec<FuncDef> {
    let mut gen = Generator::new();
    let mut funcs = Vec::new();
    for fun in &p.0 {
        if let Some(func) = gen.parse(fun) {
            funcs.push(func);
        }
    }

    funcs
}

struct Generator {
    // TODO: certainly not sure about contains this tuple
    // it has been done only for pretty_output purposes right now
    instructions: Vec<InstructionLine>,
    context: Context,
    counters: [usize; 3],
    allocated: usize,
}

#[derive(Debug)]
pub struct InstructionLine(pub Instruction, pub Option<ID>);

struct Context {
    /*
        NOTION: take away from ID as a dependency
    */
    symbols: HashMap<String, ID>,
    symbols_counter: usize,
    scopes: Vec<HashSet<String>>,
    loop_ctx: Vec<LoopContext>,
}

impl Context {
    fn new() -> Self {
        Context {
            symbols: HashMap::new(),
            symbols_counter: 0,
            scopes: vec![HashSet::new()],
            loop_ctx: Vec::new(),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn add_symbol(&mut self, name: &str) -> ID {
        if !self.add_symbol_to_scope(name) {
            /*
                TODO: Here should be raised a error since we have added the same variable to scope
                what is error

                it may be implemented as a feature, what means that we can pass here a config of polices to such type of behavior

                It's not handled anywhere above in the chain of compilation process
            */
            unimplemented!()
        }

        let id = ID::new(self.symbols_counter, IDType::Var);
        self.symbols.insert(name.to_owned(), id.clone());
        id
    }

    fn scope_symbol(&self, name: &str) -> Option<&ID> {
        let last_scope = self.scopes.last().unwrap();
        if last_scope.contains(name) {
            self.symbols.get(name)
        } else {
            None
        }
    }

    fn add_symbol_to_scope(&mut self, name: &str) -> bool {
        let last_scope = self.scopes.last_mut().unwrap();
        last_scope.insert(name.to_owned())
    }

    /*
        NOTION: could we store in context more useful information?
        e.g variables context

        on that regard we could develop a more convenient approach like

        let ctx = generator.push_ctx();
        ...

        do some stuff with context, and then it goes off the scope drop will be called
    */
    fn push_loop(&mut self, ctx: LoopContext) {
        self.loop_ctx.push(ctx);
    }

    fn pop_loop(&mut self) {
        self.loop_ctx.pop();
    }

    fn loop_end(&self) -> Label {
        self.loop_ctx.last().as_ref().unwrap().end
    }

    fn loop_start(&self) -> Label {
        self.loop_ctx.last().as_ref().unwrap().begin
    }
}

struct LoopContext {
    begin: Label,
    end: Label,
}

impl LoopContext {
    fn new(begin: Label, end: Label) -> Self {
        LoopContext { begin, end }
    }
}

impl Generator {
    pub fn new() -> Self {
        Generator {
            counters: [0, 0, 0],
            allocated: 0,
            instructions: Vec::new(),
            context: Context::new(),
        }
    }

    pub fn from(g: &Generator) -> Self {
        let mut generator = Generator::new();
        // check is it copy or clone in sense of references.
        generator.counters = g.counters;
        generator
    }

    pub fn parse(&mut self, func: &ast::FuncDecl) -> Option<FuncDef> {
        if func.blocks.is_none() {
            // here we should somehow show that this function can be called
            // with some type of parameters
            // it representation of declaration without definition
            //
            // unimplemented!()
            return None;
        }

        for p in func.parameters.iter() {
            /*
                TODO: investigate whatever it should increase alloc counter or not
            */
            self.alloc_var(&p);
        }

        let blocks = func.blocks.as_ref().unwrap();

        for block in blocks {
            self.emit_block(&block);
        }

        let vars = self
            .context
            .symbols
            .iter()
            .map(|(var, id)| (id.id, var.clone()))
            .collect::<HashMap<usize, String>>();

        self.context.symbols.clear();
        Some(FuncDef {
            name: func.name.clone(),
            frame_size: self.allocated_memory(),
            instructions: self.flush(),
            vars: vars,
        })
    }

    fn emit(&mut self, inst: Instruction) -> Option<ID> {
        let id = match &inst {
            Instruction::Op(..) => Some(self.alloc_tmp()),
            Instruction::Assignment(id, ..) => Some(id.clone()),
            Instruction::Alloc(..) => Some(self.alloc_tmp()),
            Instruction::Call(..) => {
                // TODO: we should handle somehow
                // the initial assignment to variable,
                // so might the best solution here is move call to Op type,
                // but not all calls has assignment pre operation
                //
                // It seems possible if we will have a small information about that in AST
                //
                // TODO: And what is the result unused?
                //
                // might it can be solved on some stage of optimization
                Some(self.alloc_tmp())
            }
            _ => None,
        };
        self.instructions.push(InstructionLine(inst, id.clone()));
        id
    }

    fn emit_expr(&mut self, exp: &ast::Exp) -> ID {
        match exp {
            ast::Exp::Var(name) => self.recognize_var(name),
            ast::Exp::Const(ast::Const::Int(val)) => {
                // TODO: might it should be changed since we whant to handle expresions like this
                // in this manner.
                //
                // x = 2 * a -> x := a * 2
                //
                // Without a temporary variable, but its deservers a major discussion
                self.emit(Instruction::Alloc(Const::Int(*val as i32)))
                    .unwrap()
            }
            ast::Exp::FuncCall(name, params) => {
                // Notion: it might be useful if we don't work with IDs itself here,
                // instead we could handle types which contains its size and id
                let ids = params.iter().map(|exp| self.emit_expr(exp)).collect();

                let types_size = params.len() * 4;

                self.emit(Instruction::Call(Call::new(&name, ids, types_size)))
                    .unwrap()
            }
            ast::Exp::UnOp(op, exp) => {
                let exp_id = self.emit_expr(exp);
                // TODO: looks like here the problem with additional tmp variable
                self.emit(Instruction::Op(Op::Unary(UnOp::from(op), exp_id)))
                    .unwrap()
            }
            ast::Exp::BinOp(op, exp1, exp2) => {
                let id1 = self.emit_expr(exp1);
                let id2 = self.emit_expr(exp2);
                self.emit(Instruction::Op(Op::Op(TypeOp::from(op), id1, id2)))
                    .unwrap()
            }
            ast::Exp::Assign(name, exp) => {
                let var_id = self.recognize_var(name);
                let exp_id = self.emit_expr(exp);
                self.emit(Instruction::Assignment(var_id, exp_id)).unwrap()
            }
            ast::Exp::CondExp(cond, exp1, exp2) => {
                /*
                    NOTION: if we will get a track with assign id an operator
                    it can be simplified by removing tmp_id
                */
                let end_label = self.uniq_label();
                let exp2_label = self.uniq_label();

                let tmp_id = self.alloc_tmp();

                let cond_id = self.emit_expr(cond);
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::IfGOTO(
                    cond_id, exp2_label,
                ))));
                let exp_id = self.emit_expr(exp1);
                self.emit(Instruction::Assignment(tmp_id.clone(), exp_id));
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::GOTO(
                    end_label,
                ))));
                self.emit(Instruction::ControlOp(ControlOp::Label(exp2_label)));
                let exp_id = self.emit_expr(exp2);
                self.emit(Instruction::Assignment(tmp_id.clone(), exp_id));
                self.emit(Instruction::ControlOp(ControlOp::Label(end_label)));

                tmp_id
            }
            _ => unimplemented!(),
        }
    }

    fn emit_decl(&mut self, decl: &ast::Declaration) {
        match decl {
            ast::Declaration::Declare { name, exp } => {
                let var_id = self.alloc_var(name);
                if let Some(exp) = exp {
                    let exp_id = self.emit_expr(exp);
                    self.emit(Instruction::Assignment(var_id, exp_id));
                }
            }
        }
    }

    fn emit_block(&mut self, block: &ast::BlockItem) {
        match block {
            ast::BlockItem::Declaration(decl) => self.emit_decl(decl),
            ast::BlockItem::Statement(st) => self.emit_statement(st),
        }
    }

    fn emit_statement(&mut self, st: &ast::Statement) {
        match st {
            ast::Statement::Exp { exp: exp } => {
                if let Some(exp) = exp {
                    self.emit_expr(exp);
                }
            }
            ast::Statement::Return { exp } => {
                let id = self.emit_expr(exp);
                self.emit(Instruction::ControlOp(ControlOp::Return(id)));
            }
            ast::Statement::Conditional {
                cond_expr,
                if_block,
                else_block,
            } => {
                let cond_id = self.emit_expr(cond_expr);
                let end_label = self.uniq_label();

                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::IfGOTO(
                    cond_id, end_label,
                ))));
                self.emit_statement(if_block);
                if let Some(else_block) = else_block {
                    let else_label = end_label;
                    let end_label = self.uniq_label();

                    self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::GOTO(
                        end_label,
                    ))));
                    self.emit(Instruction::ControlOp(ControlOp::Label(else_label)));
                    self.emit_statement(else_block);
                    self.emit(Instruction::ControlOp(ControlOp::Label(end_label)));
                } else {
                    self.emit(Instruction::ControlOp(ControlOp::Label(end_label)));
                }
            }
            ast::Statement::Compound { list: list } => {
                self.start_scope();

                if let Some(list) = list {
                    for block in list {
                        self.emit_block(block);
                    }
                }

                self.end_scope();
            }
            ast::Statement::While { exp, statement } => {
                let begin_label = self.uniq_label();
                let end_label = self.uniq_label();

                self.context
                    .push_loop(LoopContext::new(begin_label, end_label));

                self.emit(Instruction::ControlOp(ControlOp::Label(begin_label)));
                let cond_id = self.emit_expr(exp);
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::IfGOTO(
                    cond_id, end_label,
                ))));

                self.start_scope();
                self.emit_statement(statement);
                self.end_scope();

                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::GOTO(
                    begin_label,
                ))));
                self.emit(Instruction::ControlOp(ControlOp::Label(end_label)));

                self.context.pop_loop();
            }
            ast::Statement::Do { exp, statement } => {
                let begin_label = self.uniq_label();
                let end_label = self.uniq_label();

                self.context
                    .push_loop(LoopContext::new(begin_label, end_label));

                self.emit(Instruction::ControlOp(ControlOp::Label(begin_label)));

                self.start_scope();
                self.emit_statement(statement);
                self.end_scope();

                let cond_id = self.emit_expr(exp);
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::IfGOTO(
                    cond_id, end_label,
                ))));
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::GOTO(
                    begin_label,
                ))));
                self.emit(Instruction::ControlOp(ControlOp::Label(end_label)));

                self.context.pop_loop();
            }
            ast::Statement::ForDecl {
                decl,
                exp2,
                exp3,
                statement,
            } => {
                let begin_label = self.uniq_label();
                let end_label = self.uniq_label();

                self.context
                    .push_loop(LoopContext::new(begin_label, end_label));

                self.start_scope();
                self.emit_decl(decl);
                self.end_scope();

                self.emit(Instruction::ControlOp(ControlOp::Label(begin_label)));
                let cond_id = self.emit_expr(exp2);
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::IfGOTO(
                    cond_id, end_label,
                ))));

                self.start_scope();
                self.emit_statement(statement);
                self.end_scope();

                if let Some(exp3) = exp3 {
                    self.emit_expr(exp3);
                }
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::GOTO(
                    begin_label,
                ))));
                self.emit(Instruction::ControlOp(ControlOp::Label(end_label)));

                self.context.pop_loop();
            }
            ast::Statement::For {
                exp1,
                exp2,
                exp3,
                statement,
            } => {
                let begin_label = self.uniq_label();
                let end_label = self.uniq_label();

                self.context
                    .push_loop(LoopContext::new(begin_label, end_label));

                if let Some(exp) = exp1 {
                    self.emit_expr(exp);
                }
                self.emit(Instruction::ControlOp(ControlOp::Label(begin_label)));
                let cond_id = self.emit_expr(exp2);
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::IfGOTO(
                    cond_id, end_label,
                ))));

                self.start_scope();
                self.emit_statement(statement);
                self.end_scope();

                if let Some(exp3) = exp3 {
                    self.emit_expr(exp3);
                }
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::GOTO(
                    begin_label,
                ))));
                self.emit(Instruction::ControlOp(ControlOp::Label(end_label)));

                self.context.pop_loop();
            }
            ast::Statement::Break => {
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::GOTO(
                    self.context.loop_end(),
                ))));
            }
            ast::Statement::Continue => {
                self.emit(Instruction::ControlOp(ControlOp::Branch(Branch::GOTO(
                    self.context.loop_start(),
                ))));
            }
        }
    }

    // TODO: implement a a function which call something in scope
    fn start_scope(&mut self) {
        self.context.push_scope();
    }

    fn end_scope(&mut self) {
        self.context.pop_scope();
    }

    pub fn recognize_var(&mut self, name: &str) -> ID {
        self.context.scope_symbol(name).unwrap().clone()
    }

    pub fn allocated_memory(&self) -> BytesSize {
        self.allocated * 4
    }

    pub fn flush(&mut self) -> Vec<InstructionLine> {
        self.allocated = 0;
        let mut v = Vec::new();
        std::mem::swap(&mut self.instructions, &mut v);
        v
    }

    fn alloc_tmp(&mut self) -> ID {
        self.allocated += 1;
        ID::new(self.inc_tmp(), IDType::Temporary)
    }

    fn alloc_var(&mut self, name: &str) -> ID {
        self.allocated += 1;
        self.context.add_symbol(name)
    }

    fn inc_vars(&mut self) -> usize {
        self.counters[1] += 1;
        self.counters[1]
    }

    fn inc_tmp(&mut self) -> usize {
        let i = self.counters[0];
        self.counters[0] += 1;
        i
    }

    fn uniq_label(&mut self) -> Label {
        let l = self.counters[2];
        self.counters[2] += 1;
        l
    }

    fn id(&self, tp: IDType) -> ID {
        match tp {
            IDType::Temporary => ID {
                id: self.counters[0],
                tp,
            },
            IDType::Var => ID {
                id: self.counters[1],
                tp,
            },
        }
    }
}

#[derive(Debug)]
pub enum Instruction {
    // TODO: shake off this ID,
    // it represents assignment to a variable or a temporary one
    //
    // we would like to accomplish that since this operation is not represented by ID
    // it means that in the ID of this command will be the same as ID in parameter
    //
    // TODO: it does not support assignment of const, and constants now and Call Itself
    // the possible way is
    //
    // #[derive(Debug)]
    // enum Exp {
    //     Id(ID),
    //     Call(Call),
    //     Op(Op),
    // }
    Assignment(ID, ID),
    // Notion: Can alloc be responsible not only for tmp variables?
    Alloc(Const),
    Op(Op),
    Call(Call),
    ControlOp(ControlOp),
}

#[derive(Debug)]
enum Exp {
    Id(ID),
    Call(Call),
    Op(Op),
}

#[derive(Clone, Debug)]
pub struct ID {
    pub id: usize,
    pub tp: IDType,
}

impl ID {
    fn tmp() -> Self {
        ID {
            id: 0,
            tp: IDType::Temporary,
        }
    }

    fn new(id: usize, tp: IDType) -> Self {
        ID { id, tp }
    }
}

#[derive(Clone, Debug)]
pub enum IDType {
    Temporary,
    Var,
}

pub type Label = usize;

#[derive(Debug)]
pub enum Op {
    // TODO: it seems can be a Val
    Op(TypeOp, ID, ID),
    Unary(UnOp, ID),
}

#[derive(Debug)]
pub enum TypeOp {
    Arithmetic(ArithmeticOp),
    Relational(RelationalOp),
    Equality(EqualityOp),
    Bit(BitwiseOp),
}

impl TypeOp {
    fn from(op: &ast::BinOp) -> Self {
        match op {
            ast::BinOp::Addition => TypeOp::Arithmetic(ArithmeticOp::Add),
            ast::BinOp::Sub => TypeOp::Arithmetic(ArithmeticOp::Sub),
            ast::BinOp::Multiplication => TypeOp::Arithmetic(ArithmeticOp::Mul),
            ast::BinOp::Division => TypeOp::Arithmetic(ArithmeticOp::Div),
            ast::BinOp::Modulo => TypeOp::Arithmetic(ArithmeticOp::Mod),

            ast::BinOp::BitwiseAnd => TypeOp::Bit(BitwiseOp::And),
            ast::BinOp::BitwiseOr => TypeOp::Bit(BitwiseOp::Or),
            ast::BinOp::BitwiseXor => TypeOp::Bit(BitwiseOp::Xor),
            ast::BinOp::BitwiseLeftShift => TypeOp::Bit(BitwiseOp::LShift),
            ast::BinOp::BitwiseRightShift => TypeOp::Bit(BitwiseOp::RShift),

            ast::BinOp::Equal => TypeOp::Equality(EqualityOp::Equal),
            ast::BinOp::NotEqual => TypeOp::Equality(EqualityOp::NotEq),

            ast::BinOp::GreaterThan => TypeOp::Relational(RelationalOp::Greater),
            ast::BinOp::GreaterThanOrEqual => TypeOp::Relational(RelationalOp::GreaterOrEq),
            ast::BinOp::LessThan => TypeOp::Relational(RelationalOp::Less),
            ast::BinOp::LessThanOrEqual => TypeOp::Relational(RelationalOp::LessOrEq),

            ast::BinOp::And => unimplemented!(),
            ast::BinOp::Or => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub enum ControlOp {
    Label(Label),
    Branch(Branch),
    Return(ID),
}

type BytesSize = usize;

#[derive(Debug)]
pub enum Const {
    Int(i32),
}

#[derive(Debug)]
pub enum Val {
    Var(ID),
    Const(Const),
}

impl Val {
    fn to_var(self) -> Option<ID> {
        match self {
            Val::Var(id) => Some(id),
            _ => None,
        }
    }

    fn to_const(self) -> Option<Const> {
        match self {
            Val::Const(c) => Some(c),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug)]
pub enum BitwiseOp {
    And,
    Or,
    Xor,
    LShift,
    RShift,
}

#[derive(Debug)]
pub enum UnOp {
    Neg,
    BitComplement,
    LogicNeg,
}

impl UnOp {
    fn from(op: &ast::UnOp) -> Self {
        match op {
            ast::UnOp::Negation => UnOp::Neg,
            ast::UnOp::BitwiseComplement => UnOp::BitComplement,
            ast::UnOp::LogicalNegation => UnOp::LogicNeg,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum RelationalOp {
    Less,
    LessOrEq,
    Greater,
    GreaterOrEq,
}

#[derive(Debug)]
pub enum EqualityOp {
    Equal,
    NotEq,
}

#[derive(Debug)]
pub enum Branch {
    GOTO(Label),
    // might here can be Val?
    IfGOTO(ID, Label),
}

#[derive(Debug)]
pub struct Call {
    pub name: String,
    pub params: Vec<ID>,
    pub pop_size: BytesSize,
    pub tp: FnType,
}

impl Call {
    fn new(name: &str, params: Vec<ID>, params_size: BytesSize) -> Self {
        Call {
            name: name.to_owned(),
            tp: FnType::LCall,
            params,
            pop_size: params_size,
        }
    }
}

#[derive(Debug)]
pub enum FnType {
    LCall,
}

#[derive(Debug)]
pub struct FuncDef {
    pub name: String,
    pub frame_size: BytesSize,
    pub vars: HashMap<usize, String>,
    pub instructions: Vec<InstructionLine>,
}
