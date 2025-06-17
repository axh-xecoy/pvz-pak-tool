use clap::Parser;

// 导入库模块
use pvz_pak_tool::cli::Cli;
use pvz_pak_tool::{pack_to_pak, unpack_pak, run_repl};

fn main() {
    let cli = Cli::parse();
    
    let result = if let Some(output) = &cli.output {
        // 有输出路径，执行打包或解包操作
        if cli.input.is_dir() {
            // 输入是目录，执行打包
            pack_to_pak(&cli.input, output)
        } else if cli.input.extension().map_or(false, |ext| ext == "pak") {
            // 输入是PAK文件，执行解包
            unpack_pak(&cli.input, output)
        } else {
            eprintln!("错误: 无法识别的输入类型");
            eprintln!("  - 打包: 输入应为目录");
            eprintln!("  - 解包: 输入应为 .pak 文件");
            std::process::exit(1);
        }
    } else {
        // 没有输出路径，进入REPL模式
        if cli.input.extension().map_or(false, |ext| ext == "pak") {
            run_repl(&cli.input)
        } else {
            eprintln!("错误: REPL模式需要 .pak 文件作为输入");
            std::process::exit(1);
        }
    };
    
    if let Err(e) = result {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }
}
