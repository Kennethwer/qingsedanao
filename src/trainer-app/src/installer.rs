//! 安装、卸载、状态检测，以及启动游戏、备份存档这些和游戏目录打交道的事。

use std::fs;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::embedded::{BEPINEX_FILES, DEFAULT_GAME_DIR, PLUGIN_DLL};

pub const GAME_EXE: &str = "CYANBRAIN.exe";
const PLUGIN_DIR: &str = "BepInEx/plugins/CyanBrainTrainer";
const PLUGIN_FILE: &str = "CyanBrainTrainer.Plugin.dll";
const PLUGIN_CONFIG: &str = "BepInEx/config/local.cyanbrain.trainer.cfg";
/// BepInEx 放在游戏根目录下的文件
const LOADER_FILES: &[&str] = &["winhttp.dll", "doorstop_config.ini", ".doorstop_version", "changelog.txt"];
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    Missing,
    UpToDate,
    Outdated,
}

#[derive(Clone, Copy)]
pub struct InstallStatus {
    pub game_found: bool,
    pub loader_installed: bool,
    pub plugin: PluginState,
}

pub fn is_game_dir(dir: &Path) -> bool {
    dir.join(GAME_EXE).is_file()
}

pub fn status(dir: Option<&Path>) -> InstallStatus {
    let Some(dir) = dir.filter(|d| is_game_dir(d)) else {
        return InstallStatus { game_found: false, loader_installed: false, plugin: PluginState::Missing };
    };
    let plugin = match fs::read(dir.join(PLUGIN_DIR).join(PLUGIN_FILE)) {
        Ok(bytes) if bytes == PLUGIN_DLL => PluginState::UpToDate,
        Ok(_) => PluginState::Outdated,
        Err(_) => PluginState::Missing,
    };
    InstallStatus { game_found: true, loader_installed: loader_installed(dir), plugin }
}

fn loader_installed(dir: &Path) -> bool {
    dir.join("winhttp.dll").is_file() && dir.join("BepInEx/core/BepInEx.dll").is_file()
}

// ---- 查找游戏目录 ----

/// 依次尝试：修改器所在目录 → Steam 游戏库 → 编译时的目录。
pub fn detect_game_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.ancestors().skip(1).take(2).find(|d| is_game_dir(d)) {
            return Some(dir.to_path_buf());
        }
    if let Some(dir) = steam_game_dirs().into_iter().next() {
        return Some(dir);
    }
    let fallback = PathBuf::from(DEFAULT_GAME_DIR);
    (!DEFAULT_GAME_DIR.is_empty() && is_game_dir(&fallback)).then_some(fallback)
}

fn steam_game_dirs() -> Vec<PathBuf> {
    let Some(steam) = steam_path() else { return Vec::new() };
    let mut libraries = vec![steam.clone()];
    if let Ok(vdf) = fs::read_to_string(steam.join("steamapps/libraryfolders.vdf")) {
        for line in vdf.lines() {
            if let Some(rest) = line.trim().strip_prefix("\"path\"") {
                libraries.push(PathBuf::from(rest.trim().trim_matches('"').replace("\\\\", "\\")));
            }
        }
    }
    libraries
        .iter()
        .filter_map(|lib| fs::read_dir(lib.join("steamapps/common")).ok())
        .flat_map(|entries| entries.flatten().map(|e| e.path()))
        .filter(|p| is_game_dir(p))
        .collect()
}

fn steam_path() -> Option<PathBuf> {
    let output = Command::new("reg")
        .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().find(|l| l.contains("SteamPath"))?;
    Some(PathBuf::from(line.split("REG_SZ").nth(1)?.trim()))
}

// ---- 安装 / 卸载 ----

fn ensure_ready(dir: &Path) -> Result<(), String> {
    if !is_game_dir(dir) {
        return Err(format!("{} 里没有 {GAME_EXE}，请重新选择游戏目录", dir.display()));
    }
    if is_game_running() {
        return Err("游戏正在运行，请先关闭游戏".into());
    }
    Ok(())
}

pub fn install(dir: &Path) -> Result<String, String> {
    ensure_ready(dir)?;

    // 已经有 BepInEx（比如装过别的模组）就不动它，只放插件
    let had_loader = loader_installed(dir);
    if !had_loader {
        for (rel, bytes) in BEPINEX_FILES {
            write_file(&dir.join(rel), bytes)?;
        }
    }
    create_dir(&dir.join("BepInEx/plugins"))?;
    write_file(&dir.join(PLUGIN_DIR).join(PLUGIN_FILE), PLUGIN_DLL)?;

    Ok(if had_loader {
        "修改器已安装，启动游戏即可生效".into()
    } else {
        "已安装 BepInEx 和修改器，启动游戏即可生效".into()
    })
}

