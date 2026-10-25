//! 编译期准备要嵌进 exe 的文件：
//! 1. 调 dotnet 编译 C# 插件（CyanBrainTrainer.Plugin.dll）
//! 2. 列出 third_party 里的 BepInEx 文件
//!
//! 生成 $OUT_DIR/embedded.rs，由 src/embedded.rs include 进来。
//!
//! 游戏目录取环境变量 CYANBRAIN_GAME_DIR，没有就读仓库根目录 GameDir.local.props 里的 GameDir
//! （编译插件需要游戏自带的 Managed DLL）。

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BEPINEX_DIR: &str = "third_party/BepInEx_5.4.23.5";
const LOCAL_PROPS: &str = "GameDir.local.props";

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap().to_path_buf();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let game_dir = env::var("CYANBRAIN_GAME_DIR").ok().or_else(|| read_props_game_dir(&repo_root));
    println!("cargo:rerun-if-env-changed=CYANBRAIN_GAME_DIR");
    println!("cargo:rerun-if-changed={}", repo_root.join(LOCAL_PROPS).display());

    let plugin_dll = build_plugin(&repo_root, &out_dir, game_dir.as_deref());

    let bepinex_root = repo_root.join(BEPINEX_DIR);
    println!("cargo:rerun-if-changed={}", bepinex_root.display());
    let mut bepinex_files = Vec::new();
    collect_files(&bepinex_root, &mut bepinex_files);
    bepinex_files.sort();

    let mut code = String::new();
    writeln!(code, "pub const PLUGIN_DLL: &[u8] = include_bytes!({:?});", plugin_dll).unwrap();
    writeln!(code, "pub const BEPINEX_FILES: &[(&str, &[u8])] = &[").unwrap();
    for file in &bepinex_files {
        // XML 是 API 文档，运行时用不到
        if file.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")) {
            continue;
        }
        let rel = file.strip_prefix(&bepinex_root).unwrap().to_string_lossy().replace('\\', "/");
        writeln!(code, "    ({:?}, include_bytes!({:?})),", rel, file).unwrap();
    }
    writeln!(code, "];").unwrap();
    writeln!(code, "pub const DEFAULT_GAME_DIR: &str = {:?};", game_dir.unwrap_or_default()).unwrap();
    fs::write(out_dir.join("embedded.rs"), code).unwrap();
}

fn build_plugin(repo_root: &Path, out_dir: &Path, game_dir: Option<&str>) -> PathBuf {
    let plugin_dir = repo_root.join("src").join("CyanBrainTrainer.Plugin");
    let output = out_dir.join("plugin");

    // 只盯源文件，bin/obj 每次编译都会变，盯了会导致反复重跑
    let mut sources = Vec::new();
    collect_files(&plugin_dir, &mut sources);
    for source in sources {
        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or_default();
        if ext == "cs" || ext == "csproj" {
            println!("cargo:rerun-if-changed={}", source.display());
        }
    }
    println!("cargo:rerun-if-changed={}", repo_root.join("src/Shared/Protocol.cs").display());

    let mut cmd = Command::new("dotnet");
    cmd.arg("build")
        .arg(plugin_dir.join("CyanBrainTrainer.Plugin.csproj"))
        .args(["-c", "Release", "-nologo", "-v", "q", "-p:DeployToGame=false"])
        .arg(format!("-p:OutDir={}\\", output.display()));
    if let Some(dir) = game_dir {
        cmd.arg(format!("-p:GameDir={dir}"));
    }
    let status = cmd.status().expect("找不到 dotnet：编译插件需要 .NET SDK");
    assert!(status.success(), "C# 插件编译失败，见上面的 dotnet 输出");

    output.join("CyanBrainTrainer.Plugin.dll")
}

/// 递归列出目录下的文件，跳过 bin/obj。
fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name();
            if name != "bin" && name != "obj" {
                collect_files(&path, out);
            }
        } else {
            out.push(path);
        }
    }
}

/// 从 GameDir.local.props 里取 <GameDir ...>值</GameDir>。
fn read_props_game_dir(repo_root: &Path) -> Option<String> {
    let text = fs::read_to_string(repo_root.join(LOCAL_PROPS)).ok()?;
    let start = text.find("<GameDir")?;
    let value_start = start + text[start..].find('>')? + 1;
    let value_end = value_start + text[value_start..].find("</GameDir>")?;
    Some(text[value_start..value_end].trim().to_string())
}
