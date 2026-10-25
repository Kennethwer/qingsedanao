using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Globalization;
using System.Text;
using CyanBrainTrainer.Shared;
using UnityEngine;

namespace CyanBrainTrainer
{
    /// <summary>
    /// 跑在 Unity 主线程上的部分：执行一次性操作、处理外部命令、记录落脚点、生成状态快照。
    /// 外部命令在后台线程收到，统一排队到这里执行，避免在别的线程碰 Unity API。
    /// </summary>
    internal sealed class TrainerRuntime
    {
        public static TrainerRuntime Instance { get; private set; }

        /// <summary>修改器自己加经验时置位，让经验倍率补丁跳过这一次。</summary>
        internal static bool BypassExpMultiplier;

        private static readonly HashSet<string> KnownActions = new HashSet<string>(StringComparer.Ordinal)
        {
            ActionKeys.Heal, ActionKeys.RefillMedicine, ActionKeys.RefillAmmo, ActionKeys.AddExp, ActionKeys.ResetAll
        };

        private const int SafePositionCapacity = 12;
        private const float SafePositionInterval = 0.15f;

        private readonly TrainerSettings settings;
        private readonly ConcurrentQueue<Action> mainThreadQueue = new ConcurrentQueue<Action>();
        private readonly List<Vector2> safePositions = new List<Vector2>();
        private int safePositionsOwner;
        private float nextSafeSampleTime;
        private float nextSnapshotTime;
        private float saveDueTime = -1f;
        private volatile string stateSnapshot = "";
        private volatile bool inGame;

        public TrainerRuntime(TrainerSettings settings)
        {
            Instance = this;
            this.settings = settings;
            settings.MoveSpeed.SettingChanged += (_, __) => RefreshMoveConfig();
            settings.JumpHeight.SettingChanged += (_, __) => RefreshMoveConfig();
            settings.GameSpeed.SettingChanged += (_, __) => ApplyGameSpeed();
            settings.Config.SettingChanged += (_, __) => saveDueTime = Time.unscaledTime + 1f;
            // 游戏本身不改 timeScale（只有调试控制台会），所以启动时设一次、改动时再设即可
            if (!Mathf.Approximately(settings.GameSpeed.Value, 1f))
            {
                ApplyGameSpeed();
            }
        }

        public string StateSnapshot => stateSnapshot;

        public void Update()
        {
            while (mainThreadQueue.TryDequeue(out Action action))
            {
                try
                {
                    action();
                }
                catch (Exception e)
                {
                    Plugin.Log.LogError(e);
                }
            }

            TrackSafePosition();

            if (Time.unscaledTime >= nextSnapshotTime)
            {
                nextSnapshotTime = Time.unscaledTime + 0.2f;
                RebuildSnapshot();
            }

            if (saveDueTime > 0f && Time.unscaledTime >= saveDueTime)
            {
                saveDueTime = -1f;
                settings.Save();
            }
        }

        public void SaveNow()
        {
            if (saveDueTime > 0f)
            {
                saveDueTime = -1f;
                settings.Save();
            }
        }

        #region 一次性操作

        /// <summary>执行一次性操作，必须在主线程调用。</summary>
        public bool Execute(string action, string arg, out string message)
        {
            if (action == ActionKeys.ResetAll)
            {
                settings.ResetAll();
                message = "已恢复默认设置";
                return true;
            }

            Player player = Engine.player;
            if (player == null)
            {
                message = "还没进入游戏";
                return false;
            }

            switch (action)
            {
                case ActionKeys.Heal:
                    return Heal(player, out message);
                case ActionKeys.RefillMedicine:
                    player.status.medicineCount = player.status.maxMedicineCount;
                    player.playerUI.UpdateWeaponPanelSingle(-1, -2, player.status.medicineCount);
                    message = "药剂已补满";
                    return true;
                case ActionKeys.RefillAmmo:
                    RefillAmmo(player);
                    message = "弹药和地雷已补满";
                    return true;
                case ActionKeys.AddExp:
                    int amount = 1000;
                    if (!string.IsNullOrEmpty(arg))
                    {
                        int.TryParse(arg, NumberStyles.Integer, CultureInfo.InvariantCulture, out amount);
                    }
                    amount = Mathf.Clamp(amount, 1, 1000000);
                    BypassExpMultiplier = true;
                    try
                    {
                        player.AddExp(amount, useExpRate: false, useAudio: true);
                    }
                    finally
                    {
                        BypassExpMultiplier = false;
                    }
                    message = $"经验 +{amount}";
                    return true;
                default:
                    message = "未知操作 " + action;
                    return false;
            }
        }