pub fn uninstall(dir: &Path) -> Result<String, String> {
    ensure_ready(dir)?;

    remove_dir(&dir.join(PLUGIN_DIR))?;
    remove_file(&dir.join(PLUGIN_CONFIG))?;

    if has_other_mods(dir) {
        return Ok("已卸载修改器。检测到其他 BepInEx 模组，保留了 BepInEx".into());
    }
    for name in LOADER_FILES {
        let path = dir.join(name);
        // changelog.txt 名字太普通，只删和 BepInEx 自带内容一模一样的
        if *name == "changelog.txt" && !same_as_embedded(&path, name) {
            continue;
        }
        remove_file(&path)?;
    }
    remove_dir(&dir.join("BepInEx"))?;
    Ok("已卸载，游戏恢复为原版".into())
}

fn has_other_mods(dir: &Path) -> bool {
    ["BepInEx/plugins", "BepInEx/patchers", "BepInEx/monomod"]
        .iter()
        .filter_map(|sub| fs::read_dir(dir.join(sub)).ok())
        .any(|mut entries| entries.next().is_some())
}

fn same_as_embedded(path: &Path, rel: &str) -> bool {
    BEPINEX_FILES
        .iter()
        .find(|(r, _)| *r == rel)
        .is_some_and(|(_, bytes)| fs::read(path).is_ok_and(|b| b == *bytes))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        create_dir(parent)?;
    }
    fs::write(path, bytes).map_err(|e| format!("写入 {} 失败：{e}", path.display()))
}

fn create_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("创建 {} 失败：{e}", path.display()))
}

fn remove_file(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(format!("删除 {} 失败：{e}", path.display())),
        _ => Ok(()),
    }
}

fn remove_dir(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(format!("删除 {} 失败：{e}", path.display())),
        _ => Ok(()),
    }
}

// ---- 进程 ----

pub fn is_game_running() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };

    // SAFETY: 标准的 Toolhelp 进程枚举，entry 按要求初始化 dwSize，句柄用完关闭
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut ok = Process32FirstW(snapshot, &mut entry) != 0;
        while ok {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..len]).eq_ignore_ascii_case(GAME_EXE) {
                found = true;
                break;
            }
            ok = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
        found
    }
}

pub fn launch_game(dir: &Path) -> Result<(), String> {
    Command::new(dir.join(GAME_EXE))
        .current_dir(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("启动游戏失败：{e}"))
}

pub fn open_in_explorer(dir: &Path) {
    let _ = Command::new("explorer").arg(dir).spawn();
}

// ---- 存档 ----

/// 游戏的存档目录（Application.persistentDataPath）
pub fn save_dir() -> Option<PathBuf> {
    let profile = std::env::var_os("USERPROFILE")?;
    Some(PathBuf::from(profile).join(r"AppData\LocalLow\NEKOUJI STUDIO\CYANBRAIN"))
}

/// 把存档复制到 文档\青色大脑存档备份\<时间>，返回备份位置。
pub fn backup_saves() -> Result<PathBuf, String> {
    let src = save_dir().filter(|d| d.join("saves").is_dir()).ok_or("没有找到存档")?;
    let profile = std::env::var_os("USERPROFILE").ok_or("找不到用户目录")?;
    let dst = PathBuf::from(profile).join("Documents").join("青色大脑存档备份").join(timestamp());

    copy_dir(&src.join("saves"), &dst.join("saves"))?;
    for name in ["GameConfig.cfg", "GameCollection.clc"] {
        if src.join(name).is_file() {
            fs::copy(src.join(name), dst.join(name)).map_err(|e| format!("复制 {name} 失败：{e}"))?;
        }
    }
    Ok(dst)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    create_dir(dst)?;
    let entries = fs::read_dir(src).map_err(|e| format!("读取 {} 失败：{e}", src.display()))?;
    for entry in entries.flatten() {
        let (from, to) = (entry.path(), dst.join(entry.file_name()));
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| format!("复制 {} 失败：{e}", from.display()))?;
        }
    }
    Ok(())
}

fn timestamp() -> String {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: GetLocalTime 只往传入的结构体里写
    let t = unsafe {
        let mut t = std::mem::zeroed();
        GetLocalTime(&mut t);
        t
    };
    format!("{:04}-{:02}-{:02}_{:02}{:02}{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
}
