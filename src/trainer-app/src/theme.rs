//! 视觉风格：照着游戏主菜单来——纯黑底、灰蓝细框线、白色方形角点、青色点缀。
//! 这里的控件只负责画自己并按文字实际尺寸申请空间，摆放交给 egui 的布局。

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, IconData, Id,
    Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, TextStyle, Ui, pos2, vec2,
};

pub const BG: Color32 = Color32::from_rgb(0, 0, 0);
pub const HOVER: Color32 = Color32::from_rgb(16, 21, 25);
/// 游戏菜单边框的灰蓝色
pub const LINE: Color32 = Color32::from_rgb(138, 156, 170);
pub const LINE_DIM: Color32 = Color32::from_rgb(48, 58, 66);
pub const TEXT: Color32 = Color32::from_rgb(242, 244, 245);
pub const TEXT_DIM: Color32 = Color32::from_rgb(150, 162, 170);
pub const TEXT_OFF: Color32 = Color32::from_rgb(78, 88, 95);
pub const CYAN: Color32 = Color32::from_rgb(92, 214, 230);
pub const RED: Color32 = Color32::from_rgb(232, 104, 94);
pub const GREEN: Color32 = Color32::from_rgb(108, 214, 148);

/// 粗体中文（标题用）
pub const BOLD: &str = "bold";
/// 粗黑体西文（Logo 用）
pub const LOGO: &str = "logo";
/// 窄体西文数字（数值、版本号用，类似游戏里 "Steam v1.1.3" 那种字）
pub const DIGITS: &str = "digits";

pub fn font(size: f32, family: &str) -> FontId {
    FontId::new(size, FontFamily::Name(family.into()))
}

// ---- 字体与全局样式 ----

/// 从系统字体目录加载：中文用微软雅黑（没有就黑体），Logo 用 Arial Black，数字用 Bahnschrift。
/// 每一族都把中文字体接在后面兜底，缺哪个都不会出现方块字。
pub fn install_fonts(ctx: &Context) {
    let dir = PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into())).join("Fonts");
    let mut fonts = FontDefinitions::default();

    // .ttc 的第 2 个字形是 "Microsoft YaHei UI"，比正文版紧凑
    let mut load = |name: &str, candidates: &[(&str, u32)]| -> Option<String> {
        let (bytes, index) = candidates.iter().find_map(|(file, index)| fs::read(dir.join(file)).ok().map(|b| (b, *index)))?;
        let mut data = FontData::from_owned(bytes);
        data.index = index;
        fonts.font_data.insert(name.to_owned(), Arc::new(data));
        Some(name.to_owned())
    };
    let cjk = load("cjk", &[("msyh.ttc", 1), ("simhei.ttf", 0)]);
    let cjk_bold = load("cjk-bold", &[("msyhbd.ttc", 1), ("simhei.ttf", 0)]);
    let logo = load("latin-black", &[("ariblk.ttf", 0), ("segoeuib.ttf", 0)]);
    let digits = load("latin-narrow", &[("bahnschrift.ttf", 0), ("segoeui.ttf", 0)]);

    if let Some(cjk) = &cjk {
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts.families.entry(family).or_default().insert(0, cjk.clone());
        }
    }
    let proportional = fonts.families[&FontFamily::Proportional].clone();
    for (family, primary) in [(BOLD, cjk_bold), (LOGO, logo), (DIGITS, digits)] {
        let mut chain: Vec<String> = primary.into_iter().collect();
        chain.extend(proportional.iter().cloned());
        fonts.families.insert(FontFamily::Name(family.into()), chain);
    }
    ctx.set_fonts(fonts);
}

