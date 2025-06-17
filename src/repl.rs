use std::fs;
use std::io::{self, Write};
use std::path::Path;
use colored::*;
use crate::pak::{parse_pak_info, FileInfo, show_pak_info_simple};
use crate::utils::crypt_data;

/// 输出重定向目标
enum OutputTarget {
    Stdout,
    File(String),
}

/// 命令输出缓冲区
struct OutputBuffer {
    lines: Vec<String>,
}

impl OutputBuffer {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }
    
    fn writeln(&mut self, line: String) {
        self.lines.push(line);
    }
    
    fn flush_to(&self, target: &OutputTarget) -> io::Result<()> {
        match target {
            OutputTarget::Stdout => {
                for line in &self.lines {
                    println!("{}", line);
                }
            }
            OutputTarget::File(filename) => {
                let mut file = fs::File::create(filename)?;
                for line in &self.lines {
                    writeln!(file, "{}", line)?;
                }
            }
        }
        Ok(())
    }
}

/// 格式化文件信息
fn format_file_info(file: &FileInfo, format_str: Option<&str>) -> String {
    let default_format = "$path";
    let format = format_str.unwrap_or(default_format);
    
    // 提取文件信息
    let full_path = &file.file_name;
    let file_name = full_path.split('\\').last().unwrap_or(full_path);
    let dir_path = if let Some(pos) = full_path.rfind('\\') {
        &full_path[..pos]
    } else {
        ""
    };
    
    // 替换格式变量
    format
        .replace("$path", full_path)
        .replace("$name", file_name)
        .replace("$dir", dir_path)
        .replace("$size", &file.z_size.to_string())
        .replace("$osize", &file._size.to_string())
}

/// 格式化目录信息
fn format_dir_info(dir_path: &str, format_str: Option<&str>) -> String {
    let default_format = "$path";
    let format = format_str.unwrap_or(default_format);
    
    let dir_name = dir_path.split('\\').last().unwrap_or(dir_path);
    let parent_path = if let Some(pos) = dir_path.rfind('\\') {
        &dir_path[..pos]
    } else {
        ""
    };
    
    // 替换格式变量（目录没有大小信息）
    format
        .replace("$path", dir_path)
        .replace("$name", dir_name)
        .replace("$dir", parent_path)
        .replace("$size", "<DIR>")
        .replace("$osize", "<DIR>")
}

/// 解析命令行，提取命令和重定向信息
fn parse_command_line(input: &str) -> (String, OutputTarget) {
    if let Some(redirect_pos) = input.find(" > ") {
        let command = input[..redirect_pos].trim().to_string();
        let filename = input[redirect_pos + 3..].trim().to_string();
        (command, OutputTarget::File(filename))
    } else {
        (input.trim().to_string(), OutputTarget::Stdout)
    }
}

/// 解析命令参数，支持引号
fn parse_command_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' => {
                if in_quotes {
                    current_arg.push(ch);
                } else if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
            }
            _ => {
                current_arg.push(ch);
            }
        }
    }
    
    if !current_arg.is_empty() {
        args.push(current_arg);
    }
    
    args
}

/// PAK文件系统（用于REPL模式）
pub struct PakFileSystem {
    files: Vec<FileInfo>,
    current_path: String,
}

impl PakFileSystem {
    pub fn new(files: Vec<FileInfo>) -> Self {
        Self {
            files,
            current_path: "/".to_string(),
        }
    }
    