        private static bool Heal(Player player, out string message)
        {
            if (player.actionFlag.Has(ActionFlag.IsDying | ActionFlag.Dead))
            {
                message = "角色已经倒下，无法回血";
                return false;
            }
            Player.Status status = player.status;
            float start = Mathf.Clamp01(status.hp / status.maxHp);
            status.hp = status.maxHp;
            player.playerUI.DoHealEffect(start, 1f);
            message = "血量已回满";
            return true;
        }

        private static void RefillAmmo(Player player)
        {
            Player.Inventory inventory = player.inventory;
            foreach (GunSO gun in inventory.obtainedGuns)
            {
                BulletSO bullet = gun != null ? gun.bulletSO : null;
                // 手枪子弹本来就是无限的（备弹 -1）
                if (bullet == null || bullet.bulletType == BulletSO.BulletType.Pistol)
                {
                    continue;
                }
                inventory.bulletCountDictionary[bullet.bulletType] = bullet.max;
            }
            foreach (WeaponHandle weapon in inventory.weaponSlots)
            {
                if (weapon?.gunSO?.bulletSO != null)
                {
                    weapon.clipBulletCount = weapon.gunSO.bulletSO.clip;
                }
            }

            player.status.landmineCount = player.status.maxLandmineCount;
            player.soundLibrary.Play("pickupBullet");
            player.playerUI.inventoryPage.UpdateAllBulletEntries(inventory);
            player.playerUI.UpdateWeaponPanel();
            player.playerUI.UpdateLandmineCount(player.status.landmineCount);
        }

        #endregion

        #region 掉坑拉回

        /// <summary>每隔一小段时间记录一次玩家站稳的位置，掉进死亡区域时拉回去。</summary>
        private void TrackSafePosition()
        {
            Player player = Engine.player;
            if (player == null)
            {
                safePositions.Clear();
                return;
            }

            // 换场景后玩家是新对象，旧位置作废
            int owner = player.GetInstanceID();
            if (owner != safePositionsOwner)
            {
                safePositionsOwner = owner;
                safePositions.Clear();
            }

            if (Time.time < nextSafeSampleTime)
            {
                return;
            }
            nextSafeSampleTime = Time.time + SafePositionInterval;

            if (player.playerType != Player.PlayerType.Combat || !player.playerController.IsGrounded())
            {
                return;
            }
            if (player.actionFlag.Has(ActionFlag.Dead | ActionFlag.IsDying | ActionFlag.Interactive | ActionFlag.Roll | ActionFlag.AutoMove))
            {
                return;
            }

            Vector2 pos = player.transform.position;
            if (safePositions.Count > 0 && (safePositions[safePositions.Count - 1] - pos).sqrMagnitude < 0.01f)
            {
                return;
            }
            safePositions.Add(pos);
            if (safePositions.Count > SafePositionCapacity)
            {
                safePositions.RemoveAt(0);
            }
        }

