using System;
using BepInEx;
using BepInEx.Logging;
using CyanBrainTrainer.Shared;
using HarmonyLib;
using UnityEngine;

namespace CyanBrainTrainer
{
    [BepInPlugin(Guid, Name, Version)]
    [BepInProcess("CYANBRAIN.exe")]
    public sealed class Plugin : BaseUnityPlugin
    {
        public const string Guid = "local.cyanbrain.trainer";
        public const string Name = "CyanBrain Trainer";
        public const string Version = "1.0.0";

        internal static ManualLogSource Log;
        internal static TrainerSettings Settings;

        private Harmony harmony;

        private void Awake()
        {
            Log = Logger;
            Settings = new TrainerSettings(Config);
            TrainerRuntime runtime = new TrainerRuntime(Settings);
            ApplyPatches();

            RemoteServer server = null;
            if (Settings.RemoteEnabled.Value)
            {
                server = new RemoteServer(Settings.RemotePort.Value, $"{Protocol.Hello} {Version}", runtime.HandleRemoteCommand);
                server.Start();
            }

            // BepInEx 的管理对象在这个游戏里加载第一个场景时就被销毁了，挂在它上面的 Update/OnGUI 不会再跑。
            // 自己建一个 HideAndDontSave 的宿主对象，不随场景卸载。
            GameObject hostObject = new GameObject("CyanBrainTrainer") { hideFlags = HideFlags.HideAndDontSave };
            DontDestroyOnLoad(hostObject);
            hostObject.AddComponent<TrainerHost>().Init(runtime, server);

            Logger.LogInfo($"{Name} {Version} 已加载，按 {Settings.MenuKey.Value} 打开菜单");
        }

        /// <summary>逐个类打补丁：游戏更新后某个方法变了，只影响对应功能，不会全部失效。</summary>
        private void ApplyPatches()
        {
            harmony = new Harmony(Guid);
            int ok = 0;
            int failed = 0;
            foreach (Type type in AccessTools.GetTypesFromAssembly(typeof(Plugin).Assembly))
            {
                if (type.GetCustomAttributes(typeof(HarmonyPatch), false).Length == 0)
                {
                    continue;
                }
                try
                {
                    harmony.CreateClassProcessor(type).Patch();
                    ok++;
                }
                catch (Exception e)
                {
                    failed++;
                    Logger.LogError($"补丁 {type.Name} 失败：{e}");
                }
            }
            Logger.LogInfo($"已应用 {ok} 个补丁，失败 {failed} 个");
        }

        private void OnDestroy()
        {
            Logger.LogInfo("BepInEx 管理对象已销毁，修改器由独立宿主继续运行");
        }
    }

    /// <summary>实际跑 Update/OnGUI 的宿主组件。</summary>
    internal sealed class TrainerHost : MonoBehaviour
    {
        private TrainerRuntime runtime;
        private TrainerMenu menu;
        private RemoteServer server;

        public void Init(TrainerRuntime runtime, RemoteServer server)
        {
            this.runtime = runtime;
            this.server = server;
            menu = new TrainerMenu(Plugin.Settings, runtime, DescribeRemote);
        }

        private string DescribeRemote()
        {
            if (server == null)
            {
                return "外部控制：已关闭";
            }
            if (!server.IsRunning)
            {
                return $"外部控制：端口 {server.Port} 监听失败（{server.LastError}）";
            }
            return $"外部控制：127.0.0.1:{server.Port}，已连接 {server.ClientCount} 个客户端";
        }

        private void Update()
        {
            runtime.Update();
            menu.Update();
        }

        private void OnGUI()
        {
            menu.OnGUI();
        }

        private void OnApplicationQuit()
        {
            runtime.SaveNow();
            server?.Stop();
        }

        private void OnDestroy()
        {
            Plugin.Log.LogWarning("修改器宿主对象被销毁");
            runtime.SaveNow();
            server?.Stop();
        }
    }
}