    /// 解析路径，支持相对路径、绝对路径等
    fn resolve_path(&self, path: &str) -> String {
        let path = path.trim();
        
        if path.is_empty() {
            return self.current_path.clone();
        }
        
        // 处理绝对路径
        if path.starts_with('/') {
            let normalized = self.normalize_path(path);
            return if normalized.is_empty() { "/".to_string() } else { normalized };
        }
        
        // 处理相对路径
        let mut result_parts = if self.current_path == "/" {
            Vec::new()
        } else {
            self.current_path[1..].split('/').map(|s| s.to_string()).collect()
        };
        
        // 处理路径中的各个部分
        for part in path.split('/') {
            let part = part.trim();
            if part.is_empty() || part == "." {
                continue;
            } else if part == ".." {
                if !result_parts.is_empty() {
                    result_parts.pop();
                }
            } else {
                result_parts.push(part.to_string());
            }
        }
        
        if result_parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", result_parts.join("/"))
        }
    }
    
    /// 标准化路径，去掉多余的斜杠和..等
    fn normalize_path(&self, path: &str) -> String {
        let mut parts = Vec::new();
        
        for part in path.split('/') {
            let part = part.trim();
            if part.is_empty() || part == "." {
                continue;
            } else if part == ".." {
                if !parts.is_empty() {
                    parts.pop();
                }
            } else {
                parts.push(part);
            }
        }
        
        if parts.is_empty() {
            String::new()
        } else {
            format!("/{}", parts.join("/"))
        }
    }
    
    /// 获取指定路径的目录内容
    pub fn get_entries_at_path(&self, target_path: &str) -> (Vec<String>, Vec<&FileInfo>) {
        let resolved_path = self.resolve_path(target_path);
        
        let mut directories = Vec::new();
        let mut files = Vec::new();
        
        // 构建当前目录的前缀
        let current_prefix = if resolved_path == "/" {
            ""
        } else {
            &resolved_path[1..]
        };
        
        for file in &self.files {
            // 检查文件是否在当前目录下
            let file_path = &file.file_name;
            
            if current_prefix.is_empty() {
                // 根目录：处理所有文件
                if let Some(slash_pos) = file_path.find('\\') {
                    // 这是一个子目录中的文件
                    let dir_name = &file_path[..slash_pos];
                    if !directories.contains(&dir_name.to_string()) {
                        directories.push(dir_name.to_string());
                    }
                } else {
                    // 这是根目录中的文件
                    files.push(file);
                }
            } else {
                // 非根目录：只处理当前目录下的文件
                // 将前缀中的正斜杠转换为反斜杠以匹配文件路径格式
                let normalized_prefix = current_prefix.replace('/', "\\");
                // 检查文件路径是否以当前前缀开头
                if file_path.starts_with(&normalized_prefix) {
                    let remaining = &file_path[normalized_prefix.len()..];
                    
                    // 如果剩余路径以 \ 开头，去掉它
                    let remaining = if remaining.starts_with('\\') {
                        &remaining[1..]
                    } else {
                        remaining
                    };
                    
                    if let Some(slash_pos) = remaining.find('\\') {
                        // 这是一个子目录中的文件
                        let dir_name = &remaining[..slash_pos];
                        if !dir_name.is_empty() && !directories.contains(&dir_name.to_string()) {
                            directories.push(dir_name.to_string());
                        }
                    } else if !remaining.is_empty() {
                        // 这是当前目录中的文件
                        files.push(file);
                    }
                }
            }
        }
        
        // 按字母顺序排序，不区分大小写
        directories.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        
        // 将文件按名称排序，不区分大小写
        let mut sorted_files: Vec<&FileInfo> = files;
        sorted_files.sort_by(|a, b| {
            let name_a = a.file_name.split('\\').last().unwrap_or(&a.file_name).to_lowercase();
            let name_b = b.file_name.split('\\').last().unwrap_or(&b.file_name).to_lowercase();
            name_a.cmp(&name_b)
        });
        
        (directories, sorted_files)
    }
    
    /// 获取当前目录的内容
    pub fn get_current_entries(&self) -> (Vec<String>, Vec<&FileInfo>) {
        self.get_entries_at_path("")
    }
    
    /// 切换目录
    pub fn change_directory(&mut self, path: &str) -> Result<(), String> {
        let target_path = self.resolve_path(path);
        
        // 检查目标路径是否存在
        let (_directories, _) = self.get_entries_at_path(&target_path);
        
        // 如果目标路径是根目录，或者目标路径的父目录包含目标目录名，则路径有效
        if target_path == "/" {
            self.current_path = "/".to_string();
            Ok(())
        } else {
            // 检查目标路径是否存在
            let parent_path = if let Some(pos) = target_path.rfind('/') {
                if pos == 0 {
                    "/"
                } else {
                    &target_path[..pos]
                }
            } else {
                "/"
            };
            
            let dir_name = if let Some(pos) = target_path.rfind('/') {
                &target_path[pos + 1..]
            } else {
                &target_path
            };
            
            let (parent_dirs, _) = self.get_entries_at_path(parent_path);
            
            if parent_dirs.contains(&dir_name.to_string()) {
                self.current_path = target_path;
                Ok(())
            } else {
                Err(format!("目录不存在: {}", path))
            }
        }
    }
}

