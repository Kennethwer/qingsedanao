// 插件与外部控制程序的通信协议（本机 TCP，按行收发，UTF-8）。
// 控制程序（Rust）那边的对应定义在 src/trainer-app/src/protocol.rs，改动时两边一起改。
//
//   客户端 -> 插件                 插件 -> 客户端
//   GET                            STATE key=value;key=value;...
//   SET <key> <value>              OK | ERR <原因>
//   DO <action> [arg]              OK | ERR <原因>
//   PING                           PONG
//
// 连接建立后插件先发一行 "HELLO <版本>"。数值一律用 InvariantCulture。
namespace CyanBrainTrainer.Shared
{
    public static class Protocol
    {
        public const int DefaultPort = 37421;

        public const string Hello = "HELLO";
        public const string Get = "GET";
        public const string Set = "SET";
        public const string Do = "DO";
        public const string Ping = "PING";
        public const string Pong = "PONG";
        public const string State = "STATE";
        public const string Ok = "OK";
        public const string Err = "ERR";
    }

    /// <summary>可读写的开关/数值，同时也是 BepInEx 配置文件里的键名。</summary>
    public static class SettingKeys
    {
        public const string GodMode = "godMode";
        public const string NoInstantKill = "noInstantKill";
        public const string LockHunger = "lockHunger";
        public const string InfiniteEnergy = "infiniteEnergy";
        public const string InfiniteFlashlight = "infiniteFlashlight";

        public const string OneHitKill = "oneHitKill";
        public const string DamageMultiplier = "damageMultiplier";
        public const string InfiniteAmmo = "infiniteAmmo";

        public const string MultiJump = "multiJump";
        public const string ExtraJumps = "extraJumps";
        public const string JumpHeight = "jumpHeight";
        public const string MoveSpeed = "moveSpeed";
        public const string GameSpeed = "gameSpeed";

        public const string ExpMultiplier = "expMultiplier";
    }

    /// <summary>一次性操作。</summary>
    public static class ActionKeys
    {
        public const string Heal = "heal";
        public const string RefillMedicine = "refillMedicine";
        public const string RefillAmmo = "refillAmmo";
        public const string AddExp = "addExp";
        public const string ResetAll = "resetAll";
    }

    /// <summary>STATE 里除设置项以外的只读字段。</summary>
    public static class StateKeys
    {
        public const string InGame = "inGame";
        public const string Scene = "scene";
        public const string Hp = "hp";
        public const string MaxHp = "maxHp";
        public const string Exp = "exp";
        public const string Energy = "energy";
        public const string Medicine = "medicine";
        public const string MaxMedicine = "maxMedicine";
    }
}