pub fn apply_style(ctx: &Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(13.0)),
            (TextStyle::Body, FontId::proportional(15.0)),
            (TextStyle::Button, FontId::proportional(15.0)),
            (TextStyle::Heading, font(20.0, BOLD)),
            (TextStyle::Monospace, FontId::monospace(14.0)),
        ]
        .into();
        style.spacing.item_spacing = vec2(8.0, 6.0);
        style.interaction.selectable_labels = false;

        let v = &mut style.visuals;
        v.panel_fill = BG;
        v.window_fill = Color32::from_rgb(8, 11, 13);
        v.window_stroke = Stroke::new(1.0, LINE);
        v.extreme_bg_color = BG;
        v.faint_bg_color = HOVER;
        v.override_text_color = Some(TEXT);
        v.selection.bg_fill = Color32::from_rgb(24, 80, 88);
        v.selection.stroke = Stroke::new(1.0, CYAN);
        v.window_corner_radius = CornerRadius::ZERO;
        // 滚动条等内置控件
        let w = &mut v.widgets;
        w.inactive.bg_fill = LINE_DIM;
        w.inactive.weak_bg_fill = LINE_DIM;
        w.hovered.bg_fill = LINE;
        w.hovered.weak_bg_fill = LINE;
        w.active.bg_fill = CYAN;
        w.active.weak_bg_fill = CYAN;
        for state in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            state.corner_radius = CornerRadius::ZERO;
        }
    });
}

/// 窗口图标：黑底、灰蓝边框、白色角点、青色方块。
pub fn window_icon() -> IconData {
    const SIZE: u32 = 64;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let near_edge = |v: u32, width: u32| v < width || v >= SIZE - width;
            let color = if near_edge(x, 8) && near_edge(y, 8) {
                TEXT
            } else if near_edge(x, 3) || near_edge(y, 3) {
                LINE
            } else if (18..46).contains(&x) && (18..46).contains(&y) {
                CYAN
            } else {
                BG
            };
            rgba.extend_from_slice(&color.to_array());
        }
    }
    IconData { rgba, width: SIZE, height: SIZE }
}

// ---- 装饰 ----

/// 游戏菜单框线交点上的白色小方块
pub fn mark(painter: &Painter, center: Pos2, size: f32) {
    painter.rect_filled(Rect::from_center_size(center, vec2(size, size)), 0.0, TEXT);
}

pub fn frame_with_marks(painter: &Painter, rect: Rect, stroke: Color32, mark_size: f32) {
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, stroke), StrokeKind::Middle);
    for corner in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()] {
        mark(painter, corner, mark_size);
    }
}

/// 分区标题：青色粗字 + 延伸到右边的细线
pub fn section_title(ui: &mut Ui, text: &str) {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(16.0, BOLD), CYAN);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y + 10.0), Sense::hover());
    let y = rect.center().y;
    let text_pos = pos2(rect.left(), y - galley.size().y / 2.0);
    let line_start = text_pos.x + galley.size().x + 12.0;
    let painter = ui.painter();
    painter.galley(text_pos, galley, CYAN);
    if line_start < rect.right() {
        painter.line_segment([pos2(line_start, y), pos2(rect.right(), y)], Stroke::new(1.0, LINE_DIM));
        painter.rect_filled(Rect::from_center_size(pos2(rect.right() - 2.0, y), vec2(4.0, 4.0)), 0.0, LINE);
    }
}

// ---- 控件 ----

/// 开关行。返回是否被点击。
pub fn toggle(ui: &mut Ui, on: bool, label: &str, hint: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    let enabled = ui.is_enabled();
    let hovered = enabled && response.hovered();
    let painter = ui.painter();

    if hovered {
        painter.rect_filled(rect, 0.0, HOVER);
    }
    let box_rect = Rect::from_center_size(pos2(rect.left() + 13.0, rect.center().y), vec2(14.0, 14.0));
    let border = if !enabled { LINE_DIM } else if on || hovered { TEXT } else { LINE };
    painter.rect_stroke(box_rect, 0.0, Stroke::new(1.0, border), StrokeKind::Inside);
    if on {
        painter.rect_filled(box_rect.shrink(3.0), 0.0, if enabled { CYAN } else { TEXT_OFF });
    }
    let text_color = if !enabled { TEXT_OFF } else if on || hovered { TEXT } else { TEXT_DIM };
    painter.text(pos2(box_rect.right() + 12.0, rect.center().y), Align2::LEFT_CENTER, label, FontId::proportional(15.0), text_color);
    let state_color = if on && enabled { CYAN } else { TEXT_OFF };
    painter.text(pos2(rect.right() - 8.0, rect.center().y), Align2::RIGHT_CENTER, if on { "ON" } else { "OFF" }, font(13.0, DIGITS), state_color);

    let clicked = response.clicked();
    if !hint.is_empty() {
        response.on_hover_text(hint);
    }
    clicked
}