/// 运行交互式REPL模式
pub fn run_repl(pak_path: &Path) -> io::Result<()> {
    println!("进入交互模式...");
    println!("正在加载PAK文件: {}", pak_path.display());
    
    // 读取和解析PAK文件
    let mut data = fs::read(pak_path)?;
    
    // 检测是否加密
    let encrypted = detect_encryption(&data);
    if encrypted {
        crypt_data(&mut data);
    }
    
    let (pak_info, _) = parse_pak_info(&data)?;
    
    println!();
    println!("PAK 文件信息:");
    show_pak_info_simple(&data, encrypted, &pak_info.file_info_library);
    println!();
    
    let mut fs = PakFileSystem::new(pak_info.file_info_library);
    
    println!("交互式PAK浏览器");
    println!("输入 'help' 查看可用命令，'exit' 退出程序");
    println!();
    
    loop {
        print!("PAK:{} > ", fs.current_path);
        io::stdout().flush()?;
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                
                if input.is_empty() {
                    continue;
                }
                
                // 解析命令和重定向
                let (command_line, output_target) = parse_command_line(input);
                let parts = parse_command_args(&command_line);
                let command = parts.get(0).map(|s| s.as_str()).unwrap_or("");
                
                // 创建输出缓冲区
                let mut output = OutputBuffer::new();
                
                let result: io::Result<()> = match command {
                    "help" | "h" => {
                        show_help_to_buffer(&mut output);
                        Ok(())
                    },
                    "exit" | "quit" | "q" => {
                        println!("再见！");
                        break;
                    },
                    "ls" | "dir" => {
                        let target_path = if parts.len() > 1 {
                            &parts[1]
                        } else {
                            ""
                        };
                        
                        list_directory_to_buffer(&fs, target_path, &mut output);
                        Ok(())
                    },
                    "cd" => {
                        if parts.len() > 1 {
                            match fs.change_directory(&parts[1]) {
                                Ok(_) => Ok(()),
                                Err(e) => {
                                    output.writeln(format!("错误: {}", e));
                                    Ok(())
                                }
                            }
                        } else {
                            fs.current_path = "/".to_string();
                            Ok(())
                        }
                    },
                    "find" => {
                        let mut format_str = None;
                        let mut search_type = None;
                        let mut search_value = None;
                        let mut show_help = false;
                        let mut parse_error = false;
                        
                        // 解析find命令参数
                        let mut i = 1;
                        while i < parts.len() {
                            match parts[i].as_str() {
                                "-help" | "--help" => {
                                    show_help = true;
                                    break;
                                },
                                "-name" => {
                                    if i + 1 < parts.len() {
                                        search_type = Some("name");
                                        search_value = Some(&parts[i + 1]);
                                        i += 2;
                                    } else {
                                        output.writeln(format!("{}", "错误: -name 需要指定文件名".red()));
                                        parse_error = true;
                                        break;
                                    }
                                },
                                "-filter" => {
                                    if i + 1 < parts.len() {
                                        search_type = Some("filter");
                                        search_value = Some(&parts[i + 1]);
                                        i += 2;
                                    } else {
                                        output.writeln(format!("{}", "错误: -filter 需要指定模式".red()));
                                        parse_error = true;
                                        break;
                                    }
                                },
                                "-format" => {
                                    if i + 1 < parts.len() {
                                        format_str = Some(&parts[i + 1]);
                                        i += 2;
                                    } else {
                                        output.writeln(format!("{}", "错误: -format 需要指定格式字符串".red()));
                                        parse_error = true;
                                        break;
                                    }
                                },
                                _ => {
                                    output.writeln(format!("{}", format!("未知参数: {}", &parts[i]).red()));
                                    parse_error = true;
                                    break;
                                }
                            }
                        }
                        
                        // 根据解析结果执行相应操作
                        if show_help {
                            show_find_help(&mut output);
                        } else if parse_error {
                            // 参数解析错误，错误信息已经输出
                        } else {
                            // 执行find命令
                            match search_type {
                                Some("name") => {
                                    if let Some(filename) = search_value {
                                        find_by_name_to_buffer_with_format(&fs, filename.as_str(), format_str.map(|s| s.as_str()), &mut output);
                                    }
                                },
                                Some("filter") => {
                                    if let Some(pattern) = search_value {
                                        find_by_pattern_to_buffer_with_format(&fs, pattern.as_str(), format_str.map(|s| s.as_str()), &mut output);
                                    }
                                },
                                None => {
                                    // 没有搜索条件，列出当前目录所有文件
                                    find_all_files_in_path_to_buffer_with_format(&fs, &fs.current_path, format_str.map(|s| s.as_str()), &mut output);
                                },
                                _ => {
                                    output.writeln("用法:".to_string());
                                    output.writeln("  find [-format \"格式\"]                    列出当前目录下所有文件".to_string());
                                    output.writeln("  find -name <filename> [-format \"格式\"]   查找指定文件名".to_string());
                                    output.writeln("  find -filter <pattern> [-format \"格式\"]  根据通配符查找文件".to_string());
                                    output.writeln("支持的通配符: * ? [abc] [a-z] [!abc]".to_string());
                                    output.writeln("格式变量:".to_string());
                                    output.writeln("  $path   - 文件完整路径".to_string());
                                    output.writeln("  $name   - 文件名（不含路径）".to_string());
                                    output.writeln("  $dir    - 目录路径".to_string());
                                    output.writeln("  $size   - 文件大小（压缩后）".to_string());
                                    output.writeln("  $osize  - 原始文件大小".to_string());
                                    output.writeln("示例: find -format \"$path -- $size bytes\"".to_string());
                                }
                            }
                        }
                        Ok(())
                    },
                    "info" => {
                        show_pak_info_to_buffer(&data, encrypted, &fs.files, &mut output);
                        Ok(())
                    },
                    _ => {
                        output.writeln(format!("{}", format!("未知命令: {}. 输入 'help' 查看可用命令", command).red()));
                        Ok(())
                    }
                };
                
                // 输出结果
                if let Err(e) = result {
                    println!("执行命令时出错: {}", e);
                } else if let Err(e) = output.flush_to(&output_target) {
                    println!("输出重定向时出错: {}", e);
                }
            },
            Err(e) => {
                println!("{}", format!("读取输入时出错: {}", e).yellow());
                break;
            }
        }
        println!();
    }
    
    Ok(())
}

