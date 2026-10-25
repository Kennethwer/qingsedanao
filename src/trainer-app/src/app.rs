use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, CentralPanel, Frame, Id, LayerId, Layout, Margin, Order, Panel, RichText, ScrollArea, Sense, Stroke,
    Ui, ViewportCommand, pos2, vec2,
};

use crate::client::{Client, Snapshot};
use crate::config::AppConfig;
use crate::installer::{self, InstallStatus, PluginState};
use crate::protocol::{self, ACTIONS, Kind, LEFT_SECTIONS, RIGHT_SECTIONS, Section, state};
use crate::theme::{self, ButtonKind, CYAN, DIGITS, GREEN, LINE, LINE_DIM, RED, TEXT, TEXT_DIM, TEXT_OFF};

const STATUS_INTERVAL: Duration = Duration::from_millis(1500);
/// 本地改过的值在这段时间内优先于游戏回传的值，避免拖滑条时来回跳
const OVERRIDE_TTL: Duration = Duration::from_millis(1200);
const HEADER_HEIGHT: f32 = 96.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Trainer,
    Install,
    Help,
}

struct Toast {
    text: String,
    is_error: bool,
    until: Instant,
}

pub struct TrainerApp {
    page: Page,
    client: Client,
    config: AppConfig,
    install: InstallStatus,
    game_running: bool,
    next_status_check: Instant,
    overrides: HashMap<&'static str, (String, Instant)>,
    toast: Option<Toast>,
    confirm_uninstall_until: Option<Instant>,
}

