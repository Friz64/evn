use shaderc::CompileOptions;
use shaderc::Compiler;
use shaderc::Error;
use shaderc::OptimizationLevel;
use shaderc::ShaderKind;
use std::fs;
use std::io::Write;
use clap::{App, Arg};

fn main() {
    let matches = App::new("evn shader compiler")
        .arg(Arg::with_name("input")
            .short("i")
            .value_name("PATH")
            .required(true)
            .help("Source code directory (Required)")
        )
        .arg(Arg::with_name("output")
            .short("o")
            .value_name("PATH")
            .required(true)
            .help("Output directory (Required)")
        )
        .get_matches();
    
    let input = matches.value_of("input").unwrap();
    let output = matches.value_of("output").unwrap();

    // just to make sure
    let _ = fs::create_dir_all(&output);

    let mut compiler = Compiler::new().unwrap();
    let mut comp_options = CompileOptions::new().unwrap();
    comp_options.set_optimization_level(OptimizationLevel::Performance);

    for file in fs::read_dir(input).unwrap().filter_map(|file| file.ok()) {
        if file.file_type().unwrap().is_dir() {
            continue;
        }

        let file_name = file.file_name().into_string().unwrap();

        let shader_type = if file_name.ends_with("vert") {
            ShaderKind::Vertex
        } else if file_name.ends_with("frag") {
            ShaderKind::Fragment
        } else {
            continue;
        };

        let source_text = fs::read_to_string(file.path()).unwrap();

        println!("Compiling {}...", file_name);

        let result = compiler.compile_into_spirv(
            &source_text,
            shader_type,
            &file_name,
            "main",
            Some(&comp_options),
        );

        match result {
            Ok(result) => {
                let num_warnings = result.get_num_warnings();

                if num_warnings == 0 {
                    println!("Compilation finished\n");
                } else {
                    println!(
                        "Compilation finished with {} warnings:\n{}",
                        num_warnings,
                        result.get_warning_messages(),
                    );
                }

                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(format!("{}/{}.spv", output, file_name))
                    .unwrap();

                file.write(result.as_binary_u8()).unwrap();
            }
            Err(err) => match err {
                Error::CompilationError(num, error) => {
                    println!("Compilation failed with {} errors:\n{}", num, error)
                }
                Error::InternalError(error) => println!("Internal Error:\n{}", error),
                Error::InvalidStage(error) => println!("Invalid Stage:\n{}", error),
                Error::InvalidAssembly(error) => println!("Invalid Assembly:\n{}", error),
                Error::NullResultObject(error) => println!("Null Result Object:\n{}", error),
            },
        };
    }
}