/// 显示帮助信息
fn show_help() {
    println!("可用命令:");
    println!("  help, h                  显示此帮助信息");
    println!("  ls [path]                列出目录内容 (支持相对/绝对路径)");
    println!("  cd <path>                切换目录 (支持 .., ./, ../, /abs/path, rel/path)");
    println!("  find                     列出当前目录下所有文件");
    println!("  find -name <filename>    查找指定文件名");
    println!("  find -filter <pattern>   根据通配符查找文件");
    println!("    支持通配符: * ? [abc] [a-z] [!abc]");
    println!("    示例: find -filter /compiled/* 或 find -filter *.jpg");
    println!("  info                     显示PAK文件信息");
    println!("  exit, quit, q            退出程序");
    println!("  [command] > file.txt     重定向输出到文件");
}

/// 显示帮助信息到缓冲区
fn show_help_to_buffer(output: &mut OutputBuffer) {
    output.writeln(format!("{}", "可用命令:".bright_cyan().bold()));
    output.writeln(format!("  {}                  显示此帮助信息", "help, h".bright_green()));
    output.writeln(format!("  {}                列出目录内容 (支持相对/绝对路径)", "ls [path]".bright_green()));
    output.writeln(format!("  {}                切换目录 (支持 .., ./, ../, /abs/path, rel/path)", "cd <path>".bright_green()));
    output.writeln(format!("  {}                     列出当前目录下所有文件", "find".bright_green()));
    output.writeln(format!("  {}               显示find命令详细帮助", "find -help".bright_green()));
    output.writeln(format!("  {}    查找指定文件名", "find -name <filename>".bright_green()));
    output.writeln(format!("  {}   根据通配符查找文件", "find -filter <pattern>".bright_green()));
    output.writeln(format!("    支持通配符: {}", "* ? [abc] [a-z] [!abc]".yellow()));
    output.writeln(format!("    示例: {} 或 {}", "find -filter /compiled/*".yellow(), "find -filter *.jpg".yellow()));
    output.writeln(format!("  {}                     显示PAK文件信息", "info".bright_green()));
    output.writeln(format!("  {}            退出程序", "exit, quit, q".bright_green()));
    output.writeln(format!("  {}     重定向输出到文件", "[command] > file.txt".yellow()));
}

/// 显示find命令详细帮助信息
fn show_find_help(output: &mut OutputBuffer) {
    output.writeln(format!("{}", "FIND - 文件查找命令".bright_cyan().bold()));
    output.writeln("".to_string());
    output.writeln(format!("{}", "用法:".bright_cyan()));
    output.writeln(format!("  {}", "find [选项]".bright_white()));
    output.writeln("".to_string());
    output.writeln(format!("{}", "选项:".bright_cyan()));
    output.writeln(format!("  {}            显示此帮助信息", "-help, --help".bright_green()));
    output.writeln(format!("  {}           按确切文件名查找", "-name <文件名>".bright_green()));
    output.writeln(format!("  {}           按通配符模式查找", "-filter <模式>".bright_green()));
    output.writeln(format!("  {}     自定义输出格式", "-format <格式字符串>".bright_green()));
    output.writeln("".to_string());
    output.writeln(format!("{}", "通配符:".bright_cyan()));
    output.writeln(format!("  {}              匹配任意数量的字符", "*".yellow()));
    output.writeln(format!("  {}              匹配单个字符", "?".yellow()));
    output.writeln(format!("  {}          匹配方括号中的任意一个字符", "[abc]".yellow()));
    output.writeln(format!("  {}          匹配指定范围内的字符", "[a-z]".yellow()));
    output.writeln(format!("  {}         匹配不在方括号中的字符", "[!abc]".yellow()));
    output.writeln("".to_string());
    output.writeln(format!("{}", "格式变量:".bright_cyan()));
    output.writeln(format!("  {}          文件的完整路径", "$path".magenta()));
    output.writeln(format!("  {}          文件名（不含路径）", "$name".magenta()));
    output.writeln(format!("  {}           文件所在目录路径", "$dir".magenta()));
    output.writeln(format!("  {}          文件大小（压缩后，字节）", "$size".magenta()));
    output.writeln(format!("  {}         原始文件大小（字节）", "$osize".magenta()));
    output.writeln("".to_string());
    output.writeln(format!("{}", "使用示例:".bright_cyan()));
    output.writeln("".to_string());
    output.writeln(format!("{}", "1. 基本查找:".bright_white()));
    output.writeln(format!("   {}                              # 列出当前目录所有文件", "find".yellow()));
    output.writeln(format!("   {}                # 查找名为app.jpg的文件", "find -name app.jpg".yellow()));
    output.writeln(format!("   {}                # 查找所有xml文件", "find -filter *.xml".yellow()));
    output.writeln(format!("   {}          # 查找compiled目录下所有文件", "find -filter /compiled/*".yellow()));
    output.writeln(format!("   {}      # 查找data目录下以数字开头的txt文件", "find -filter data/[0-9]*.txt".yellow()));
    output.writeln("".to_string());
    output.writeln(format!("{}", "2. 自定义格式输出:".bright_white()));
    output.writeln(format!("   {}               # 默认格式，显示完整路径", "find -format \"$path\"".yellow()));
    output.writeln(format!("   {}               # 仅显示文件名", "find -format \"$name\"".yellow()));
    output.writeln(format!("   {} # 显示路径和大小", "find -format \"$path -- $size bytes\"".yellow()));
    output.writeln(format!("   {}      # 显示文件名和原始大小", "find -format \"$name ($osize)\"".yellow()));
    output.writeln(format!("   {}          # 显示目录/文件名格式", "find -format \"$dir/$name\"".yellow()));
    output.writeln("".to_string());
    output.writeln(format!("{}", "3. 组合使用:".bright_white()));
    output.writeln(format!("   {}", "find -name \"*.jpg\" -format \"$name in $dir - $size bytes\"".yellow()));
    output.writeln(format!("   {}", "find -filter \"config*\" -format \"$path,$size,$osize\"".yellow()));
    output.writeln("".to_string());
    output.writeln(format!("{}", "4. 输出重定向:".bright_white()));
    output.writeln(format!("   {}", "find -format \"$path,$size,$osize\" > files.csv".yellow()));
    output.writeln(format!("   {}", "find -filter \"*.xml\" > xml_files.txt".yellow()));
    output.writeln("".to_string());
    output.writeln(format!("{}", "注意:".bright_cyan()));
    output.writeln(format!("- 路径分隔符统一使用正斜杠 {} 进行搜索", "/".yellow()));
    output.writeln(format!("- 绝对路径以 {} 开头，相对路径基于当前目录", "/".yellow()));
    output.writeln(format!("- 目录项的 {} 和 {} 显示为 {}", "$size".magenta(), "$osize".magenta(), "<DIR>".yellow()));
    output.writeln(format!("- 所有输出都可以通过 {} 重定向到文件", "> filename".yellow()));
}