impl TrainerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        theme::apply_style(&cc.egui_ctx);

        let mut config = AppConfig::load();
        if !config.game_dir.as_deref().is_some_and(installer::is_game_dir) {
            config.game_dir = installer::detect_game_dir();
            if config.game_dir.is_some() {
                config.save();
            }
        }
        let install = installer::status(config.game_dir.as_deref());
        let page = if install.plugin == PluginState::Missing { Page::Install } else { Page::Trainer };
        let client = Client::start(config.port, cc.egui_ctx.clone());

        TrainerApp {
            page,
            client,
            config,
            install,
            game_running: installer::is_game_running(),
            next_status_check: Instant::now() + STATUS_INTERVAL,
            overrides: HashMap::new(),
            toast: None,
            confirm_uninstall_until: None,
        }
    }

    fn refresh_status(&mut self) {
        self.install = installer::status(self.config.game_dir.as_deref());
        self.game_running = installer::is_game_running();
        self.next_status_check = Instant::now() + STATUS_INTERVAL;
    }

    fn toast(&mut self, text: impl Into<String>, is_error: bool) {
        self.toast = Some(Toast { text: text.into(), is_error, until: Instant::now() + Duration::from_secs(5) });
    }

    fn value(&self, snap: &Snapshot, key: &str) -> Option<String> {
        match self.overrides.get(key) {
            Some((value, at)) if at.elapsed() < OVERRIDE_TTL => Some(value.clone()),
            _ => snap.state.get(key).cloned(),
        }
    }

    fn set_value(&mut self, key: &'static str, value: String) {
        self.client.set(key, &value);
        self.overrides.insert(key, (value, Instant::now()));
    }

    fn set_game_dir(&mut self, dir: PathBuf) {
        self.config.game_dir = Some(dir);
        self.config.save();
        self.refresh_status();
    }

    fn launch_game(&mut self) {
        let Some(dir) = self.config.game_dir.clone() else {
            self.page = Page::Install;
            self.toast("还没有设置游戏目录", true);
            return;
        };
        if self.game_running {
            self.toast("游戏已经在运行", false);
            return;
        }
        match installer::launch_game(&dir) {
            Ok(()) if self.install.plugin == PluginState::Missing => self.toast("已启动游戏。修改器还没安装，这次是原版", true),
            Ok(()) => self.toast("正在启动游戏，加载完成后会自动连接", false),
            Err(e) => self.toast(e, true),
        }
        self.next_status_check = Instant::now() + Duration::from_millis(500);
    }

    // ---- 外框 ----

    fn header(&mut self, ui: &mut Ui, snap: &Snapshot) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("CYAN BRAIN").font(theme::font(34.0, theme::LOGO)).color(TEXT));
                ui.label(RichText::new("シ ア ン ブ レ イ ン　·　修 改 器").size(12.0).color(TEXT_DIM));
            });
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if theme::window_button(ui, true).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
                if theme::window_button(ui, false).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                ui.add_space(14.0);
                let (text, color) = if snap.connected {
                    (format!("■ 已连接游戏 · 插件 v{}", snap.plugin_version), GREEN)
                } else if self.game_running {
                    ("■ 游戏运行中，等待插件连接…".to_owned(), CYAN)
                } else {
                    ("■ 未连接".to_owned(), TEXT_OFF)
                };
                // 和窗口按钮同高，文字垂直居中
                ui.allocate_ui_with_layout(vec2(360.0, 28.0), Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(text).size(13.0).color(color));
                });
            });
        });
    }

    fn nav(&mut self, ui: &mut Ui) {
        for (page, title, subtitle) in [
            (Page::Trainer, "修改", "TRAINER"),
            (Page::Install, "安装", "INSTALL"),
            (Page::Help, "说明", "GUIDE"),
        ] {
            if theme::nav_item(ui, title, subtitle, self.page == page).clicked() {
                self.page = page;
            }
            ui.add_space(8.0);
        }
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            if theme::button(ui, "启动游戏", ButtonKind::Primary).clicked() {
                self.launch_game();
            }
        });
    }

    fn footer(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            match &self.toast {
                Some(toast) if toast.until > Instant::now() => {
                    ui.label(RichText::new(&toast.text).size(13.0).color(if toast.is_error { RED } else { CYAN }));
                }
                _ => {
                    ui.label(RichText::new("游戏内按 F8 也能打开同样的菜单，两边设置实时同步").size(13.0).color(TEXT_OFF));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("TRAINER v{}", env!("CARGO_PKG_VERSION")))
                        .font(theme::font(13.0, DIGITS))
                        .color(TEXT_OFF),
                );
            });
        });
    }

    /// 沿着各面板的实际边界画框线和交点方块，像游戏主菜单那样。
    fn paint_chrome(&self, ui: &Ui, header: egui::Rect, nav: egui::Rect, footer: egui::Rect) {
        let painter = ui.ctx().layer_painter(LayerId::new(Order::Foreground, Id::new("chrome")));
        let outer = ui.max_rect().shrink(6.0);
        let stroke = Stroke::new(1.0, LINE);
        theme::frame_with_marks(&painter, outer, LINE, 6.0);

        let header_y = header.bottom();
        let footer_y = footer.top();
        let nav_x = nav.right();
        painter.line_segment([pos2(outer.left(), header_y), pos2(outer.right(), header_y)], stroke);
        painter.line_segment([pos2(outer.left(), footer_y), pos2(outer.right(), footer_y)], stroke);
        painter.line_segment([pos2(nav_x, header_y), pos2(nav_x, footer_y)], stroke);
        for point in [
            pos2(outer.left(), header_y),
            pos2(outer.right(), header_y),
            pos2(outer.left(), footer_y),
            pos2(outer.right(), footer_y),
            pos2(nav_x, header_y),
            pos2(nav_x, footer_y),
        ] {
            theme::mark(&painter, point, 6.0);
        }
    }

    // ---- 修改页 ----

    fn trainer_page(&mut self, ui: &mut Ui, snap: &Snapshot) {
        let in_game = snap.state.get(state::IN_GAME).is_some_and(|v| v == "1");

        if snap.connected {
            ui.label(RichText::new(describe_player(snap, in_game)).size(14.0).color(TEXT_DIM));
        } else {
            self.disconnected_hint(ui);
        }
        ui.add_space(6.0);

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_enabled_ui(snap.connected, |ui| {
                ui.columns(2, |columns| {
                    self.sections(&mut columns[0], LEFT_SECTIONS, snap);
                    self.sections(&mut columns[1], RIGHT_SECTIONS, snap);
                });
                theme::section_title(ui, "一键操作");
                ui.horizontal_wrapped(|ui| {
                    for action in ACTIONS {
                        ui.add_enabled_ui(!action.needs_player || in_game, |ui| {
                            if theme::button(ui, action.label, ButtonKind::Normal).clicked() {
                                self.client.action(action.command, action.label);
                            }
                        });
                    }
                });
                if !in_game {
                    ui.label(RichText::new("回血、补给、加经验需要先进入关卡").size(13.0).color(TEXT_OFF));
                }
            });
        });
    }

    fn disconnected_hint(&mut self, ui: &mut Ui) {
        Frame::new().stroke(Stroke::new(1.0, LINE_DIM)).inner_margin(Margin::symmetric(14, 10)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // 先定好行高，文字才能和右边的按钮垂直居中
                ui.set_min_height(38.0);
                let (text, button) = if self.install.plugin == PluginState::Missing {
                    ("修改器还没装进游戏，请先到「安装」页安装", Some("去安装"))
                } else if self.game_running {
                    ("游戏正在运行，插件加载完成后会自动连接…", None)
                } else {
                    ("游戏没有运行。启动游戏后会自动连接", Some("启动游戏"))
                };
                ui.label(RichText::new(text).size(14.0).color(TEXT));
                if let Some(label) = button {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::button(ui, label, ButtonKind::Primary).clicked() {
                            if self.install.plugin == PluginState::Missing {
                                self.page = Page::Install;
                            } else {
                                self.launch_game();
                            }
                        }
                    });
                }
            });
        });
    }

    fn sections(&mut self, ui: &mut Ui, sections: &'static [Section], snap: &Snapshot) {
        for section in sections {
            theme::section_title(ui, section.title);
            for setting in section.settings {
                let current = self.value(snap, setting.key);
                match &setting.kind {
                    Kind::Toggle => {
                        let on = current.as_deref() == Some("1");
                        if theme::toggle(ui, on, setting.label, setting.hint) {
                            self.set_value(setting.key, if on { "0" } else { "1" }.to_owned());
                        }
                    }
                    &Kind::Slider { min, max, step, default, unit } => {
                        let spec = theme::SliderSpec { min, max, step, default, unit };
                        let value = current.and_then(|v| v.parse().ok()).unwrap_or(default);
                        if let Some(new_value) = theme::slider(ui, Id::new(setting.key), setting.label, value, &spec) {
                            self.set_value(setting.key, theme::format_value(new_value, step));
                        }
                    }
                }
            }
            ui.add_space(12.0);
        }
    }

    // ---- 安装页 ----

    fn install_page(&mut self, ui: &mut Ui) {
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            theme::section_title(ui, "游戏目录");
            let dir_text = match &self.config.game_dir {
                Some(dir) => dir.display().to_string(),
                None => "没有找到游戏，请手动选择包含 CYANBRAIN.exe 的文件夹".to_owned(),
            };
            ui.label(RichText::new(dir_text).size(14.0).color(if self.install.game_found { TEXT } else { RED }));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if theme::button(ui, "选择目录…", ButtonKind::Normal).clicked() {
                    self.pick_game_dir();
                }
                if theme::button(ui, "自动查找", ButtonKind::Normal).clicked() {
                    match installer::detect_game_dir() {
                        Some(dir) => {
                            self.set_game_dir(dir);
                            self.toast("已找到游戏目录", false);
                        }
                        None => self.toast("没有自动找到游戏，请手动选择目录", true),
                    }
                }
                ui.add_enabled_ui(self.install.game_found, |ui| {
                    if theme::button(ui, "打开目录", ButtonKind::Normal).clicked()
                        && let Some(dir) = &self.config.game_dir {
                            installer::open_in_explorer(dir);
                        }
                });
            });
            ui.add_space(14.0);

            theme::section_title(ui, "当前状态");
            let s = self.install;
            let found = |ok: bool, yes: &str, no: &str| if ok { (yes.to_owned(), GREEN) } else { (no.to_owned(), RED) };
            let (text, color) = found(s.game_found, "已找到", "未找到");
            theme::status_row(ui, "游戏程序", &text, color);
            let (text, color) = found(s.loader_installed, "已安装 BepInEx 5", "未安装");
            theme::status_row(ui, "模组加载器", &text, color);
            let (text, color) = match s.plugin {
                PluginState::UpToDate => ("已安装，是最新版".to_owned(), GREEN),
                PluginState::Outdated => ("已安装，但不是这个版本，建议更新".to_owned(), CYAN),
                PluginState::Missing => ("未安装".to_owned(), RED),
            };
            theme::status_row(ui, "修改器插件", &text, color);
            let (text, color) = if self.game_running { ("运行中".to_owned(), CYAN) } else { ("未运行".to_owned(), TEXT_DIM) };
            theme::status_row(ui, "游戏进程", &text, color);
            ui.add_space(14.0);

            theme::section_title(ui, "安装 / 卸载");
            let can_modify = s.game_found && !self.game_running;
            ui.horizontal_wrapped(|ui| {
                ui.add_enabled_ui(can_modify, |ui| {
                    let label = match s.plugin {
                        PluginState::Missing => "安装修改器",
                        PluginState::Outdated => "更新修改器",
                        PluginState::UpToDate => "重新安装",
                    };
                    if theme::button(ui, label, ButtonKind::Primary).clicked() {
                        self.run_install();
                    }
                    let confirming = self.confirm_uninstall_until.is_some_and(|t| t > Instant::now());
                    ui.add_enabled_ui(s.plugin != PluginState::Missing || s.loader_installed, |ui| {
                        if theme::button(ui, if confirming { "再点一次确认卸载" } else { "卸载" }, ButtonKind::Danger).clicked() {
                            if confirming {
                                self.confirm_uninstall_until = None;
                                self.run_uninstall();
                            } else {
                                self.confirm_uninstall_until = Some(Instant::now() + Duration::from_secs(4));
                            }
                        }
                    });
                });
                if theme::button(ui, "启动游戏", ButtonKind::Normal).clicked() {
                    self.launch_game();
                }
            });
            let note = if self.game_running {
                "游戏正在运行：安装、卸载前请先关闭游戏"
            } else {
                "卸载时如果没有别的 BepInEx 模组，会把 BepInEx 一起删掉，游戏恢复原版"
            };
            ui.label(RichText::new(note).size(13.0).color(if self.game_running { CYAN } else { TEXT_OFF }));
            ui.add_space(14.0);

            theme::section_title(ui, "存档");
            let save_dir = installer::save_dir();
            ui.label(
                RichText::new(save_dir.as_ref().map(|d| d.display().to_string()).unwrap_or_default())
                    .size(14.0)
                    .color(TEXT_DIM),
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if theme::button(ui, "备份存档", ButtonKind::Normal).clicked() {
                    match installer::backup_saves() {
                        Ok(dst) => self.toast(format!("存档已备份到 {}", dst.display()), false),
                        Err(e) => self.toast(e, true),
                    }
                }
                if theme::button(ui, "打开存档目录", ButtonKind::Normal).clicked()
                    && let Some(dir) = &save_dir {
                        installer::open_in_explorer(dir);
                    }
            });
        });
    }

    fn pick_game_dir(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("选择青色大脑的游戏目录（包含 CYANBRAIN.exe 的文件夹）");
        if let Some(dir) = &self.config.game_dir {
            dialog = dialog.set_directory(dir);
        }
        let Some(dir) = dialog.pick_folder() else { return };
        if installer::is_game_dir(&dir) {
            self.set_game_dir(dir);
            self.toast("游戏目录已设置", false);
        } else {
            self.toast(format!("{} 里没有 {}", dir.display(), installer::GAME_EXE), true);
        }
    }

    fn run_install(&mut self) {
        let Some(dir) = self.config.game_dir.clone() else { return };
        match installer::install(&dir) {
            Ok(message) => self.toast(message, false),
            Err(e) => self.toast(e, true),
        }
        self.refresh_status();
    }

    fn run_uninstall(&mut self) {
        let Some(dir) = self.config.game_dir.clone() else { return };
        match installer::uninstall(&dir) {
            Ok(message) => self.toast(message, false),
            Err(e) => self.toast(e, true),
        }
        self.refresh_status();
    }

    // ---- 说明页 ----

    fn help_page(&mut self, ui: &mut Ui) {
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let paragraph = |ui: &mut Ui, text: &str| {
                ui.label(RichText::new(text).size(14.0).color(TEXT_DIM));
            };
            theme::section_title(ui, "使用步骤");
            paragraph(ui, "1. 在「安装」页确认游戏目录，点「安装修改器」。");
            paragraph(ui, "2. 启动游戏，本软件会自动连接，在「修改」页勾选功能即可，实时生效。");
            paragraph(ui, "3. 游戏里按 F8 也能打开同样的菜单，两边的设置是同一份，会保存到下次。");
            ui.add_space(10.0);

            theme::section_title(ui, "功能说明");
            for section in LEFT_SECTIONS.iter().chain(RIGHT_SECTIONS) {
                for setting in section.settings {
                    if !setting.hint.is_empty() {
                        paragraph(ui, &format!("{}：{}", setting.label, setting.hint));
                    }
                }
            }
            paragraph(ui, "伤害 / 经验 / 跳跃 / 移动 / 游戏速度：拖动滑条调整倍率，点「默认」恢复。");
            ui.add_space(10.0);

            theme::section_title(ui, "注意");
            paragraph(ui, "· 修改经验等数值会被游戏写进存档，建议先在「安装」页备份存档。");
            paragraph(ui, "· 游戏里残留的开发者调试代码按 F1 会重置 Steam 成就，修改器菜单用的是 F8。");
            paragraph(ui, "· 游戏更新后如果某个功能失效，重新安装一次修改器；仍然失效说明游戏代码改了，需要更新修改器。");
            paragraph(ui, &format!("· 本软件通过本机端口 127.0.0.1:{} 与游戏通信，不联网。", protocol::DEFAULT_PORT));
        });
    }
}