        public bool TryRescueFromDeadZone(Player player)
        {
            if (safePositions.Count == 0)
            {
                Plugin.Log.LogWarning("掉进死亡区域，但还没有记录到落脚点，无法拉回");
                return false;
            }

            // 取大约半秒前的落脚点，离坑边远一点；用过之后丢掉更新的点，连续掉坑会越退越远
            int index = Mathf.Max(0, safePositions.Count - 4);
            Vector2 pos = safePositions[index];
            safePositions.RemoveRange(index + 1, safePositions.Count - index - 1);

            player.playerController.Transport(pos);
            if (Engine.gameCamera != null)
            {
                Engine.gameCamera.ApplyToTargetPos();
            }
            Plugin.Log.LogInfo($"防秒杀：掉坑，已拉回 {pos}");
            return true;
        }

        #endregion

        public static void RefreshMoveConfig()
        {
            Player player = Engine.player;
            if (player != null && player.playerController != null && player.playerController.setting != null)
            {
                player.playerController.CalcMoveConfig();
            }
        }

        private void ApplyGameSpeed()
        {
            Time.timeScale = settings.GameSpeed.Value;
        }

        #region 外部控制

        private void RebuildSnapshot()
        {
            StringBuilder sb = new StringBuilder(512);
            settings.AppendState(sb);

            Player player = Engine.player;
            bool hasPlayer = player != null;
            inGame = hasPlayer;
            Append(sb, StateKeys.InGame, hasPlayer ? "1" : "0");
            Append(sb, StateKeys.Scene, GameState.currentScene.ToString());
            if (hasPlayer)
            {
                Player.Status status = player.status;
                Append(sb, StateKeys.Hp, TrainerSettings.Format(status.hp));
                Append(sb, StateKeys.MaxHp, TrainerSettings.Format(status.maxHp));
                Append(sb, StateKeys.Exp, TrainerSettings.Format(Mathf.Floor(status.exp)));
                Append(sb, StateKeys.Energy, TrainerSettings.Format(status.energy));
                Append(sb, StateKeys.Medicine, TrainerSettings.Format(status.medicineCount));
                Append(sb, StateKeys.MaxMedicine, TrainerSettings.Format(status.maxMedicineCount));
            }
            stateSnapshot = sb.ToString();
        }

        private static void Append(StringBuilder sb, string key, string value)
        {
            sb.Append(key).Append('=').Append(value).Append(';');
        }

        /// <summary>处理一行外部命令。在后台线程调用，只做解析，真正的修改排队到主线程。</summary>
        public string HandleRemoteCommand(string line)
        {
            string[] parts = (line ?? "").Trim().Split(new[] { ' ' }, 3, StringSplitOptions.RemoveEmptyEntries);
            if (parts.Length == 0)
            {
                return Protocol.Err + " 空命令";
            }

            switch (parts[0].ToUpperInvariant())
            {
                case Protocol.Ping:
                    return Protocol.Pong;

                case Protocol.Get:
                    return Protocol.State + " " + stateSnapshot;

                case Protocol.Set:
                {
                    if (parts.Length < 3)
                    {
                        return Protocol.Err + " 用法：SET <key> <value>";
                    }
                    if (!settings.TryParse(parts[1], parts[2], out var entry, out object value, out string error))
                    {
                        return Protocol.Err + " " + error;
                    }
                    mainThreadQueue.Enqueue(() => entry.BoxedValue = value);
                    return Protocol.Ok;
                }

                case Protocol.Do:
                {
                    if (parts.Length < 2)
                    {
                        return Protocol.Err + " 用法：DO <action> [arg]";
                    }
                    string action = parts[1];
                    string arg = parts.Length > 2 ? parts[2] : null;
                    if (!KnownActions.Contains(action))
                    {
                        return Protocol.Err + " 未知操作 " + action;
                    }
                    if (!inGame && action != ActionKeys.ResetAll)
                    {
                        return Protocol.Err + " 还没进入游戏";
                    }
                    mainThreadQueue.Enqueue(() =>
                    {
                        if (!Execute(action, arg, out string message))
                        {
                            Plugin.Log.LogWarning($"外部操作 {action} 失败：{message}");
                        }
                    });
                    return Protocol.Ok;
                }

                default:
                    return Protocol.Err + " 未知命令 " + parts[0];
            }
        }

        #endregion
    }
}
