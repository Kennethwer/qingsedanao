using System;
using HarmonyLib;
using UnityEngine;

namespace CyanBrainTrainer.Patches
{
    // ---- 无敌 / 摔落 ----

    [HarmonyPatch(typeof(Player), nameof(Player.TakeDamage))]
    internal static class PlayerTakeDamagePatch
    {
        // 返回 true 表示这次伤害把玩家打死了，怪物会接着进入处决，所以这里统一返回 false
        private static bool Prefix(DamageType damageType, ref bool __result)
        {
            TrainerSettings s = Plugin.Settings;
            if (s.GodMode.Value || (s.NoInstantKill.Value && damageType == DamageType.Fall))
            {
                __result = false;
                return false;
            }
            return true;
        }
    }

    [HarmonyPatch(typeof(Mech), nameof(Mech.TakeDamage))]
    internal static class MechTakeDamagePatch
    {
        private static bool Prefix(ref bool __result)
        {
            if (Plugin.Settings.GodMode.Value)
            {
                __result = false;
                return false;
            }
            return true;
        }
    }

    // ---- 防秒杀：这些来源不走 TakeDamage，碰到就直接进处决 ----

    [HarmonyPatch(typeof(Trap), "CheckDamage")]
    internal static class TrapCheckDamagePatch
    {
        private static bool Prefix()
        {
            return !Plugin.Settings.BlockInstantKill;
        }
    }

    [HarmonyPatch(typeof(Player), nameof(Player.CanTriggerLandmine))]
    internal static class PlayerLandminePatch
    {
        private static void Postfix(ref bool __result)
        {
            if (Plugin.Settings.BlockInstantKill)
            {
                __result = false;
            }
        }
    }

    [HarmonyPatch(typeof(Player), nameof(Player.EnterDeadZone))]
    internal static class PlayerDeadZonePatch
    {
        private static bool Prefix(Player __instance)
        {
            if (!Plugin.Settings.BlockInstantKill)
            {
                return true;
            }
            return !TrainerRuntime.Instance.TryRescueFromDeadZone(__instance);
        }
    }

    /// <summary>M4：玩家一进入它的触发区就被处决。</summary>
    [HarmonyPatch(typeof(M4), "OnTriggerEnter2D")]
    internal static class M4ExecutePatch
    {
        private static bool Prefix(Collider2D other)
        {
            return !(Plugin.Settings.BlockInstantKill && other.CompareTag("Player"));
        }
    }

    /// <summary>
    /// 追击型怪物在追击中碰到玩家（且玩家没在翻滚）就直接处决。
    /// 开启保护时照搬原逻辑，只去掉“碰到即处决”这一步，怪物会继续追。
    /// </summary>
    [HarmonyPatch(typeof(UtilMonster.MoveToTargetAndExecuteStrategy), nameof(UtilMonster.MoveToTargetAndExecuteStrategy.Process))]
    internal static class ChaseExecutePatch
    {
        private static readonly AccessTools.FieldRef<UtilMonster, float> ChasingFinishTime =
            AccessTools.FieldRefAccess<UtilMonster, float>("chasingFinishTime");

        private static bool Prefix(UtilMonster ___monster, Func<bool> ___exitCondition, ref Node.Status __result)
        {
            if (!Plugin.Settings.BlockInstantKill)
            {
                return true;
            }
            if (___exitCondition() || ChasingFinishTime(___monster) < Time.time)
            {
                __result = Node.Status.Failure;
                return false;
            }
            if ((___monster.actionFlags & MonsterBase.ActionFlags.GoingToExecute) == 0)
            {
                ___monster.controller.SetNewTarget(Engine.player.transform.position);
                __result = Node.Status.Success;
                return false;
            }
            __result = Node.Status.Running;
            return false;
        }
    }

    // ---- 资源锁定 ----

    /// <summary>在 Player.Update 之前把数值拉满，这样游戏同一帧刷新的 UI 也是满的。</summary>
    [HarmonyPatch(typeof(Player), "Update")]
    internal static class PlayerResourcePatch
    {
        private static void Prefix(Player __instance)
        {
            TrainerSettings s = Plugin.Settings;
            Player.Status status = __instance.status;
            if (s.InfiniteEnergy.Value)
            {
                status.energy = 1f;
            }
            if (s.LockHunger.Value)
            {
                status.hunger = 1f;
            }
            if (s.InfiniteFlashlight.Value)
            {
                status.flashlightPower = 1f;
            }
        }
    }
}
