# 第三方组件

`BepInEx_5.4.23.5/` 是 [BepInEx](https://github.com/BepInEx/BepInEx) 5.4.23.5 官方发布包 `BepInEx_win_x64_5.4.23.5.zip` 解压后的原样内容，未做任何修改。
原始压缩包 sha256：`82f9878551030f54657792c0740d9d51a09500eeae1fba21106b0c441e6732c4`
下载地址：https://github.com/BepInEx/BepInEx/releases/tag/v5.4.23.5

控制软件编译时会把这些文件（API 文档 `*.xml` 除外）嵌进 exe，安装时原样写入游戏目录。

| 组件 | 许可证 | 源码 |
|---|---|---|
| BepInEx（BepInEx.dll、BepInEx.Preloader.dll、BepInEx.Harmony.dll、HarmonyXInterop.dll） | LGPL-2.1 | https://github.com/BepInEx/BepInEx |
| UnityDoorstop（winhttp.dll） | LGPL-2.1 | https://github.com/NeighTools/UnityDoorstop |
| HarmonyX（0Harmony.dll、0Harmony20.dll） | MIT | https://github.com/BepInEx/HarmonyX |
| MonoMod（MonoMod.RuntimeDetour.dll、MonoMod.Utils.dll） | MIT | https://github.com/MonoMod/MonoMod |
| Mono.Cecil（Mono.Cecil*.dll） | MIT | https://github.com/jbevain/cecil |