/// 列出目录内容到缓冲区
fn list_directory_to_buffer(fs: &PakFileSystem, target_path: &str, output: &mut OutputBuffer) {
    let (directories, files) = fs.get_entries_at_path(target_path);
    
    let dirs_empty = directories.is_empty();
    let files_empty = files.is_empty();
    
    // 先显示目录
    for dir in directories {
        output.writeln(format!("{}", dir.cyan()));
    }
    
    // 再显示文件
    for file in files {
        let file_name = file.file_name.split('\\').last().unwrap_or(&file.file_name);
        output.writeln(format!("{}", file_name.bright_white()));
    }
    
    if dirs_empty && files_empty {
        output.writeln(format!("{}", "目录为空".yellow()));
    }
}

/// 列出指定路径下的所有文件（包括子目录）
fn find_all_files_in_path(fs: &PakFileSystem, base_path: &str) {
    let resolved_path = fs.resolve_path(base_path);
    let prefix = if resolved_path == "/" {
        ""
    } else {
        &resolved_path[1..]
    };
    
    let mut found_files = Vec::new();
    
    for file in &fs.files {
        let file_path = &file.file_name;
        
        if prefix.is_empty() {
            // 根目录，包含所有文件
            found_files.push(file);
        } else {
            // 检查文件是否在指定路径下
            let normalized_prefix = prefix.replace('/', "\\");
            if file_path.starts_with(&normalized_prefix) {
                let remaining = &file_path[normalized_prefix.len()..];
                if remaining.starts_with('\\') || remaining.is_empty() {
                    found_files.push(file);
                }
            }
        }
    }
    
    for file in found_files {
        println!("{}", file.file_name);
    }
}

/// 列出指定路径下的所有文件（包括子目录）到缓冲区
fn find_all_files_in_path_to_buffer(fs: &PakFileSystem, base_path: &str, output: &mut OutputBuffer) {
    let resolved_path = fs.resolve_path(base_path);
    let prefix = if resolved_path == "/" {
        ""
    } else {
        &resolved_path[1..]
    };
    
    let mut found_files = Vec::new();
    
    for file in &fs.files {
        let file_path = &file.file_name;
        
        if prefix.is_empty() {
            // 根目录，包含所有文件
            found_files.push(file);
        } else {
            // 检查文件是否在指定路径下
            let normalized_prefix = prefix.replace('/', "\\");
            if file_path.starts_with(&normalized_prefix) {
                let remaining = &file_path[normalized_prefix.len()..];
                if remaining.starts_with('\\') || remaining.is_empty() {
                    found_files.push(file);
                }
            }
        }
    }
    
    for file in found_files {
        output.writeln(file.file_name.clone());
    }
}

