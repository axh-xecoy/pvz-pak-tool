# PVZ PAK Tool

一个植物大战僵尸PAK解包打包的CLI工具

## 功能特性

- 🔓 **解包PAK文件** - 将.pak文件解包到指定目录
- 📦 **打包目录** - 将目录打包为.pak文件
- 🖥️ **交互式浏览器** - REPL模式下浏览PAK文件内容
- ⚡ **批处理模式** - 支持命令行批量操作
- 🎨 **彩色输出** - 美观的终端界面显示
- 🔍 **文件搜索** - 支持正则表达式搜索文件

## 安装

确保已安装Rust环境，然后克隆并编译：

```bash
git clone https://github.com/axh-xecoy/pvz-pak-tool.git
cd pvz-pak-tool
cargo build --release
```

编译后的可执行文件位于 `target/release/pkt.exe`

## 使用方法

### 基本用法

```bash
# 解包PAK文件到目录
pkt game.pak -o extracted_files/

# 将目录打包为PAK文件
pkt game_files/ -o game.pak

# 进入交互式浏览模式
pkt game.pak

# 批处理模式执行命令
pkt game.pak -c "ls" -c "find -filter *.xml"
```

### 交互式模式命令

在REPL模式下，支持以下命令：

- `ls [path]` - 列出当前目录或指定路径的文件
- `cd <path>` - 切换到指定目录
- `find -name <filename>` - 按文件名精确查找
- `find -filter <pattern>` - 通配符搜索（支持 * ? [abc] [a-z] [!abc]）
- `find -match <regex>` - 正则表达式搜索
- `find -extract <dir>` - 搜索并提取文件到指定目录
- `info` - 显示PAK文件信息
- `pwd` - 显示当前路径
- `help` - 显示帮助信息
- `exit` - 退出程序

### 高级功能

- 支持输出重定向：`ls > filelist.txt`
- 支持自定义格式化输出
- 自动检测PAK文件压缩模式
- 跨平台路径处理

## 项目特色

- **强大的搜索功能** - 交互模式下，支持文件名、通配符和正则表达式多种搜索方式，导出特定条件的文件
- **交互式文件管理** - 类Unix的命令行界面，支持目录导航和文件操作
- **批量文件提取** - 支持按条件筛选并批量提取文件

## 许可证

本项目采用开源许可证，详见LICENSE文件。