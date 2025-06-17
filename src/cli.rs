use clap::{Parser, ColorChoice};
use std::path::PathBuf;

/// 获取自定义的clap样式
pub fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .header(clap::builder::styling::AnsiColor::Yellow.on_default().bold())
        .usage(clap::builder::styling::AnsiColor::Green.on_default().bold())
        .literal(clap::builder::styling::AnsiColor::Blue.on_default().bold())
        .placeholder(clap::builder::styling::AnsiColor::Cyan.on_default())
        .error(clap::builder::styling::AnsiColor::Red.on_default().bold())
        .valid(clap::builder::styling::AnsiColor::Green.on_default().bold())
        .invalid(clap::builder::styling::AnsiColor::Red.on_default().bold())
}

#[derive(Parser)]
#[command(
    author = "PVZ PAK Tool",
    version,
    about = "🌱 PVZ PAK文件操作工具 - 植物大战僵尸资源包管理器",
    long_about = "一个强大的Plants vs Zombies PAK文件操作工具，支持打包和解包操作。\n\n支持的操作：\n  • 解包 .pak 文件到目录\n  • 将目录打包为 .pak 文件\n  • 交互式文件浏览器（REPL模式）",
    color = ColorChoice::Auto,
    styles = get_styles()
)]
#[command(override_usage = "pkt.exe <INPUT> [--output <o>]")]
pub struct Cli {
    /// 输入文件或目录 (.pak文件将被解包，目录将被打包)
    #[arg(
        value_name = "INPUT",
        help = "输入文件或目录路径"
    )]
    pub input: PathBuf,
    
    /// 输出路径（可选，不提供时进入REPL模式）
    #[arg(
        short = 'o',
        long = "output",
        value_name = "OUTPUT",
        help = "输出路径（目录或.pak文件）"
    )]
    pub output: Option<PathBuf>,
} 