/// 列出指定路径下的所有文件（包括子目录）到缓冲区（带格式化）
fn find_all_files_in_path_to_buffer_with_format(fs: &PakFileSystem, base_path: &str, format_str: Option<&str>, output: &mut OutputBuffer) {
    let resolved_path = fs.resolve_path(base_path);
    let prefix = if resolved_path == "/" {
        ""
    } else {
        &resolved_path[1..]
    };
    
    let mut found_files = Vec::new();
    
    for file in &fs.files {
        let file_path = &file.file_name;
        
        if prefix.is_empty() {
            // 根目录，包含所有文件
            found_files.push(file);
        } else {
            // 检查文件是否在指定路径下
            let normalized_prefix = prefix.replace('/', "\\");
            if file_path.starts_with(&normalized_prefix) {
                let remaining = &file_path[normalized_prefix.len()..];
                if remaining.starts_with('\\') || remaining.is_empty() {
                    found_files.push(file);
                }
            }
        }
    }
    
    for file in found_files {
        let formatted = format_file_info(file, format_str);
        output.writeln(formatted);
    }
}

/// 根据文件名查找文件和目录（限制在当前路径下）
fn find_by_name(fs: &PakFileSystem, filename: &str) {
    let current_prefix = if fs.current_path == "/" {
        ""
    } else {
        &fs.current_path[1..]
    };
    
    let mut found_files = Vec::new();
    let mut found_dirs = std::collections::HashSet::new();
    
    // 查找文件
    for file in &fs.files {
        let file_path = &file.file_name;
        
        // 检查文件是否在当前目录下
        let file_in_current_path = if current_prefix.is_empty() {
            true // 根目录，包含所有文件
        } else {
            let normalized_prefix = current_prefix.replace('/', "\\");
            file_path.starts_with(&normalized_prefix) && 
            (file_path.len() == normalized_prefix.len() || 
             file_path.chars().nth(normalized_prefix.len()) == Some('\\'))
        };
        
        if file_in_current_path {
                         // 检查文件名
             let relative_path = if current_prefix.is_empty() {
                 file_path.as_str()
             } else {
                 let normalized_prefix = current_prefix.replace('/', "\\");
                 let remaining = &file_path[normalized_prefix.len()..];
                 remaining.strip_prefix('\\').unwrap_or(remaining)
             };
            
            let file_basename = relative_path.split('\\').last().unwrap_or(relative_path);
            if file_basename == filename {
                found_files.push(file);
            }
            
            // 查找目录名（仅在相对路径中）
            let path_parts: Vec<&str> = relative_path.split('\\').collect();
            for (i, part) in path_parts.iter().enumerate() {
                if *part == filename {
                    // 构建完整目录路径
                    let relative_dir_path = path_parts[0..=i].join("\\");
                    let full_dir_path = if current_prefix.is_empty() {
                        relative_dir_path
                    } else {
                        format!("{}\\{}", current_prefix.replace('/', "\\"), relative_dir_path)
                    };
                    found_dirs.insert(full_dir_path);
                }
            }
        }
    }
    
    // 先显示目录
    let mut sorted_dirs: Vec<String> = found_dirs.into_iter().collect();
    sorted_dirs.sort();
    for dir in sorted_dirs {
        println!("{}", dir);
    }
    
    // 再显示文件
    for file in found_files {
        println!("{}", file.file_name);
    }
}

/// 根据文件名查找文件和目录（限制在当前路径下）到缓冲区
fn find_by_name_to_buffer(fs: &PakFileSystem, filename: &str, output: &mut OutputBuffer) {
    let current_prefix = if fs.current_path == "/" {
        ""
    } else {
        &fs.current_path[1..]
    };
    
    let mut found_files = Vec::new();
    let mut found_dirs = std::collections::HashSet::new();
    
    // 查找文件
    for file in &fs.files {
        let file_path = &file.file_name;
        
        // 检查文件是否在当前目录下
        let file_in_current_path = if current_prefix.is_empty() {
            true // 根目录，包含所有文件
        } else {
            let normalized_prefix = current_prefix.replace('/', "\\");
            file_path.starts_with(&normalized_prefix) && 
            (file_path.len() == normalized_prefix.len() || 
             file_path.chars().nth(normalized_prefix.len()) == Some('\\'))
        };
        
        if file_in_current_path {
            // 检查文件名
            let relative_path = if current_prefix.is_empty() {
                file_path.as_str()
            } else {
                let normalized_prefix = current_prefix.replace('/', "\\");
                let remaining = &file_path[normalized_prefix.len()..];
                remaining.strip_prefix('\\').unwrap_or(remaining)
            };
            
            let file_basename = relative_path.split('\\').last().unwrap_or(relative_path);
            if file_basename == filename {
                found_files.push(file);
            }
            
            // 查找目录名（仅在相对路径中）
            let path_parts: Vec<&str> = relative_path.split('\\').collect();
            for (i, part) in path_parts.iter().enumerate() {
                if *part == filename {
                    // 构建完整目录路径
                    let relative_dir_path = path_parts[0..=i].join("\\");
                    let full_dir_path = if current_prefix.is_empty() {
                        relative_dir_path
                    } else {
                        format!("{}\\{}", current_prefix.replace('/', "\\"), relative_dir_path)
                    };
                    found_dirs.insert(full_dir_path);
                }
            }
        }
    }
    
    // 先显示目录
    let mut sorted_dirs: Vec<String> = found_dirs.into_iter().collect();
    sorted_dirs.sort();
    for dir in sorted_dirs {
        output.writeln(dir);
    }
    
    // 再显示文件
    for file in found_files {
        output.writeln(file.file_name.clone());
    }
}

