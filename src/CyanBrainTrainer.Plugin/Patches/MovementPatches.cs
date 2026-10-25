using HarmonyLib;
using UnityEngine;

namespace CyanBrainTrainer.Patches
{
    /// <summary>
    /// PlayerController 的移动参数全部来自 PlayerSettingSO.CalcMoveConfig()，在这里统一乘倍率。
    /// 改了倍率后调用 TrainerRuntime.RefreshMoveConfig() 让游戏重新算一次。
    /// </summary>
    [HarmonyPatch(typeof(PlayerSettingSO), nameof(PlayerSettingSO.CalcMoveConfig))]
    internal static class MoveConfigPatch
    {
        private static void Postfix(ref PlayerController.MoveConfig __result)
        {
            TrainerSettings s = Plugin.Settings;
            float speed = s.MoveSpeed.Value;
            __result.combatRunSpeed *= speed;
            __result.combatWalkSpeed *= speed;
            __result.homeRunSpeed *= speed;
            __result.homeWalkSpeed *= speed;
            __result.groundAcc *= speed;
            __result.groundDec *= speed;
            __result.airAcc *= speed;
            __result.airDec *= speed;
            // 重力不变时，跳跃高度与起跳速度的平方成正比
            __result.jumpVelocity *= Mathf.Sqrt(s.JumpHeight.Value);
        }
    }

    /// <summary>
    /// 原版 HandleJump 里的 CanJump 是局部函数，只允许落地或土狼时间内起跳。
    /// 连跳开启时，在原方法前面处理空中起跳，其余情况交还原逻辑。
    /// </summary>
    [HarmonyPatch(typeof(PlayerController), "HandleJump")]
    internal static class MultiJumpPatch
    {
        private const ActionFlag BlockingFlags =
            ActionFlag.Interactive | ActionFlag.AutoMove | ActionFlag.Roll | ActionFlag.IsDying | ActionFlag.Dead;

        private static int airJumpsUsed;

        private static bool Prefix(
            PlayerController __instance,
            ref int ___jumpBufferFrameLeft,
            int ___jumpCoyoteTimeFrameLeft,
            EntityCollision ___collision,
            Player ___player,
            SoundLibrary ___soundLibrary)
        {
            TrainerSettings s = Plugin.Settings;
            if (!s.MultiJump.Value)
            {
                return true;
            }
            if (___collision.collisionInfo.below)
            {
                airJumpsUsed = 0;
                return true;
            }
            if (___jumpBufferFrameLeft <= 0)
            {
                return true;
            }

            ActionFlag flags = ___player.actionFlag;
            // 刚走下平台、还在土狼时间里的那一跳算地面跳，交给原逻辑
            if (!flags.Has(ActionFlag.Jumping) && ___jumpCoyoteTimeFrameLeft > 0)
            {
                return true;
            }
            if (airJumpsUsed >= s.ExtraJumps.Value || flags.Has(BlockingFlags) || ___player.playerType != Player.PlayerType.Combat)
            {
                return true;
            }

            airJumpsUsed++;
            __instance.acc.y = 0f;
            __instance.velocity.y = __instance.moveConfig.jumpVelocity;
            ___player.actionFlag.TurnOn(ActionFlag.Jumping);
            ___soundLibrary.Play("jumpUp");
            ___jumpBufferFrameLeft = 0;
            return false;
        }
    }
}