pub struct SliderSpec {
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub default: f32,
    pub unit: &'static str,
}

pub fn format_value(value: f32, step: f32) -> String {
    if step >= 1.0 { format!("{}", value.round() as i32) } else { format!("{value:.1}") }
}

/// 滑条行：标签 | 轨道 | 数值 | 默认。返回新值（没变就是 None）。
pub fn slider(ui: &mut Ui, id: Id, label: &str, value: f32, spec: &SliderSpec) -> Option<f32> {
    let enabled = ui.is_enabled();
    let label_galley = ui.painter().layout_no_wrap(label.to_owned(), FontId::proportional(15.0), TEXT);
    let reset_galley = ui.painter().layout_no_wrap("默认".to_owned(), FontId::proportional(12.0), TEXT);
    let value_text = format!("{}{}", format_value(value, spec.step), spec.unit);
    let value_galley = ui.painter().layout_no_wrap(value_text, font(15.0, DIGITS), CYAN);

    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    let y = rect.center().y;
    let label_width = label_galley.size().x.max(60.0) + 16.0;
    let value_width = value_galley.size().x.max(40.0) + 10.0;
    let reset_rect = Rect::from_min_size(
        pos2(rect.right() - reset_galley.size().x - 14.0, y - 11.0),
        vec2(reset_galley.size().x + 14.0, 22.0),
    );
    let track_left = rect.left() + label_width;
    let track_right = (reset_rect.left() - value_width - 8.0).max(track_left + 20.0);
    let track = Rect::from_min_max(pos2(track_left, y - 9.0), pos2(track_right, y + 9.0));

    let track_response = ui.interact(track.expand2(vec2(6.0, 4.0)), id, Sense::click_and_drag());
    let reset_response = ui.interact(reset_rect, id.with("reset"), Sense::click());

    let mut new_value = None;
    if enabled && (track_response.dragged() || track_response.clicked())
        && let Some(pointer) = track_response.interact_pointer_pos() {
            let t = ((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let raw = spec.min + t * (spec.max - spec.min);
            let snapped = ((raw / spec.step).round() * spec.step).clamp(spec.min, spec.max);
            if (snapped - value).abs() > spec.step * 0.01 {
                new_value = Some(snapped);
            }
        }
    if enabled && reset_response.clicked() && (value - spec.default).abs() > spec.step * 0.01 {
        new_value = Some(spec.default);
    }

    let shown = new_value.unwrap_or(value);
    let t = ((shown - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0);
    let handle_x = track.left() + t * track.width();
    let active = enabled && (track_response.hovered() || track_response.dragged());
    let painter = ui.painter();

    let label_color = if enabled { TEXT_DIM } else { TEXT_OFF };
    painter.galley_with_override_text_color(pos2(rect.left(), y - label_galley.size().y / 2.0), label_galley, label_color);
    painter.line_segment([pos2(track.left(), y), pos2(track.right(), y)], Stroke::new(2.0, LINE_DIM));
    painter.line_segment([pos2(track.left(), y), pos2(handle_x, y)], Stroke::new(2.0, if enabled { CYAN } else { TEXT_OFF }));
    // 默认值位置的小刻度
    let default_x = track.left() + ((spec.default - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0) * track.width();
    painter.line_segment([pos2(default_x, y - 5.0), pos2(default_x, y + 5.0)], Stroke::new(1.0, LINE));
    let handle = Rect::from_center_size(pos2(handle_x, y), vec2(8.0, 18.0));
    painter.rect_filled(handle, 0.0, if !enabled { TEXT_OFF } else if active { TEXT } else { CYAN });

    let value_color = if enabled { CYAN } else { TEXT_OFF };
    painter.galley_with_override_text_color(pos2(reset_rect.left() - 8.0 - value_galley.size().x, y - value_galley.size().y / 2.0), value_galley, value_color);

    let reset_hovered = enabled && reset_response.hovered();
    painter.rect_stroke(reset_rect, 0.0, Stroke::new(1.0, if reset_hovered { TEXT } else { LINE_DIM }), StrokeKind::Inside);
    let reset_color = if !enabled { TEXT_OFF } else if reset_hovered { TEXT } else { TEXT_DIM };
    painter.galley_with_override_text_color(reset_rect.center() - reset_galley.size() / 2.0, reset_galley, reset_color);

    new_value
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Normal,
    Primary,
    Danger,
}

pub fn button(ui: &mut Ui, text: &str, kind: ButtonKind) -> Response {
    let font_id = FontId::proportional(15.0);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font_id, TEXT);
    let size = vec2((galley.size().x + 36.0).max(92.0), galley.size().y.max(18.0) + 18.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    let hovered = enabled && response.hovered();
    let pressed = hovered && response.is_pointer_button_down_on();
    let accent = match kind {
        ButtonKind::Normal => LINE,
        ButtonKind::Primary => CYAN,
        ButtonKind::Danger => RED,
    };

    let painter = ui.painter();
    let fill = if pressed { accent.gamma_multiply(0.3) } else if hovered { HOVER } else { BG };
    painter.rect_filled(rect, 0.0, fill);
    let border = if !enabled { LINE_DIM } else if hovered { TEXT } else { accent };
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, border), StrokeKind::Inside);
    if enabled && kind != ButtonKind::Normal {
        for corner in [rect.left_top(), rect.right_bottom()] {
            painter.rect_filled(Rect::from_center_size(corner, vec2(4.0, 4.0)), 0.0, accent);
        }
    }
    let text_color = if !enabled { TEXT_OFF } else if kind == ButtonKind::Normal && !hovered { TEXT_DIM } else { TEXT };
    painter.galley_with_override_text_color(rect.center() - galley.size() / 2.0, galley, text_color);
    response
}

/// 左侧导航项：中文大字 + 英文小字，选中时像游戏菜单里那样套一个框。
pub fn nav_item(ui: &mut Ui, title: &str, subtitle: &str, selected: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 62.0), Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter();
    if selected {
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, LINE), StrokeKind::Inside);
        painter.rect_filled(Rect::from_min_size(rect.left_top(), vec2(3.0, rect.height())), 0.0, CYAN);
    } else if hovered {
        painter.rect_filled(rect, 0.0, HOVER);
    }
    let color = if selected || hovered { TEXT } else { TEXT_DIM };
    painter.text(pos2(rect.left() + 20.0, rect.center().y - 1.0), Align2::LEFT_CENTER, title, FontId::proportional(24.0), color);
    painter.text(
        pos2(rect.right() - 12.0, rect.center().y + 2.0),
        Align2::RIGHT_CENTER,
        subtitle,
        font(12.0, DIGITS),
        if selected { CYAN } else { TEXT_OFF },
    );
    response
}

