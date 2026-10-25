# 青色大脑 修改器

《青色大脑 CYAN BRAIN》（Unity 2022.3 / Mono）的修改器，分两部分：

| 部分 | 位置 | 说明 |
|---|---|---|
| 游戏内插件（C#） | `src/CyanBrainTrainer.Plugin` | BepInEx 5 + HarmonyX，在游戏里改逻辑；按 F8 打开游戏内菜单；在 `127.0.0.1:37421` 开放控制端口 |
| 控制软件（Rust） | `src/trainer-app` | egui 界面，单个 exe；内嵌 BepInEx 和插件，一键安装 / 卸载 / 启动游戏 / 备份存档，并实时控制插件 |

两边的通信协议见 `src/Shared/Protocol.cs` 和 `src/trainer-app/src/protocol.rs`。

## 使用

直接运行 `dist/CyanBrainTrainer.exe`：「安装」页确认游戏目录 → 安装修改器 → 启动游戏 → 在「修改」页勾选功能。

## 构建

需要 .NET SDK（编译插件）、Rust 工具链（MSVC），以及本机装有游戏（插件要引用游戏自带的 DLL，仓库里不包含任何游戏文件）。

1. 在仓库根目录新建 `GameDir.local.props`（已加入 .gitignore），写上游戏目录：

   ```xml
   <Project>
     <PropertyGroup>
       <GameDir Condition="'$(GameDir)' == ''">D:\Games\CYAN BRAIN</GameDir>
     </PropertyGroup>
   </Project>
   ```

   也可以改用环境变量 `CYANBRAIN_GAME_DIR`。

2. 编译：

   ```
   cd src/trainer-app
   cargo build --release
   ```

`build.rs` 会先调用 `dotnet build` 编译插件，再把插件和 `third_party/BepInEx_5.4.23.5` 一起嵌进 exe。

- 单独改插件时 `dotnet build src/CyanBrainTrainer.Plugin -c Release` 会直接复制进游戏（游戏里已装 BepInEx 时）。

第三方组件的来源和许可证见 [third_party/README.md](third_party/README.md)。