/// 根据文件名查找文件和目录（限制在当前路径下）到缓冲区（带格式化）
fn find_by_name_to_buffer_with_format(fs: &PakFileSystem, filename: &str, format_str: Option<&str>, output: &mut OutputBuffer) {
    let current_prefix = if fs.current_path == "/" {
        ""
    } else {
        &fs.current_path[1..]
    };
    
    let mut found_files = Vec::new();
    let mut found_dirs = std::collections::HashSet::new();
    
    // 查找文件
    for file in &fs.files {
        let file_path = &file.file_name;
        
        // 检查文件是否在当前目录下
        let file_in_current_path = if current_prefix.is_empty() {
            true // 根目录，包含所有文件
        } else {
            let normalized_prefix = current_prefix.replace('/', "\\");
            file_path.starts_with(&normalized_prefix) && 
            (file_path.len() == normalized_prefix.len() || 
             file_path.chars().nth(normalized_prefix.len()) == Some('\\'))
        };
        
        if file_in_current_path {
            // 检查文件名
            let relative_path = if current_prefix.is_empty() {
                file_path.as_str()
            } else {
                let normalized_prefix = current_prefix.replace('/', "\\");
                let remaining = &file_path[normalized_prefix.len()..];
                remaining.strip_prefix('\\').unwrap_or(remaining)
            };
            
            let file_basename = relative_path.split('\\').last().unwrap_or(relative_path);
            if file_basename == filename {
                found_files.push(file);
            }
            
            // 查找目录名（仅在相对路径中）
            let path_parts: Vec<&str> = relative_path.split('\\').collect();
            for (i, part) in path_parts.iter().enumerate() {
                if *part == filename {
                    // 构建完整目录路径
                    let relative_dir_path = path_parts[0..=i].join("\\");
                    let full_dir_path = if current_prefix.is_empty() {
                        relative_dir_path
                    } else {
                        format!("{}\\{}", current_prefix.replace('/', "\\"), relative_dir_path)
                    };
                    found_dirs.insert(full_dir_path);
                }
            }
        }
    }
    
    // 先显示目录
    let mut sorted_dirs: Vec<String> = found_dirs.into_iter().collect();
    sorted_dirs.sort();
    for dir in sorted_dirs {
        let formatted = format_dir_info(&dir, format_str);
        output.writeln(formatted);
    }
    
    // 再显示文件
    for file in found_files {
        let formatted = format_file_info(file, format_str);
        output.writeln(formatted);
    }
}

/// 根据通配符模式查找文件
fn find_by_pattern(fs: &PakFileSystem, pattern: &str) {
    let mut found = Vec::new();
    
    // 如果模式以/开头，从根目录搜索；否则基于当前路径搜索
    let search_pattern = if pattern.starts_with('/') {
        // 移除开头的/，因为PAK文件路径不以/开头
        pattern[1..].to_string()
    } else {
        // 相对路径，添加当前路径前缀
        if fs.current_path == "/" {
            pattern.to_string()
        } else {
            format!("{}/{}", &fs.current_path[1..], pattern)
        }
    };
    
    // 将模式中的/转换为\以匹配PAK文件路径格式
    let normalized_pattern = search_pattern.replace('/', "\\");
    
    for file in &fs.files {
        if matches_glob_pattern(&file.file_name, &normalized_pattern) {
            found.push(file);
        }
    }
    
    for file in found {
        println!("{}", file.file_name);
    }
}

/// 根据通配符模式查找文件到缓冲区
fn find_by_pattern_to_buffer(fs: &PakFileSystem, pattern: &str, output: &mut OutputBuffer) {
    let mut found = Vec::new();
    
    // 如果模式以/开头，从根目录搜索；否则基于当前路径搜索
    let search_pattern = if pattern.starts_with('/') {
        // 移除开头的/，因为PAK文件路径不以/开头
        pattern[1..].to_string()
    } else {
        // 相对路径，添加当前路径前缀
        if fs.current_path == "/" {
            pattern.to_string()
        } else {
            format!("{}/{}", &fs.current_path[1..], pattern)
        }
    };
    
    // 将模式中的/转换为\以匹配PAK文件路径格式
    let normalized_pattern = search_pattern.replace('/', "\\");
    
    for file in &fs.files {
        if matches_glob_pattern(&file.file_name, &normalized_pattern) {
            found.push(file);
        }
    }
    
    for file in found {
        output.writeln(file.file_name.clone());
    }
}

