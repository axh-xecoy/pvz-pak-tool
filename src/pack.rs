use std::fs::{self, File};
use std::io::{self, Write, BufWriter};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use crate::pak::{FileInfo, PakInfo};
use crate::utils::{write_string_by_u8_head, crypt_data};

/// 收集目录中的所有文件
pub fn collect_files(dir: &Path, base_dir: &Path) -> io::Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();
    let entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    
    // 不排序，使用文件系统原始顺序
    for entry in entries {
        let path = entry.path();
        let relative_path = path.strip_prefix(base_dir)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let relative_str = relative_path.to_string_lossy().replace('/', "\\"); // 使用Windows风格路径
        
        if path.is_file() {
            files.push((relative_str, path));
        } else if path.is_dir() {
            // 递归处理子目录
            let mut sub_files = collect_files(&path, base_dir)?;
            files.append(&mut sub_files);
        }
    }
    
    Ok(files)
}

/// 将目录打包为PAK文件
pub fn pack_to_pak(input_dir: &Path, output_path: &Path) -> io::Result<()> {
    // 验证输入目录
    if !input_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("输入目录不存在: {}", input_dir.display())
        ));
    }
    
    if !input_dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "输入路径必须是目录"
        ));
    }
    
    // 验证输出文件
    if let Some(ext) = output_path.extension() {
        if ext != "pak" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "输出文件必须是 .pak 文件"
            ));
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "输出文件必须是 .pak 文件"
        ));
    }
    
    // 检查输出文件是否已存在
    if output_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("输出文件已存在: {}", output_path.display())
        ));
    }
    
    println!("正在打包: {}", input_dir.display());
    println!("输出文件: {}", output_path.display());
    
    // 收集所有文件
    let files = collect_files(input_dir, input_dir)?;
    
    if files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "输入目录为空"
        ));
    }
    
    println!("找到 {} 个文件", files.len());
    
    // 检查文件名重复
    let mut file_names = HashSet::new();
    for (relative_path, _) in &files {
        if !file_names.insert(relative_path.clone()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("发现重复的文件名: {}", relative_path)
            ));
        }
    }
    
    // 构建文件信息
    let mut file_infos = Vec::new();
    for (relative_path, file_path) in &files {
        let metadata = fs::metadata(file_path)?;
        let file_size = metadata.len();
        
        if file_size > u32::MAX as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("文件过大 (>4GB): {}", relative_path)
            ));
        }
        
        file_infos.push(FileInfo {
            file_name: relative_path.clone(),
            z_size: file_size as u32,
            _size: file_size as u32,
            _file_time: PakInfo::DEFAULT_FILE_TIME,
        });
    }
    
    // 创建PAK文件
    let mut pak_info = PakInfo::new();
    pak_info.file_info_library = file_infos;
    pak_info.compress = Some(false); // 不压缩模式
    
    let output_file = File::create(output_path)?;
    let mut writer = BufWriter::new(output_file);
    
    // 写入头部
    writer.write_all(&PakInfo::MAGIC.to_le_bytes())?;
    writer.write_all(&PakInfo::VERSION.to_le_bytes())?;
    
    // 写入文件信息
    for file_info in &pak_info.file_info_library {
        writer.write_all(&[0u8])?; // flag
        write_string_by_u8_head(&mut writer, &file_info.file_name)?;
        writer.write_all(&file_info.z_size.to_le_bytes())?;
        
        // 如果启用压缩，写入原始大小
        if pak_info.compress.unwrap_or(false) {
            writer.write_all(&file_info._size.to_le_bytes())?;
        }
        
        // 总是写入文件时间戳
        writer.write_all(&file_info._file_time.to_le_bytes())?;
    }
    
    // 写入结束标志
    writer.write_all(&[PakInfo::INFO_END])?;
    
    // 写入文件数据
    for (index, (_, file_path)) in files.iter().enumerate() {
        if index % 100 == 0 {
            println!("正在打包: {}/{}", index + 1, files.len());
        }
        
        let file_data = fs::read(file_path)?;
        writer.write_all(&file_data)?;
    }
    
    // 刷新缓冲区
    writer.flush()?;
    drop(writer);
    
    // 加密整个文件
    println!("正在加密PAK文件...");
    let mut pak_data = fs::read(output_path)?;
    crypt_data(&mut pak_data);
    fs::write(output_path, pak_data)?;
    
    println!("打包完成！生成了包含 {} 个文件的PAK", pak_info.file_info_library.len());
    
    // 显示文件大小
    let output_size = fs::metadata(output_path)?.len();
    println!("输出文件大小: {:.2} MB", output_size as f64 / 1024.0 / 1024.0);
    
    Ok(())
} 