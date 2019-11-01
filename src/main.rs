use std::io::Write;

use simple_c_compiler::{gen, Lexer, Program};

mod pretty_output;

fn main() {
    let file = std::env::args().collect::<Vec<String>>()[1].clone();
    let program = std::fs::File::open(file).unwrap();
    let lexer = Lexer::new();
    let mut tokens = lexer.lex(program);
    let program = Program::parse(&mut tokens).expect("Cannot parse program");
    println!("{}", pretty_output::pretty_program(&program));
    let mut asm_file = std::fs::File::create("assembly.s").expect("Cannot create assembler code");
    asm_file.write_all(gen(program, "main").as_ref()).unwrap();
}
