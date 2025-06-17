use clap::Parser;

// 导入库模块
use pvz_pak_tool::cli::Cli;
use pvz_pak_tool::{pack_to_pak, unpack_pak, run_repl, run_batch_commands};

#[cfg(windows)]
use colored::control;

fn main() {
    // 在Windows上启用终端颜色支持
    #[cfg(windows)]
    {
        if !control::set_virtual_terminal(true).is_ok() {
            // 如果无法启用虚拟终端，则禁用颜色
            control::set_override(false);
        }
    }
    
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
    } else if !cli.commands.is_empty() {
        // 有命令参数，执行批处理模式
        if cli.input.extension().map_or(false, |ext| ext == "pak") {
            run_batch_commands(&cli.input, &cli.commands)
        } else {
            eprintln!("错误: 批处理模式需要 .pak 文件作为输入");
            std::process::exit(1);
        }
    } else {
        // 没有输出路径也没有命令
        if cli.input.is_dir() {
            // 输入是目录但没有指定输出，要求指定输出PAK文件
            eprintln!("错误: 打包目录需要指定输出PAK文件");
            eprintln!("用法: pkt <目录> -o <输出.pak文件>");
            std::process::exit(1);
        } else if cli.input.extension().map_or(false, |ext| ext == "pak") {
            // 输入是PAK文件，进入REPL模式
            run_repl(&cli.input)
        } else {
            eprintln!("错误: 无法识别的输入类型");
            eprintln!("  - 打包: pkt <目录> -o <输出.pak文件>");
            eprintln!("  - 解包: pkt <输入.pak文件> -o <输出目录>");
            eprintln!("  - REPL: pkt <输入.pak文件>");
            eprintln!("  - 批处理: pkt <输入.pak文件> -c '命令1' -c '命令2'");
            std::process::exit(1);
        }
    };
    
    if let Err(e) = result {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }
}
