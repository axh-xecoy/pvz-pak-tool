use std::io;
use crate::utils::{read_string_by_u8_head, read_u32_le, read_u64_le};

/// PAK文件中的文件信息
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub file_name: String,
    pub z_size: u32,
    pub _size: u32,      // 原始大小（压缩模式下使用，但当前实现不需要）
    pub _file_time: u64, // 文件时间戳（从PAK读取但不使用）
}

/// PAK文件信息
#[derive(Debug)]
pub struct PakInfo {
    pub _magic: u32,     // PAK文件魔数（验证后不再使用）
    pub version: u32,
    pub file_info_library: Vec<FileInfo>,
    pub compress: Option<bool>,
    pub pc: bool,
    pub win: bool,
}

impl PakInfo {
    pub const MAGIC: u32 = 0xBAC04AC0; // -1161803072 in signed
    pub const VERSION: u32 = 0x0;
    pub const INFO_END: u8 = 0x80;
    pub const DEFAULT_FILE_TIME: u64 = 129146222018596744;
    
    pub fn new() -> Self {
        Self {
            _magic: Self::MAGIC,
            version: Self::VERSION,
            file_info_library: Vec::new(),
            compress: None,
            pc: true,
            win: true,
        }
    }
}

/// 解析PAK文件头
pub fn parse_pak_info(data: &[u8]) -> io::Result<(PakInfo, usize)> {
    let mut pos = 0;
    let mut pak_info = PakInfo::new();
    
    // 读取并验证magic
    let magic = read_u32_le(data, &mut pos)?;
    if magic != PakInfo::MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid PAK magic: expected 0x{:08X}, got 0x{:08X}", PakInfo::MAGIC, magic)
        ));
    }
    
    // 读取版本
    let version = read_u32_le(data, &mut pos)?;
    pak_info.version = version;
    
    // 读取文件条目
    loop {
        if pos >= data.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Unexpected end of data"));
        }
        
        let flag = data[pos];
        pos += 1;
        
        if flag == PakInfo::INFO_END {
            break;
        } else if flag != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData, 
                format!("Invalid file flag: 0x{:02X} at position {} (expected 0x00 or 0x80)", flag, pos - 1)
            ));
        }
        
        // 自动检测压缩模式（仅在第一个文件时）
        if pak_info.compress.is_none() {
            let saved_pos = pos;
            
            // 跳过文件名
            if pos >= data.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Cannot read filename length"));
            }
            let name_len = data[pos] as usize;
            pos += 1 + name_len;
            
            // 跳过z_size
            pos += 4;
            
            // 检测是否有size字段（压缩模式）
            if pos + 12 < data.len() {
                pos += 4; // 假设有size字段
                pos += 8; // 跳过timestamp
                
                // 检查下一个标志位
                if pos < data.len() && (data[pos] == 0 || data[pos] == PakInfo::INFO_END) {
                    pak_info.compress = Some(true);
                } else {
                    pak_info.compress = Some(false);
                }
            } else {
                pak_info.compress = Some(false);
            }
            
            pos = saved_pos; // 恢复位置
        }
        
        // 读取文件信息
        let file_name = read_string_by_u8_head(data, &mut pos)?;
        let z_size = read_u32_le(data, &mut pos)?;
        
        let size = if pak_info.compress.unwrap_or(false) {
            read_u32_le(data, &mut pos)?
        } else {
            0
        };
        
        let file_time = read_u64_le(data, &mut pos)?;
        
        pak_info.file_info_library.push(FileInfo {
            file_name,
            z_size,
            _size: size,
            _file_time: file_time,
        });
    }
    
    Ok((pak_info, pos))
}

/// 显示PAK文件简要信息
pub fn show_pak_info_simple(data: &[u8], is_encrypted: bool, files: &[FileInfo]) {
    println!("  PAK 文件大小: {:.2} MB", data.len() as f64 / 1024.0 / 1024.0);
    println!("  加密状态: {}", if is_encrypted { "已加密" } else { "未加密" });
    println!("  文件数量: {}", files.len());
} 