/// 标题栏上的最小化 / 关闭按钮，图形自己画，不依赖字体里有没有这些符号。
pub fn window_button(ui: &mut Ui, close: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(34.0, 28.0), Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter();
    if hovered {
        painter.rect_filled(rect, 0.0, if close { RED.gamma_multiply(0.8) } else { HOVER });
    }
    let color = if hovered { TEXT } else { TEXT_DIM };
    let c = rect.center();
    let stroke = Stroke::new(1.3, color);
    if close {
        painter.line_segment([c + vec2(-5.0, -5.0), c + vec2(5.0, 5.0)], stroke);
        painter.line_segment([c + vec2(-5.0, 5.0), c + vec2(5.0, -5.0)], stroke);
    } else {
        painter.line_segment([c + vec2(-6.0, 0.0), c + vec2(6.0, 0.0)], stroke);
    }
    response
}

/// 状态行：左边固定宽度的名称，右边小方块 + 状态文字。
pub fn status_row(ui: &mut Ui, name: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(96.0, 26.0), Sense::hover());
        ui.painter().text(rect.left_center(), Align2::LEFT_CENTER, name, FontId::proportional(15.0), TEXT_DIM);
        let (dot, _) = ui.allocate_exact_size(vec2(10.0, 26.0), Sense::hover());
        ui.painter().rect_filled(Rect::from_center_size(dot.center(), vec2(8.0, 8.0)), 0.0, color);
        ui.label(egui::RichText::new(value).color(TEXT).size(15.0));
    });
}