/// 根据通配符模式查找文件到缓冲区（带格式化）
fn find_by_pattern_to_buffer_with_format(fs: &PakFileSystem, pattern: &str, format_str: Option<&str>, output: &mut OutputBuffer) {
    let mut found = Vec::new();
    
    // 如果模式以/开头，从根目录搜索；否则基于当前路径搜索
    let search_pattern = if pattern.starts_with('/') {
        // 移除开头的/，因为PAK文件路径不以/开头
        pattern[1..].to_string()
    } else {
        // 相对路径，添加当前路径前缀
        if fs.current_path == "/" {
            pattern.to_string()
        } else {
            format!("{}/{}", &fs.current_path[1..], pattern)
        }
    };
    
    // 将模式中的/转换为\以匹配PAK文件路径格式
    let normalized_pattern = search_pattern.replace('/', "\\");
    
    for file in &fs.files {
        if matches_glob_pattern(&file.file_name, &normalized_pattern) {
            found.push(file);
        }
    }
    
    for file in found {
        let formatted = format_file_info(file, format_str);
        output.writeln(formatted);
    }
}

/// 显示PAK文件信息到缓冲区
fn show_pak_info_to_buffer(data: &[u8], _encrypted: bool, files: &[FileInfo], output: &mut OutputBuffer) {
    output.writeln(format!("{}: {}", "PAK 文件大小".bright_cyan(), format!("{:.2} MB", data.len() as f64 / 1024.0 / 1024.0).bright_white()));
    output.writeln(format!("{}: {}", "文件数量".bright_cyan(), format!("{}", files.len()).bright_white()));
    
    let total_compressed: u32 = files.iter().map(|f| f.z_size).sum();
    let total_uncompressed: u32 = files.iter().map(|f| f._size).sum();
    
    output.writeln(format!("{}: {}", "压缩总大小".bright_cyan(), format!("{} bytes", total_compressed).bright_white()));
    
    if total_uncompressed > 0 {
        output.writeln(format!("{}: {}", "原始总大小".bright_cyan(), format!("{} bytes", total_uncompressed).bright_white()));
        let ratio = (total_compressed as f64 / total_uncompressed as f64) * 100.0;
        output.writeln(format!("{}: {}", "压缩率".bright_cyan(), format!("{:.1}%", ratio).bright_green()));
    }
}

/// 检查路径是否匹配通配符模式
fn matches_glob_pattern(path: &str, pattern: &str) -> bool {
    // 直接匹配，不进行路径分隔符转换，因为现在pattern已经是反斜杠格式
    glob_match(path, pattern)
}

/// 实现基本的glob匹配
fn glob_match(text: &str, pattern: &str) -> bool {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();
    
    glob_match_recursive(&text_chars, &pattern_chars, 0, 0)
}

fn glob_match_recursive(text: &[char], pattern: &[char], t_idx: usize, p_idx: usize) -> bool {
    // 模式结束
    if p_idx >= pattern.len() {
        return t_idx >= text.len();
    }
    
    // 文本结束但模式未结束
    if t_idx >= text.len() {
        // 检查剩余模式是否都是*
        return pattern[p_idx..].iter().all(|&c| c == '*');
    }
    
    match pattern[p_idx] {
        '*' => {
            // *匹配0个或多个字符
            // 尝试匹配0个字符
            if glob_match_recursive(text, pattern, t_idx, p_idx + 1) {
                return true;
            }
            // 尝试匹配1个或多个字符
            for i in t_idx..text.len() {
                if glob_match_recursive(text, pattern, i + 1, p_idx + 1) {
                    return true;
                }
            }
            false
        }
        '?' => {
            // ?匹配单个字符
            glob_match_recursive(text, pattern, t_idx + 1, p_idx + 1)
        }
        '[' => {
            // 字符类匹配
            if let Some(end_bracket) = pattern[p_idx..].iter().position(|&c| c == ']') {
                let char_class = &pattern[p_idx + 1..p_idx + end_bracket];
                let current_char = text[t_idx];
                
                if matches_char_class(current_char, char_class) {
                    glob_match_recursive(text, pattern, t_idx + 1, p_idx + end_bracket + 1)
                } else {
                    false
                }
            } else {
                // 没有找到闭合的]，按字面量匹配
                text[t_idx] == pattern[p_idx] && 
                glob_match_recursive(text, pattern, t_idx + 1, p_idx + 1)
            }
        }
        c => {
            // 字面量字符匹配
            text[t_idx] == c && glob_match_recursive(text, pattern, t_idx + 1, p_idx + 1)
        }
    }
}

fn matches_char_class(ch: char, char_class: &[char]) -> bool {
    if char_class.is_empty() {
        return false;
    }
    
    let negated = char_class[0] == '!';
    let chars_to_check = if negated { &char_class[1..] } else { char_class };
    
    let mut i = 0;
    let mut matched = false;
    
    while i < chars_to_check.len() {
        if i + 2 < chars_to_check.len() && chars_to_check[i + 1] == '-' {
            // 范围匹配如 a-z
            let start = chars_to_check[i];
            let end = chars_to_check[i + 2];
            if ch >= start && ch <= end {
                matched = true;
                break;
            }
            i += 3;
        } else {
            // 单个字符匹配
            if ch == chars_to_check[i] {
                matched = true;
                break;
            }
            i += 1;
        }
    }
    
    if negated { !matched } else { matched }
}

/// 检测PAK文件是否加密
fn detect_encryption(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    
    // 检查magic number（PC版本应该是0xBAC04AC0）
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    magic != 0xBAC04AC0
} 