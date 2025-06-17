use std::fs;
use std::io::{self, Write};
use std::path::Path;
use colored::*;
use crate::pak::{parse_pak_info, FileInfo, show_pak_info_simple};
use crate::utils::crypt_data;

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
                
                let parts: Vec<&str> = input.split_whitespace().collect();
                let command = parts[0];
                
                match command {
                    "help" | "h" => show_help(),
                    "exit" | "quit" | "q" => {
                        println!("再见！");
                        break;
                    },
                    "ls" | "dir" => {
                        let target_path = if parts.len() > 1 {
                            parts[1]
                        } else {
                            ""
                        };
                        
                        let (directories, files) = fs.get_entries_at_path(target_path);
                        
                        let dirs_empty = directories.is_empty();
                        let files_empty = files.is_empty();
                        
                        // 先显示目录
                        for dir in directories {
                            println!("{}", dir.cyan());
                        }
                        
                        // 再显示文件
                        for file in files {
                            let file_name = file.file_name.split('\\').last().unwrap_or(&file.file_name);
                            println!("{}", file_name.green());
                        }
                        
                        if dirs_empty && files_empty {
                            println!("目录为空");
                        }
                    },
                    "cd" => {
                        if parts.len() > 1 {
                            match fs.change_directory(parts[1]) {
                                Ok(_) => {},
                                Err(e) => println!("{}", format!("错误: {}", e).yellow()),
                            }
                        } else {
                            fs.current_path = "/".to_string();
                        }
                    },

                    "find" => {
                        if parts.len() == 1 {
                            // 直接find，列出当前路径下的所有文件（包括子目录）
                            find_all_files_in_path(&fs, &fs.current_path);
                        } else if parts.len() >= 3 && parts[1] == "-name" {
                            // find -name filename
                            find_by_name(&fs, parts[2]);
                        } else if parts.len() >= 3 && parts[1] == "-filter" {
                            // find -filter pattern
                            find_by_pattern(&fs, parts[2]);
                        } else {
                            println!("{}", "用法:".yellow());
                            println!("  find                    列出当前目录下所有文件");
                            println!("  find -name <filename>   查找指定文件名");
                            println!("  find -filter <pattern>  根据通配符查找文件");
                            println!("支持的通配符: * ? [abc] [a-z] [!abc]");
                        }
                    },
                    "info" => {
                        show_pak_info_simple(&data, encrypted, &fs.files);
                    },
                    _ => {
                        println!("{}", format!("未知命令: {}. 输入 'help' 查看可用命令", command).yellow());
                    }
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

/// 根据通配符模式查找文件（限制在当前路径下）
fn find_by_pattern(fs: &PakFileSystem, pattern: &str) {
    let current_prefix = if fs.current_path == "/" {
        ""
    } else {
        &fs.current_path[1..]
    };
    
    let mut found = Vec::new();
    
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
        
        if file_in_current_path && matches_glob_pattern(&file.file_name, pattern) {
            found.push(file);
        }
    }
    
    for file in found {
        println!("{}", file.file_name);
    }
}

/// 检查路径是否匹配通配符模式
fn matches_glob_pattern(path: &str, pattern: &str) -> bool {
    // 将Windows路径分隔符统一为Unix风格
    let normalized_path = path.replace('\\', "/");
    glob_match(&normalized_path, pattern)
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