using System;
using BepInEx.Configuration;
using CyanBrainTrainer.Shared;
using UnityEngine;
using UnityEngine.InputSystem;

namespace CyanBrainTrainer
{
    /// <summary>游戏内 IMGUI 菜单。</summary>
    internal sealed class TrainerMenu
    {
        private const int WindowId = 0x43425452;
        private const float WindowWidth = 470f;
        private const float WindowHeight = 770f;
        private const float LabelWidth = 128f;

        private static readonly Color Accent = new Color(0.30f, 0.88f, 0.86f);
        private static readonly Color TextColor = new Color(0.88f, 0.92f, 0.93f);
        private static readonly Color MutedColor = new Color(0.58f, 0.66f, 0.68f);

        private readonly TrainerSettings settings;
        private readonly TrainerRuntime runtime;
        private readonly Func<string> describeRemote;

        private bool visible;
        private bool pushedGameState;
        private bool menuKeyBroken;
        private Rect windowRect = new Rect(30f, 30f, WindowWidth, WindowHeight);
        private Vector2 scroll;
        private string toast;
        private bool toastIsError;
        private float toastUntil;

        private Texture2D windowTex;
        private Font font;
        private GUIStyle windowStyle;
        private GUIStyle titleStyle;
        private GUIStyle sectionStyle;
        private GUIStyle labelStyle;
        private GUIStyle valueStyle;
        private GUIStyle hintStyle;
        private GUIStyle toggleStyle;
        private GUIStyle buttonStyle;
        private GUIStyle smallButtonStyle;

        public TrainerMenu(TrainerSettings settings, TrainerRuntime runtime, Func<string> describeRemote)
        {
            this.settings = settings;
            this.runtime = runtime;
            this.describeRemote = describeRemote;
        }

        public void Update()
        {
            if (pushedGameState && !IsGameState(GameState.Console))
            {
                // 游戏自己切走了状态（比如换场景），不要再替它 Pop
                pushedGameState = false;
            }

            Keyboard keyboard = Keyboard.current;
            if (keyboard == null || menuKeyBroken)
            {
                return;
            }
            try
            {
                if (keyboard[settings.MenuKey.Value].wasPressedThisFrame)
                {
                    SetVisible(!visible);
                }
            }
            catch (ArgumentException)
            {
                menuKeyBroken = true;
                Plugin.Log.LogError($"菜单按键 {settings.MenuKey.Value} 无效，请在配置文件里改成别的键");
            }
        }

        private void SetVisible(bool show)
        {
            visible = show;
            if (show)
            {
                // 在关卡里打开时压一个 Console 状态：游戏会显示鼠标、隐藏准星、停止响应操作，
                // 点菜单时角色不会开枪。游戏自带的调试控制台也是这么做的。
                if (IsGameState(GameState.Gameplay))
                {
                    GameState.Push(GameState.Console);
                    pushedGameState = true;
                }
            }
            else
            {
                if (pushedGameState && IsGameState(GameState.Console))
                {
                    GameState.Pop();
                }
                pushedGameState = false;
                runtime.SaveNow();
            }
        }

        private static bool IsGameState(GameState.State state)
        {
            return GameState.Current != null && GameState.Is(state);
        }

        public void OnGUI()
        {
            if (!visible)
            {
                return;
            }
            EnsureStyles();

            Matrix4x4 oldMatrix = GUI.matrix;
            float scale = Mathf.Clamp(Screen.height / 1080f, 1f, 3f);
            GUI.matrix = Matrix4x4.Scale(new Vector3(scale, scale, 1f));

            float screenW = Screen.width / scale;
            float screenH = Screen.height / scale;
            windowRect.height = Mathf.Min(WindowHeight, screenH - 20f);
            windowRect.y = Mathf.Min(windowRect.y, screenH - windowRect.height - 10f);
            windowRect = GUI.Window(WindowId, windowRect, DrawWindow, GUIContent.none, windowStyle);
            windowRect.x = Mathf.Clamp(windowRect.x, 80f - windowRect.width, screenW - 80f);
            windowRect.y = Mathf.Clamp(windowRect.y, 0f, screenH - 40f);

            GUI.matrix = oldMatrix;
        }

