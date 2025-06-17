use std::fs;
use std::io::{self, Write};
use std::path::Path;
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
    
    /// 获取当前目录的内容
    pub fn get_current_entries(&self) -> (Vec<String>, Vec<&FileInfo>) {
        let mut directories = Vec::new();
        let mut files = Vec::new();
        
        for file in &self.files {
            if file.file_name.starts_with(&self.current_path[1..]) || self.current_path == "/" {
                let relative_path = if self.current_path == "/" {
                    file.file_name.as_str()
                } else {
                    &file.file_name[self.current_path.len() - 1..]
                };
                
                if let Some(slash_pos) = relative_path.find('/') {
                    let dir_name = &relative_path[..slash_pos];
                    if !directories.contains(&dir_name.to_string()) {
                        directories.push(dir_name.to_string());
                    }
                } else if !relative_path.is_empty() {
                    files.push(file);
                }
            }
        }
        
        directories.sort();
        (directories, files)
    }
    
    /// 切换目录
    pub fn change_directory(&mut self, path: &str) -> Result<(), String> {
        match path {
            ".." => {
                if self.current_path != "/" {
                    if let Some(pos) = self.current_path[..self.current_path.len()-1].rfind('/') {
                        self.current_path = self.current_path[..pos+1].to_string();
                    } else {
                        self.current_path = "/".to_string();
                    }
                }
                Ok(())
            },
            "/" => {
                self.current_path = "/".to_string();
                Ok(())
            },
            _ => {
                let new_path = if self.current_path == "/" {
                    format!("/{}/", path)
                } else {
                    format!("{}{}/", self.current_path, path)
                };
                
                // 检查目录是否存在
                let (directories, _) = self.get_current_entries();
                if directories.contains(&path.to_string()) {
                    self.current_path = new_path;
                    Ok(())
                } else {
                    Err(format!("目录不存在: {}", path))
                }
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
        println!("检测到加密，正在解密...");
        crypt_data(&mut data);
    }
    
    let (pak_info, _) = parse_pak_info(&data)?;
    
    println!();
    println!("PAK 文件信息:");
    show_pak_info_simple(&data, encrypted, &pak_info.file_info_library);
    println!();
    
    let mut fs = PakFileSystem::new(pak_info.file_info_library);
    
    println!("交互式PAK浏览器");
    println!("输入 'help' 查看可用命令");
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
                        let (directories, files) = fs.get_current_entries();
                        
                        let dirs_empty = directories.is_empty();
                        let files_empty = files.is_empty();
                        
                        if !dirs_empty {
                            println!("目录:");
                            for dir in directories {
                                println!("  📁 {}", dir);
                            }
                        }
                        
                        if !files_empty {
                            println!("文件:");
                            for file in files {
                                println!("  📄 {} ({} bytes)", file.file_name.split('/').last().unwrap_or(&file.file_name), file.z_size);
                            }
                        }
                        
                        if dirs_empty && files_empty {
                            println!("目录为空");
                        }
                    },
                    "cd" => {
                        if parts.len() > 1 {
                            match fs.change_directory(parts[1]) {
                                Ok(_) => {},
                                Err(e) => println!("错误: {}", e),
                            }
                        } else {
                            fs.current_path = "/".to_string();
                        }
                    },
                    "pwd" => {
                        println!("{}", fs.current_path);
                    },
                    "find" => {
                        if parts.len() > 1 {
                            find_files(&fs.files, parts[1]);
                        } else {
                            println!("用法: find <pattern>");
                        }
                    },
                    "info" => {
                        show_pak_info_simple(&data, encrypted, &fs.files);
                    },
                    _ => {
                        println!("未知命令: {}. 输入 'help' 查看可用命令", command);
                    }
                }
            },
            Err(e) => {
                println!("读取输入时出错: {}", e);
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
    println!("  help, h       显示此帮助信息");
    println!("  ls, dir       列出当前目录内容");
    println!("  cd <dir>      切换目录 (使用 .. 返回上级目录)");
    println!("  pwd           显示当前路径");
    println!("  find <name>   查找包含指定名称的文件");
    println!("  info          显示PAK文件信息");
    println!("  exit, quit, q 退出程序");
}

/// 查找文件
fn find_files(files: &[FileInfo], pattern: &str) {
    let pattern = pattern.to_lowercase();
    let mut found = Vec::new();
    
    for file in files {
        if file.file_name.to_lowercase().contains(&pattern) {
            found.push(file);
        }
    }
    
    if found.is_empty() {
        println!("未找到包含 '{}' 的文件", pattern);
    } else {
        println!("找到 {} 个匹配的文件:", found.len());
        for file in found {
            println!("  📄 {} ({} bytes)", file.file_name, file.z_size);
        }
    }
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