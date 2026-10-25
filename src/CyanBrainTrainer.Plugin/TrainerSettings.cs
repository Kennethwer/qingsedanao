using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;
using BepInEx.Configuration;
using CyanBrainTrainer.Shared;
using UnityEngine.InputSystem;

namespace CyanBrainTrainer
{
    /// <summary>
    /// 所有开关/数值都存在 BepInEx 配置文件（BepInEx/config/local.cyanbrain.trainer.cfg）里，
    /// 游戏内菜单和外部程序改的是同一份数据。
    /// </summary>
    internal sealed class TrainerSettings
    {
        public readonly ConfigFile Config;

        public readonly ConfigEntry<bool> GodMode;
        public readonly ConfigEntry<bool> NoInstantKill;
        public readonly ConfigEntry<bool> LockHunger;
        public readonly ConfigEntry<bool> InfiniteEnergy;
        public readonly ConfigEntry<bool> InfiniteFlashlight;

        public readonly ConfigEntry<bool> OneHitKill;
        public readonly ConfigEntry<float> DamageMultiplier;
        public readonly ConfigEntry<bool> InfiniteAmmo;

        public readonly ConfigEntry<bool> MultiJump;
        public readonly ConfigEntry<int> ExtraJumps;
        public readonly ConfigEntry<float> JumpHeight;
        public readonly ConfigEntry<float> MoveSpeed;
        public readonly ConfigEntry<float> GameSpeed;

        public readonly ConfigEntry<float> ExpMultiplier;

        public readonly ConfigEntry<Key> MenuKey;
        public readonly ConfigEntry<bool> RemoteEnabled;
        public readonly ConfigEntry<int> RemotePort;

        private readonly Dictionary<string, ConfigEntryBase> byKey = new Dictionary<string, ConfigEntryBase>(StringComparer.OrdinalIgnoreCase);
        private readonly List<ConfigEntryBase> cheats = new List<ConfigEntryBase>();

        public TrainerSettings(ConfigFile config)
        {
            Config = config;
            // 拖动滑条时每帧都会改值，改成由 TrainerRuntime 延迟统一保存
            config.SaveOnConfigSet = false;

            GodMode = Bind("生存", SettingKeys.GodMode, false, "无敌：不掉血，也不会被陷阱、地雷、怪物处决");
            NoInstantKill = Bind("生存", SettingKeys.NoInstantKill, false, "防秒杀：屏蔽陷阱处决、踩地雷、掉坑（自动拉回落脚点）、怪物抓取处决、摔落伤害，普通伤害照常");
            LockHunger = Bind("生存", SettingKeys.LockHunger, false, "锁定饱食度为满");
            InfiniteEnergy = Bind("生存", SettingKeys.InfiniteEnergy, false, "能量（翻滚消耗）始终为满");
            InfiniteFlashlight = Bind("生存", SettingKeys.InfiniteFlashlight, false, "手电电量始终为满");

            OneHitKill = Bind("战斗", SettingKeys.OneHitKill, false, "一击必杀");
            DamageMultiplier = Bind("战斗", SettingKeys.DamageMultiplier, 1f, "玩家对怪物的伤害倍率", new AcceptableValueRange<float>(1f, 20f));
            InfiniteAmmo = Bind("战斗", SettingKeys.InfiniteAmmo, false, "开枪不消耗子弹，弹夹不减少");

            MultiJump = Bind("移动", SettingKeys.MultiJump, false, "允许在空中再次起跳");
            ExtraJumps = Bind("移动", SettingKeys.ExtraJumps, 1, "连跳时空中额外可跳的次数", new AcceptableValueRange<int>(1, 10));
            JumpHeight = Bind("移动", SettingKeys.JumpHeight, 1f, "跳跃高度倍率", new AcceptableValueRange<float>(0.5f, 3f));
            MoveSpeed = Bind("移动", SettingKeys.MoveSpeed, 1f, "移动速度倍率", new AcceptableValueRange<float>(0.5f, 3f));
            GameSpeed = Bind("移动", SettingKeys.GameSpeed, 1f, "游戏整体速度（Time.timeScale）", new AcceptableValueRange<float>(0.1f, 3f));

            ExpMultiplier = Bind("经验", SettingKeys.ExpMultiplier, 1f, "获得经验的倍率", new AcceptableValueRange<float>(1f, 10f));

            MenuKey = config.Bind("通用", "menuKey", Key.F8,
                "呼出游戏内菜单的按键（Unity InputSystem 的 Key 名称）。不要设成 F1：游戏里残留的调试代码按 F1 会重置 Steam 成就");
            RemoteEnabled = config.Bind("外部控制", "enabled", true, "是否开启本机 TCP 端口供外部控制程序连接");
            RemotePort = config.Bind("外部控制", "port", Protocol.DefaultPort,
                new ConfigDescription("监听端口（只监听 127.0.0.1）", new AcceptableValueRange<int>(1024, 65535)));

            config.Save();
        }

        /// <summary>陷阱、地雷、掉坑、抓取处决这类“直接死”的来源是否要屏蔽。</summary>
        public bool BlockInstantKill => GodMode.Value || NoInstantKill.Value;

        private ConfigEntry<T> Bind<T>(string section, string key, T defaultValue, string description, AcceptableValueBase range = null)
        {
            ConfigEntry<T> entry = Config.Bind(section, key, defaultValue, new ConfigDescription(description, range));
            byKey[key] = entry;
            cheats.Add(entry);
            return entry;
        }

        public void Save()
        {
            Config.Save();
        }

        public void ResetAll()
        {
            foreach (ConfigEntryBase entry in cheats)
            {
                entry.BoxedValue = entry.DefaultValue;
            }
        }

        public void AppendState(StringBuilder sb)
        {
            foreach (ConfigEntryBase entry in cheats)
            {
                sb.Append(entry.Definition.Key).Append('=').Append(Format(entry.BoxedValue)).Append(';');
            }
        }

        /// <summary>解析外部传来的值。只读元数据，可以在后台线程调用。</summary>
        public bool TryParse(string key, string raw, out ConfigEntryBase entry, out object value, out string error)
        {
            value = null;
            error = null;
            if (!byKey.TryGetValue(key, out entry))
            {
                error = "未知设置项 " + key;
                return false;
            }

            Type type = entry.SettingType;
            if (type == typeof(bool))
            {
                switch (raw.Trim().ToLowerInvariant())
                {
                    case "1":
                    case "true":
                    case "on":
                        value = true;
                        return true;
                    case "0":
                    case "false":
                    case "off":
                        value = false;
                        return true;
                }
            }
            else if (type == typeof(float))
            {
                if (float.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out float f) && !float.IsNaN(f) && !float.IsInfinity(f))
                {
                    value = Clamp(entry, f);
                    return true;
                }
            }
            else if (type == typeof(int))
            {
                if (int.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out int i))
                {
                    value = Clamp(entry, i);
                    return true;
                }
            }

            error = $"{key} 的值无效：{raw}";
            return false;
        }

        private static object Clamp(ConfigEntryBase entry, object value)
        {
            AcceptableValueBase range = entry.Description.AcceptableValues;
            return range != null ? range.Clamp(value) : value;
        }

        public static string Format(object value)
        {
            switch (value)
            {
                case bool b:
                    return b ? "1" : "0";
                case float f:
                    return f.ToString("0.###", CultureInfo.InvariantCulture);
                case IFormattable formattable:
                    return formattable.ToString(null, CultureInfo.InvariantCulture);
                default:
                    return value?.ToString() ?? "";
            }
        }
    }
}
