using HarmonyLib;

namespace CyanBrainTrainer.Patches
{
    // ---- 伤害 ----

    /// <summary>所有打到怪物身上的伤害最终都进 MonsterBase.TakeDamage（UtilMonster 的重写也会调 base）。</summary>
    [HarmonyPatch(typeof(MonsterBase), nameof(MonsterBase.TakeDamage))]
    internal static class MonsterTakeDamagePatch
    {
        private static void Prefix(DamageType damageType, ref float damage)
        {
            // 只放大玩家造成的伤害（枪、近战、地雷）
            if (damageType != DamageType.Gun && damageType != DamageType.PlayerMelee && damageType != DamageType.Lambine)
            {
                return;
            }
            TrainerSettings s = Plugin.Settings;
            if (s.OneHitKill.Value)
            {
                damage = 1e7f;
            }
            else
            {
                damage *= s.DamageMultiplier.Value;
            }
        }
    }

    // ---- 子弹 ----

    [HarmonyPatch(typeof(Player.Inventory), nameof(Player.Inventory.CostBullet))]
    internal static class CostBulletPatch
    {
        private static bool Prefix()
        {
            return !Plugin.Settings.InfiniteAmmo.Value;
        }
    }

    [HarmonyPatch(typeof(WeaponHandle), nameof(WeaponHandle.ShootBullet))]
    internal static class ShootBulletPatch
    {
        // 弹夹不减；栓动枪照样要拉栓
        private static bool Prefix(WeaponHandle __instance)
        {
            if (!Plugin.Settings.InfiniteAmmo.Value)
            {
                return true;
            }
            if (__instance.gunSO != null && __instance.gunSO.needPullBolt)
            {
                __instance.needPullBolt = true;
            }
            return false;
        }
    }

    // ---- 经验 ----

    /// <summary>杀怪经验是直接加到 status.exp 上的，按前后差值补足倍率。</summary>
    [HarmonyPatch(typeof(Player), nameof(Player.OnMonsterKilled))]
    internal static class MonsterKilledExpPatch
    {
        private static void Prefix(Player __instance, out float __state)
        {
            __state = __instance.status.exp;
        }

        private static void Postfix(Player __instance, float __state)
        {
            float multiplier = Plugin.Settings.ExpMultiplier.Value;
            float gained = __instance.status.exp - __state;
            if (gained <= 0f || multiplier <= 1f)
            {
                return;
            }
            __instance.status.exp += gained * (multiplier - 1f);
            __instance.playerUI.UpdateExpCount((int)__instance.status.exp);
        }
    }

    /// <summary>拾取经验等走 AddExp。</summary>
    [HarmonyPatch(typeof(Player), nameof(Player.AddExp))]
    internal static class AddExpPatch
    {
        private static void Prefix(ref float amount)
        {
            if (!TrainerRuntime.BypassExpMultiplier)
            {
                amount *= Plugin.Settings.ExpMultiplier.Value;
            }
        }
    }
}
