//! 编译期嵌进 exe 的文件，由 build.rs 生成。
//!
//! - `PLUGIN_DLL`：C# 插件
//! - `BEPINEX_FILES`：BepInEx 5 的文件，(相对游戏目录的路径, 内容)
//! - `DEFAULT_GAME_DIR`：编译时的游戏目录，自动查找的最后一个候选

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
