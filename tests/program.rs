mod compare_gcc {
    use super::*;

    mod statements {
        use super::*;

        #[test]
        fn if_statement() {
            compare_expr(r"
                int a = 10;
                if (a > 0) return 10;
                else
                    return 20;
            ");

            compare_expr(r"
                int a = 10;
                if (a > 0) return 10;
                return 20;
            ");

            compare_expr(r"
                int a = 10;
                if (a > 0) return 10;
            ");

            
            compare_expr(r"
                int a = 10;
                else a = 1;
                return a;
            ");
        }
    }
}

fn compare_expr(expr: &str) {
    assert_eq!(compile_gcc_expr(expr), compile_expr(expr));
}

fn compile_expr(expr: &str) -> usize {
    use std::io::Write;

    let code = format!("int main(){{ {} }}", expr);

    let code_file = random_name("code_", ".c");
    let mut file = std::fs::File::create(&code_file).unwrap();
    file.write_all(code.as_bytes()).unwrap();

    let asm_file = random_name("asm_", ".s");
    let bin_file = random_name("bin_", ".out");


    let compiler = std::process::Command::new("./target/debug/simple-c-compiler")
        .arg(&code_file)
        .arg(&asm_file)
        .output()
        .expect("start compilation process");
    if !compiler.status.success() {
        println!("{:?}", code_file);
        panic!();
    }

    let gcc = std::process::Command::new("gcc")
        .args(&["-m64", "-o", &bin_file, &asm_file])
        .output()
        .expect("Run gcc to compile asm")
        .status;

    if !gcc.success() {
        println!("{:?}", std::env::current_dir());
        panic!();
    }

    let program = std::process::Command::new(format!("./{}", bin_file))
        .output()
        .expect("Run compiled programm")
        .status;

    std::fs::remove_file(code_file).unwrap();
    std::fs::remove_file(asm_file).unwrap();
    std::fs::remove_file(bin_file).unwrap();

    program.code().unwrap() as usize
}

fn compile_gcc_expr(expr: &str) -> usize {
    use std::io::Write;

    let code = format!("int main(){{ {} }}", expr);

    let code_file = random_name("code_", ".c");
    let mut file = std::fs::File::create(&code_file).unwrap();
    file.write_all(code.as_bytes()).unwrap();

    let bin_file = random_name("bin_", ".out");

    let gcc = std::process::Command::new("gcc")
        .args(&["-m64", "-o", &bin_file, &code_file])
        .output()
        .expect("Run gcc to compile asm")
        .status;

    if !gcc.success() {
        println!("{:?}", std::env::current_dir());
        panic!();
    }

    let program = std::process::Command::new(format!("./{}", bin_file))
        .output()
        .expect("Run compiled programm")
        .status;

    std::fs::remove_file(code_file).unwrap();
    std::fs::remove_file(bin_file).unwrap();

    program.code().unwrap() as usize
}

fn random_name(prefix: &str, suffix: &str) -> String {
    lazy_static::lazy_static!{
        static ref INDEX: std::sync::Mutex<usize> = std::sync::Mutex::new(0);
    }
    let mut i = INDEX.lock().unwrap();
    *i += 1;

    format!("{}{}{}", prefix, i, suffix)
}