        private void DrawWindow(int id)
        {
            GUILayout.BeginHorizontal();
            GUILayout.Label("青色大脑 修改器", titleStyle);
            GUILayout.FlexibleSpace();
            if (GUILayout.Button("✕", smallButtonStyle, GUILayout.Width(32f)))
            {
                SetVisible(false);
            }
            GUILayout.EndHorizontal();
            GUILayout.Label(DescribePlayer(), hintStyle);

            scroll = GUILayout.BeginScrollView(scroll, false, false);

            Section("生存");
            Toggle(settings.GodMode, "无敌（不掉血、不被处决）");
            Toggle(settings.NoInstantKill, "防秒杀（陷阱 / 地雷 / 掉坑 / 抓取处决 / 摔落）");
            Toggle(settings.LockHunger, "锁定饱食度");
            Toggle(settings.InfiniteEnergy, "无限能量（翻滚）");
            Toggle(settings.InfiniteFlashlight, "无限手电电量");

            Section("战斗");
            Toggle(settings.OneHitKill, "一击必杀");
            Slider(settings.DamageMultiplier, "伤害倍率");
            Toggle(settings.InfiniteAmmo, "无限子弹（不耗弹、不用换弹）");

            Section("移动");
            Toggle(settings.MultiJump, "连跳");
            IntSlider(settings.ExtraJumps, "空中额外跳跃");
            Slider(settings.JumpHeight, "跳跃高度");
            Slider(settings.MoveSpeed, "移动速度");
            Slider(settings.GameSpeed, "游戏速度");

            Section("经验");
            Slider(settings.ExpMultiplier, "经验倍率");

            Section("一键操作");
            GUILayout.BeginHorizontal();
            ActionButton("回满血", ActionKeys.Heal);
            ActionButton("补满药剂", ActionKeys.RefillMedicine);
            ActionButton("补满弹药", ActionKeys.RefillAmmo);
            GUILayout.EndHorizontal();
            GUILayout.BeginHorizontal();
            ActionButton("经验 +1000", ActionKeys.AddExp, "1000");
            ActionButton("经验 +10000", ActionKeys.AddExp, "10000");
            ActionButton("恢复默认设置", ActionKeys.ResetAll);
            GUILayout.EndHorizontal();

            GUILayout.EndScrollView();

            GUILayout.Space(4f);
            if (toast != null && Time.unscaledTime < toastUntil)
            {
                Color old = GUI.contentColor;
                GUI.contentColor = toastIsError ? new Color(1f, 0.55f, 0.5f) : Accent;
                GUILayout.Label(toast, labelStyle);
                GUI.contentColor = old;
            }
            GUILayout.Label(describeRemote(), hintStyle);
            GUILayout.Label($"{settings.MenuKey.Value} 开关菜单 · 拖动标题栏移动窗口", hintStyle);

            GUI.DragWindow(new Rect(0f, 0f, 10000f, 40f));
        }

        private static string DescribePlayer()
        {
            Player player = Engine.player;
            if (player == null)
            {
                return "未进入关卡";
            }
            Player.Status s = player.status;
            return $"血量 {s.hp:0}/{s.maxHp:0}    经验 {Mathf.FloorToInt(s.exp)}    药剂 {s.medicineCount}/{s.maxMedicineCount}";
        }

        private void Section(string title)
        {
            GUILayout.Space(10f);
            GUILayout.Label(title, sectionStyle);
        }

        private void Toggle(ConfigEntry<bool> entry, string label)
        {
            bool value = GUILayout.Toggle(entry.Value, " " + label, toggleStyle);
            if (value != entry.Value)
            {
                entry.Value = value;
            }
        }

        private void Slider(ConfigEntry<float> entry, string label)
        {
            AcceptableValueRange<float> range = (AcceptableValueRange<float>)entry.Description.AcceptableValues;
            float value = SliderRow(label, entry.Value, range.MinValue, range.MaxValue, entry.Value.ToString("0.0") + " x", (float)entry.DefaultValue);
            value = Mathf.Round(value * 10f) / 10f;
            if (!Mathf.Approximately(value, entry.Value))
            {
                entry.Value = value;
            }
        }

        private void IntSlider(ConfigEntry<int> entry, string label)
        {
            AcceptableValueRange<int> range = (AcceptableValueRange<int>)entry.Description.AcceptableValues;
            int value = Mathf.RoundToInt(SliderRow(label, entry.Value, range.MinValue, range.MaxValue, entry.Value + " 次", (int)entry.DefaultValue));
            if (value != entry.Value)
            {
                entry.Value = value;
            }
        }