fn describe_player(snap: &Snapshot, in_game: bool) -> String {
    let get = |key: &str| snap.state.get(key).map(String::as_str).unwrap_or("?");
    let scene = get(state::SCENE);
    if !in_game {
        return format!("当前：{scene}（还没进入关卡）");
    }
    let number = |key: &str| get(key).parse::<f32>().map(|v| format!("{v:.0}")).unwrap_or_else(|_| "?".into());
    format!(
        "场景 {scene}　　血量 {}/{}　　经验 {}　　药剂 {}/{}",
        number(state::HP),
        number(state::MAX_HP),
        get(state::EXP),
        get(state::MEDICINE),
        get(state::MAX_MEDICINE)
    )
}

impl eframe::App for TrainerApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for notice in self.client.take_notices() {
            self.toast(notice.text, notice.is_error);
        }
        if Instant::now() >= self.next_status_check {
            self.refresh_status();
        }
        // 状态检测、提示消失都需要定期刷新
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let snap = self.client.snapshot();
        let panel_frame = |margin: Margin| Frame::new().fill(theme::BG).inner_margin(margin);

        // 无边框窗口：窗口顶部标题区可以拖动。先于所有控件注册，标题区里的按钮后注册、点击优先
        let window = ui.max_rect();
        let title_bar = egui::Rect::from_min_max(window.min, pos2(window.max.x, window.min.y + HEADER_HEIGHT));
        if ui.interact(title_bar, Id::new("drag-window"), Sense::drag()).drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }

        let header = Panel::top("header")
            .exact_size(HEADER_HEIGHT)
            .show_separator_line(false)
            .frame(panel_frame(Margin { left: 30, right: 14, top: 14, bottom: 8 }))
            .show(ui, |ui| self.header(ui, &snap));
        let footer = Panel::bottom("footer")
            .exact_size(36.0)
            .show_separator_line(false)
            .frame(panel_frame(Margin::symmetric(30, 4)))
            .show(ui, |ui| self.footer(ui));
        let nav = Panel::left("nav")
            .exact_size(210.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(panel_frame(Margin { left: 22, right: 18, top: 22, bottom: 18 }))
            .show(ui, |ui| self.nav(ui));
        CentralPanel::no_frame()
            .frame(panel_frame(Margin { left: 30, right: 26, top: 20, bottom: 12 }))
            .show(ui, |ui| match self.page {
                Page::Trainer => self.trainer_page(ui, &snap),
                Page::Install => self.install_page(ui),
                Page::Help => self.help_page(ui),
            });

        self.paint_chrome(ui, header.response.rect, nav.response.rect, footer.response.rect);
    }
}
