use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use crate::pak::{parse_pak_info, show_pak_info_simple};
use crate::utils::{ensure_directory_exists, is_directory_empty, crypt_data};

/// 解包PAK文件到指定目录
pub fn unpack_pak(input_path: &Path, output_dir: &Path) -> io::Result<()> {
    // 验证输入文件
    if !input_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("输入文件不存在: {}", input_path.display())
        ));
    }
    
    if !input_path.extension().map_or(false, |ext| ext == "pak") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "输入文件必须是 .pak 文件"
        ));
    }
    
    // 验证输出目录
    if output_dir.exists() && !is_directory_empty(output_dir)? {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("输出目录不为空: {}", output_dir.display())
        ));
    }
    
    // 创建输出目录
    fs::create_dir_all(output_dir)?;
    
    println!("正在解包: {}", input_path.display());
    println!("输出目录: {}", output_dir.display());
    
    // 读取PAK文件
    let mut data = fs::read(input_path)?;
    
    // 检测是否加密
    let encrypted = detect_encryption(&data);
    
    if encrypted {
        println!("检测到加密，正在解密...");
        crypt_data(&mut data);
    }
    
    // 解析PAK信息
    let (pak_info, header_size) = parse_pak_info(&data)?;
    
    println!("PAK 文件信息:");
    show_pak_info_simple(&data, encrypted, &pak_info.file_info_library);
    println!();
    
    // 提取文件
    let mut file_offset = header_size;
    for (index, file_info) in pak_info.file_info_library.iter().enumerate() {
        if index % 100 == 0 {
            println!("正在解包: {}/{}", index + 1, pak_info.file_info_library.len());
        }
        
        // 检查数据边界
        if file_offset + file_info.z_size as usize > data.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("文件 {} 数据超出PAK文件边界", file_info.file_name)
            ));
        }
        
        // 读取文件数据
        let file_data = &data[file_offset..file_offset + file_info.z_size as usize];
        
        // 创建输出文件路径
        let output_file_path = output_dir.join(&file_info.file_name);
        ensure_directory_exists(&output_file_path)?;
        
        // 写入文件
        let mut output_file = File::create(&output_file_path)?;
        output_file.write_all(file_data)?;
        
        file_offset += file_info.z_size as usize;
    }
    
    println!("解包完成！提取了 {} 个文件", pak_info.file_info_library.len());
    Ok(())
}

/// 检测PAK文件是否加密
fn detect_encryption(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    
    // 检查magic number
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    
    // PC版加密后的magic是0x4D37BD37 (1295498551)
    // 解密后应该是0xBAC04AC0
    magic == 0x4D37BD37
} 