        private float SliderRow(string label, float value, float min, float max, string display, float defaultValue)
        {
            GUILayout.BeginHorizontal();
            GUILayout.Label(label, labelStyle, GUILayout.Width(LabelWidth));
            GUILayout.BeginVertical();
            GUILayout.Space(9f);
            value = GUILayout.HorizontalSlider(value, min, max);
            GUILayout.EndVertical();
            GUILayout.Label(display, valueStyle, GUILayout.Width(58f));
            if (GUILayout.Button("↺", smallButtonStyle, GUILayout.Width(30f)))
            {
                value = defaultValue;
            }
            GUILayout.EndHorizontal();
            return value;
        }

        private void ActionButton(string text, string action, string arg = null)
        {
            if (GUILayout.Button(text, buttonStyle))
            {
                bool ok = runtime.Execute(action, arg, out string message);
                toast = message;
                toastIsError = !ok;
                toastUntil = Time.unscaledTime + 3f;
            }
        }

        private void EnsureStyles()
        {
            // 换场景时运行期创建的贴图可能被卸载，丢了就重建
            if (windowStyle != null && windowTex != null && font != null)
            {
                return;
            }

            font = Font.CreateDynamicFontFromOSFont(new[] { "Microsoft YaHei UI", "Microsoft YaHei", "SimHei", "Arial" }, 16);
            font.hideFlags = HideFlags.HideAndDontSave;
            windowTex = MakeTexture(new Color(0.06f, 0.08f, 0.10f, 0.95f));
            Texture2D buttonTex = MakeTexture(new Color(0.16f, 0.21f, 0.24f, 1f));
            Texture2D buttonHoverTex = MakeTexture(new Color(0.20f, 0.30f, 0.33f, 1f));
            Texture2D buttonActiveTex = MakeTexture(new Color(0.18f, 0.55f, 0.55f, 1f));

            windowStyle = new GUIStyle(GUI.skin.window)
            {
                padding = new RectOffset(16, 16, 12, 12),
                border = new RectOffset(0, 0, 0, 0)
            };
            windowStyle.normal.background = windowTex;
            windowStyle.onNormal.background = windowTex;

            // 动态字体合成粗体时字形上沿会被裁掉，标题统一用常规字重
            titleStyle = TextStyle(GUI.skin.label, 21, Accent);
            sectionStyle = TextStyle(GUI.skin.label, 17, Accent);
            labelStyle = TextStyle(GUI.skin.label, 15, TextColor);
            valueStyle = TextStyle(GUI.skin.label, 15, Accent);
            valueStyle.alignment = TextAnchor.MiddleRight;
            hintStyle = TextStyle(GUI.skin.label, 13, MutedColor);
            hintStyle.wordWrap = true;

            toggleStyle = TextStyle(GUI.skin.toggle, 15, TextColor);
            toggleStyle.onNormal.textColor = Accent;
            toggleStyle.onHover.textColor = Accent;
            toggleStyle.hover.textColor = Color.white;
            toggleStyle.margin = new RectOffset(4, 4, 5, 5);

            buttonStyle = TextStyle(GUI.skin.button, 15, TextColor);
            buttonStyle.normal.background = buttonTex;
            buttonStyle.hover.background = buttonHoverTex;
            buttonStyle.active.background = buttonActiveTex;
            buttonStyle.hover.textColor = Color.white;
            buttonStyle.active.textColor = Color.white;
            buttonStyle.border = new RectOffset(0, 0, 0, 0);
            buttonStyle.padding = new RectOffset(8, 8, 7, 7);
            buttonStyle.margin = new RectOffset(3, 3, 4, 4);

            smallButtonStyle = new GUIStyle(buttonStyle) { padding = new RectOffset(2, 2, 3, 3) };
        }

        private GUIStyle TextStyle(GUIStyle baseStyle, int size, Color color)
        {
            GUIStyle style = new GUIStyle(baseStyle) { font = font, fontSize = size, wordWrap = false };
            style.normal.textColor = color;
            return style;
        }

        private static Texture2D MakeTexture(Color color)
        {
            Texture2D tex = new Texture2D(1, 1, TextureFormat.RGBA32, false) { hideFlags = HideFlags.HideAndDontSave };
            tex.SetPixel(0, 0, color);
            tex.Apply();
            return tex;
        }
    }
}
