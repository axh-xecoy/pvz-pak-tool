use std::fs;
use std::io::{self, Write};
use std::path::Path;
use encoding_rs::GBK;

/// 确保目录存在（创建父目录）
pub fn ensure_directory_exists(file_path: &Path) -> io::Result<()> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// 检查目录是否为空
pub fn is_directory_empty(dir: &Path) -> io::Result<bool> {
    if !dir.exists() {
        return Ok(true);
    }
    
    let mut entries = fs::read_dir(dir)?;
    Ok(entries.next().is_none())
}

/// 读取字符串（长度前缀为u8）
pub fn read_string_by_u8_head(data: &[u8], pos: &mut usize) -> io::Result<String> {
    if *pos >= data.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Unexpected end of data"));
    }
    
    let length = data[*pos] as usize;
    *pos += 1;
    
    if *pos + length > data.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "String length exceeds data"));
    }
    
    let string_bytes = &data[*pos..*pos + length];
    *pos += length;
    
    // 使用GBK (ANSI) 编码解析，与PopStudio保持一致
    let (decoded, _, _) = GBK.decode(string_bytes);
    Ok(decoded.to_string())
}

/// 写入字符串（长度前缀为u8）
pub fn write_string_by_u8_head(writer: &mut dyn Write, s: &str) -> io::Result<()> {
    // 使用GBK (ANSI) 编码，与PopStudio保持一致
    let (encoded, _, _) = GBK.encode(s);
    let bytes = encoded.as_ref();
    
    if bytes.len() > 255 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("String too long: {} bytes (max 255)", bytes.len())
        ));
    }
    
    writer.write_all(&[bytes.len() as u8])?;
    writer.write_all(bytes)?;
    Ok(())
}

/// 读取小端序u32
pub fn read_u32_le(data: &[u8], pos: &mut usize) -> io::Result<u32> {
    if *pos + 4 > data.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Cannot read u32"));
    }
    
    let value = u32::from_le_bytes([
        data[*pos],
        data[*pos + 1],
        data[*pos + 2],
        data[*pos + 3],
    ]);
    *pos += 4;
    Ok(value)
}

/// 读取小端序u64
pub fn read_u64_le(data: &[u8], pos: &mut usize) -> io::Result<u64> {
    if *pos + 8 > data.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Cannot read u64"));
    }
    
    let value = u64::from_le_bytes([
        data[*pos],
        data[*pos + 1],
        data[*pos + 2],
        data[*pos + 3],
        data[*pos + 4],
        data[*pos + 5],
        data[*pos + 6],
        data[*pos + 7],
    ]);
    *pos += 8;
    Ok(value)
}

/// 数据处理（PC版PAK格式转换）
pub fn crypt_data(data: &mut [u8]) {
    const KEY: u8 = 0xF7;
    for byte in data.iter_mut() {
        *byte ^= KEY;
    }
} 