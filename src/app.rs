use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, Event, FontFamily, FontId, Key, Layout,
    Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect, RichText, Sense, Stroke, ThemePreference,
    Ui, Vec2, ViewportCommand, pos2, vec2,
};

use crate::folder;
use crate::i18n::{Lang, Text};
use crate::loader::{Loaded, Loader, Outcome};
use crate::theme::{self, ICONS, Pal};
use crate::view::{STEP, View, Zoom, fit_scale};

/// How many photos on each side of the current one are decoded ahead of time.
const PREFETCH: usize = 2;
/// Textures further away than this are dropped to keep GPU memory bounded.
const KEEP: usize = 3;

const TOOLBAR_H: f32 = 32.0;
const STATUS_H: f32 = 23.0;

pub struct App {
    files: Vec<PathBuf>,
    index: usize,
    /// +1 or -1, the direction of the last move. Prefetch favors that side.
    direction: isize,
    cache: HashMap<PathBuf, Outcome>,
    loader: Loader,
    view: View,
    /// The last photo that was on screen. Stays visible, dimmed, while the next one decodes.
    shown: Option<Loaded>,
    theme: ThemePreference,
    lang: Lang,
    has_icons: bool,
    message: Option<String>,
    title: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, start: Option<PathBuf>) -> Self {
        let ctx = &cc.egui_ctx;
        let has_icons = theme::install_fonts(ctx);
        let theme =
            cc.storage
                .and_then(|s| s.get_string("theme"))
                .map_or(ThemePreference::System, |s| match s.as_str() {
                    "dark" => ThemePreference::Dark,
                    "light" => ThemePreference::Light,
                    _ => ThemePreference::System,
                });
        theme::apply(ctx, theme);
        let lang = cc
            .storage
            .and_then(|s| s.get_string("lang"))
            .and_then(|c| Lang::from_code(&c))
            .unwrap_or_else(Lang::detect);

        let mut app = Self {
            files: Vec::new(),
            index: 0,
            direction: 1,
            cache: HashMap::new(),
            loader: Loader::new(ctx),
            view: View::default(),
            shown: None,
            theme,
            lang,
            has_icons,
            message: None,
            title: String::new(),
        };
        if let Some(path) = start {
            app.open(vec![path]);
        }
        app
    }

    fn t(&self) -> &'static Text {
        self.lang.text()
    }

    fn current(&self) -> Option<&PathBuf> {
        self.files.get(self.index)
    }

    /// One file opens its whole folder, several files open just those.
    fn open(&mut self, mut paths: Vec<PathBuf>) {
        self.message = None;
        let (files, index) = if paths.len() == 1 {
            let path = paths.remove(0);
            if path.is_dir() {
                (folder::list_images(&path), 0)
            } else {
                let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
                match folder::list_images(&dir) {
                    Ok(mut files) => {
                        let i = match files.iter().position(|f| *f == path) {
                            Some(i) => i,
                            None => {
                                files.insert(0, path);
                                0
                            }
                        };
                        (Ok(files), i)
                    }
                    Err(_) => (Ok(vec![path]), 0),
                }
            }
        } else {
            paths.retain(|p| folder::is_image(p));
            folder::sort(&mut paths);
            (Ok(paths), 0)
        };

        match files {
            Ok(files) if !files.is_empty() => {
                self.files = files;
                self.index = index;
                self.direction = 1;
                self.cache.clear();
            }
            Ok(_) => self.message = Some(self.t().no_images.into()),
            Err(e) => self.message = Some(self.t().folder_error.replace("{error}", &e.to_string())),
        }
    }

    fn go(&mut self, step: isize) {
        let n = self.files.len();
        if n < 2 {
            return;
        }
        self.index = (self.index as isize + step).rem_euclid(n as isize) as usize;
        self.direction = step.signum();
    }

    fn go_to(&mut self, index: usize) {
        if index < self.files.len() && index != self.index {
            self.direction = if index > self.index { 1 } else { -1 };
            self.index = index;
        }
    }

    fn neighbor(&self, offset: isize) -> usize {
        (self.index as isize + offset).rem_euclid(self.files.len() as isize) as usize
    }

    /// Indices around the current photo, nearest first, the direction of travel before the other side.
    fn near(&self, dist: usize) -> Vec<usize> {
        let mut v = vec![self.index];
        for d in 1..=dist.min(self.files.len() / 2) {
            for i in [
                self.neighbor(self.direction * d as isize),
                self.neighbor(-self.direction * d as isize),
            ] {
                if !v.contains(&i) {
                    v.push(i);
                }
            }
        }
        v
    }

    /// Queues the current photo first, then its neighbors in the direction of travel.
    fn schedule(&mut self) {
        for (path, outcome) in self.loader.poll().collect::<Vec<_>>() {
            self.cache.insert(path, outcome);
        }
        if self.files.is_empty() {
            return;
        }

        let keep: Vec<&PathBuf> = self
            .near(KEEP)
            .into_iter()
            .map(|i| &self.files[i])
            .collect();
        self.cache.retain(|p, _| keep.contains(&p));

        let wanted: Vec<PathBuf> = self
            .near(PREFETCH)
            .into_iter()
            .map(|i| self.files[i].clone())
            .filter(|p| !self.cache.contains_key(p))
            .collect();
        self.loader.want(wanted);
    }

    fn cycle_theme(&mut self, ctx: &egui::Context) {
        self.theme = match self.theme {
            ThemePreference::System => ThemePreference::Dark,
            ThemePreference::Dark => ThemePreference::Light,
            ThemePreference::Light => ThemePreference::System,
        };
        ctx.set_theme(self.theme);
    }

    fn pick_files(&mut self) {
        let picked = rfd::FileDialog::new()
            .set_title(self.t().dialog_title)
            .add_filter(
                self.t().dialog_filter,
                &[
                    "jpg", "jpeg", "jfif", "png", "webp", "gif", "bmp", "tif", "tiff", "ico",
                ],
            )
            .pick_files();
        if let Some(paths) = picked {
            self.open(paths);
        }
    }

    fn toggle_fullscreen(ctx: &egui::Context) {
        let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!full));
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = match self.current().and_then(|p| p.file_name()) {
            Some(name) => format!("{} · Guren", name.to_string_lossy()),
            None => "Guren".to_owned(),
        };
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn keyboard(&mut self, ctx: &egui::Context, area: Rect) {
        let image = self.shown.as_ref().map(|l| size_vec(l.size));
        let ppp = ctx.pixels_per_point();
        let area_px = scale_rect(area, ppp);

        let mut action = None;
        let mut pan = Vec2::ZERO;
        ctx.input_mut(|i| {
            if i.consume_key(Modifiers::COMMAND, Key::O) {
                action = Some(Action::Open);
            }
            let next = [Key::ArrowRight, Key::PageDown, Key::Space];
            let prev = [Key::ArrowLeft, Key::PageUp, Key::Backspace];
            let mut step = 0isize;
            for key in next {
                step += i.count_and_consume_key(Modifiers::NONE, key) as isize;
            }
            for key in prev {
                step -= i.count_and_consume_key(Modifiers::NONE, key) as isize;
            }
            if step != 0 {
                action = Some(Action::Go(step));
            }
            if i.consume_key(Modifiers::NONE, Key::Home) {
                action = Some(Action::GoTo(0));
            }
            if i.consume_key(Modifiers::NONE, Key::End) {
                action = Some(Action::GoTo(usize::MAX));
            }
            if i.consume_key(Modifiers::NONE, Key::Plus)
                || i.consume_key(Modifiers::NONE, Key::Equals)
            {
                action = Some(Action::ZoomBy(STEP));
            }
            if i.consume_key(Modifiers::NONE, Key::Minus) {
                action = Some(Action::ZoomBy(1.0 / STEP));
            }
            if i.consume_key(Modifiers::NONE, Key::Num0) {
                action = Some(Action::Fit);
            }
            if i.consume_key(Modifiers::NONE, Key::Num1) {
                action = Some(Action::Actual);
            }
            if i.consume_key(Modifiers::NONE, Key::F11) || i.consume_key(Modifiers::NONE, Key::F) {
                action = Some(Action::Fullscreen);
            }
            if i.consume_key(Modifiers::NONE, Key::Escape)
                && i.viewport().fullscreen.unwrap_or(false)
            {
                action = Some(Action::Fullscreen);
            }
            if i.consume_key(Modifiers::NONE, Key::T) {
                action = Some(Action::Theme);
            }
            if i.consume_key(Modifiers::NONE, Key::L) {
                action = Some(Action::Language);
            }
            let held = |k| i.key_down(k);
            let speed = 900.0 * i.stable_dt.min(0.05) * ppp;
            if held(Key::ArrowUp) {
                pan.y += speed;
            }
            if held(Key::ArrowDown) {
                pan.y -= speed;
            }
        });

        if pan != Vec2::ZERO {
            if let Some(image) = image {
                self.view.pan(pan, image, area_px.size());
            }
            ctx.request_repaint();
        }
        if let Some(action) = action {
            self.run(action, ctx, area);
        }
    }

    fn run(&mut self, action: Action, ctx: &egui::Context, area: Rect) {
        let ppp = ctx.pixels_per_point();
        let area_px = scale_rect(area, ppp);
        let image = self.shown.as_ref().map(|l| size_vec(l.size));
        match action {
            Action::Open => self.pick_files(),
            Action::Go(step) => self.go(step),
            Action::GoTo(i) => self.go_to(i.min(self.files.len().saturating_sub(1))),
            Action::ZoomBy(f) => {
                if let Some(image) = image {
                    self.view.zoom_by(f, area_px.center(), image, area_px);
                }
            }
            Action::Fit => self.view = View::default(),
            Action::Actual => {
                if let Some(image) = image {
                    self.view.zoom = Zoom::Pixel(self.view.scale(image, area_px.size()));
                    self.view.set_scale(1.0, area_px.center(), image, area_px);
                }
            }
            Action::Fullscreen => Self::toggle_fullscreen(ctx),
            Action::Theme => self.cycle_theme(ctx),
            Action::Language => self.lang = self.lang.next(),
        }
    }

    fn toolbar(&mut self, ui: &mut Ui, pal: &Pal, area: Rect) {
        let t = self.t();
        let mut action = None;
        ui.horizontal_centered(|ui| {
            ui.add_space(6.0);
            if self
                .icon_button(ui, "\u{E8E5}", t.open, t.open_tip)
                .clicked()
            {
                action = Some(Action::Open);
            }
            ui.add_space(6.0);
            let many = self.files.len() > 1;
            if ui
                .add_enabled_ui(many, |ui| self.icon_button(ui, "\u{E76B}", "‹", t.prev_tip))
                .inner
                .clicked()
            {
                action = Some(Action::Go(-1));
            }
            if ui
                .add_enabled_ui(many, |ui| self.icon_button(ui, "\u{E76C}", "›", t.next_tip))
                .inner
                .clicked()
            {
                action = Some(Action::Go(1));
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(6.0);
                let full = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                let (glyph, tip) = if full {
                    ("\u{E73F}", t.leave_fullscreen_tip)
                } else {
                    ("\u{E740}", t.fullscreen_tip)
                };
                if self.icon_button(ui, glyph, t.fullscreen, tip).clicked() {
                    action = Some(Action::Fullscreen);
                }
                let theme_tip = match self.theme {
                    ThemePreference::System => t.theme_system_tip,
                    ThemePreference::Dark => t.theme_dark_tip,
                    ThemePreference::Light => t.theme_light_tip,
                };
                if self
                    .icon_button(ui, "\u{E793}", t.theme, theme_tip)
                    .clicked()
                {
                    action = Some(Action::Theme);
                }
                let lang = egui::Button::new(self.lang.code().to_uppercase())
                    .frame_when_inactive(false)
                    .min_size(vec2(32.0, 24.0));
                if ui.add(lang).on_hover_text(t.language_tip).clicked() {
                    action = Some(Action::Language);
                }
                ui.add_space(6.0);
                let has_image = self.shown.is_some();
                ui.add_enabled_ui(has_image, |ui| {
                    if ui
                        .add(egui::Button::new("1:1").min_size(vec2(32.0, 24.0)))
                        .on_hover_text(t.actual_size_tip)
                        .clicked()
                    {
                        action = Some(Action::Actual);
                    }
                    if self.icon_button(ui, "\u{E9A6}", t.fit, t.fit_tip).clicked() {
                        action = Some(Action::Fit);
                    }
                    if self
                        .icon_button(ui, "\u{E8A3}", "+", t.zoom_in_tip)
                        .clicked()
                    {
                        action = Some(Action::ZoomBy(STEP));
                    }
                    let percent = self.zoom_percent(ui.ctx(), area);
                    ui.add_sized(
                        vec2(52.0, 24.0),
                        egui::Label::new(RichText::new(percent).color(pal.weak)),
                    );
                    if self
                        .icon_button(ui, "\u{E71F}", "−", t.zoom_out_tip)
                        .clicked()
                    {
                        action = Some(Action::ZoomBy(1.0 / STEP));
                    }
                });
            });
        });
        if let Some(action) = action {
            self.run(action, ui.ctx(), area);
        }
    }

    fn icon_button(&self, ui: &mut Ui, glyph: &str, fallback: &str, tip: &str) -> egui::Response {
        let text = if self.has_icons {
            RichText::new(glyph)
                .family(FontFamily::Name(ICONS.into()))
                .size(16.0)
        } else {
            RichText::new(fallback)
        };
        ui.add(
            egui::Button::new(text)
                .frame_when_inactive(false)
                .min_size(vec2(32.0, 24.0)),
        )
        .on_hover_text(tip)
    }

    fn zoom_percent(&self, ctx: &egui::Context, area: Rect) -> String {
        match &self.shown {
            Some(l) => {
                let ppp = ctx.pixels_per_point();
                let scale = self.view.scale(size_vec(l.size), area.size() * ppp);
                format!("{:.0}%", scale * 100.0)
            }
            None => "–".into(),
        }
    }

    fn status_bar(&self, ui: &mut Ui, pal: &Pal) {
        ui.horizontal_centered(|ui| {
            ui.add_space(10.0);
            let Some(path) = self.current() else {
                return;
            };
            let small = |s: String, c: Color32| RichText::new(s).size(12.0).color(c);
            ui.label(small(
                format!("{} / {}", self.index + 1, self.files.len()),
                pal.weak,
            ));
            ui.add_space(8.0);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default();
            ui.label(small(name.into_owned(), pal.text));
            match self.cache.get(path) {
                Some(Ok(l)) => {
                    ui.add_space(8.0);
                    ui.label(small(format!("{} × {}", l.size[0], l.size[1]), pal.weak));
                    ui.add_space(8.0);
                    ui.label(small(human_size(l.bytes), pal.weak));
                }
                Some(Err(_)) => {}
                None => {
                    ui.add_space(8.0);
                    ui.label(small(self.t().loading.into(), pal.weak));
                }
            }
        });
    }

    fn canvas(&mut self, ui: &mut Ui, pal: &Pal) {
        let area = ui.max_rect();
        let ctx = ui.ctx().clone();
        let ppp = ctx.pixels_per_point();
        let area_px = scale_rect(area, ppp);

        let current = self.current().cloned();
        let outcome = current.as_ref().and_then(|p| self.cache.get(p)).cloned();
        let ready = matches!(outcome, Some(Ok(_)));
        if let Some(Ok(l)) = &outcome {
            self.shown = Some(l.clone());
        }
        if current.is_none() {
            self.shown = None;
        }

        let response = ui.allocate_rect(area, Sense::click_and_drag());
        let image = self.shown.as_ref().map(|l| size_vec(l.size));

        if let Some(image) = image {
            if response.dragged_by(PointerButton::Primary) {
                self.view
                    .pan(response.drag_delta() * ppp, image, area_px.size());
            }
            if response.hovered() {
                let zoom = wheel_zoom(&ctx);
                if zoom != 1.0 {
                    let anchor = response.hover_pos().unwrap_or(area.center());
                    self.view
                        .zoom_by(zoom, (anchor.to_vec2() * ppp).to_pos2(), image, area_px);
                }
            }
            if response.double_clicked() {
                let anchor = response.interact_pointer_pos().unwrap_or(area.center());
                let anchor = (anchor.to_vec2() * ppp).to_pos2();
                if self.view.is_fit() {
                    self.view.zoom = Zoom::Pixel(self.view.scale(image, area_px.size()));
                    self.view.set_scale(1.0, anchor, image, area_px);
                } else {
                    self.view = View::default();
                }
            }
        }
        if response.clicked_by(PointerButton::Extra1) {
            self.go(-1);
        }
        if response.clicked_by(PointerButton::Extra2) {
            self.go(1);
        }

        let painter = ui.painter_at(area);
        let failed = matches!(outcome, Some(Err(_)));
        if let (Some(l), Some(image), false) = (&self.shown, image, failed) {
            let rect_px = self.view.image_rect(image, area_px);
            let rect = Rect::from_min_size(
                (rect_px.min.to_vec2() / ppp).to_pos2(),
                rect_px.size() / ppp,
            );
            let tint = if ready {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(110)
            };
            painter.image(
                l.texture.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                tint,
            );
            if self.view.scale(image, area_px.size()) > fit_scale(image, area_px.size()) {
                ui.ctx().set_cursor_icon(if response.dragged() {
                    CursorIcon::Grabbing
                } else {
                    CursorIcon::Grab
                });
            }
        }

        match outcome {
            Some(Err(e)) => {
                let name = current
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                painter.text(
                    area.center(),
                    Align2::CENTER_CENTER,
                    self.t()
                        .open_failed
                        .replace("{name}", &name)
                        .replace("{error}", &e),
                    FontId::proportional(14.0),
                    pal.error_text,
                );
            }
            None if current.is_some() => {
                ui.put(
                    Rect::from_center_size(area.center(), vec2(32.0, 32.0)),
                    egui::Spinner::new().size(32.0).color(pal.accent_text),
                );
            }
            _ => {}
        }

        if current.is_none() {
            self.empty_state(ui, pal, area);
        } else if self.files.len() > 1 {
            self.side_arrows(ui, pal, area);
        }
    }

    fn empty_state(&mut self, ui: &mut Ui, pal: &Pal, area: Rect) {
        let hovering = ui.input(|i| !i.raw.hovered_files.is_empty());
        let rect = Rect::from_center_size(area.center(), vec2(360.0, 140.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.vertical_centered(|ui| {
                let text = if hovering {
                    self.t().drop_release
                } else {
                    self.t().drop_here
                };
                ui.label(RichText::new(text).size(20.0));
                ui.add_space(4.0);
                if let Some(m) = &self.message {
                    ui.label(RichText::new(m).color(pal.error_text));
                } else {
                    ui.label(RichText::new(self.t().or_ctrl_o).color(pal.weak));
                }
                ui.add_space(12.0);
                let open =
                    egui::Button::new(RichText::new(self.t().open_photo).color(pal.on_accent))
                        .fill(pal.accent_text)
                        .stroke(Stroke::NONE)
                        .min_size(vec2(120.0, 32.0));
                if ui.add(open).clicked() {
                    self.pick_files();
                }
            });
        });
        if hovering {
            ui.painter().rect_stroke(
                area.shrink(12.0),
                CornerRadius::same(8),
                Stroke::new(2.0, pal.accent_text),
                egui::StrokeKind::Inside,
            );
        }
    }

    /// Round buttons on both edges, visible only while the pointer is near them.
    fn side_arrows(&mut self, ui: &mut Ui, pal: &Pal, area: Rect) {
        let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) else {
            return;
        };
        if !area.contains(pointer) {
            return;
        }
        let zone = 120.0_f32.min(area.width() / 4.0);
        for (step, glyph, fallback, x) in [
            (-1, "\u{E76B}", "‹", area.left() + 32.0),
            (1, "\u{E76C}", "›", area.right() - 32.0),
        ] {
            if (pointer.x - x).abs() > zone {
                continue;
            }
            let rect = Rect::from_center_size(pos2(x, area.center().y), vec2(40.0, 40.0));
            let resp = ui.interact(rect, ui.scope_id().with(("side", step)), Sense::click());
            let fill = if resp.is_pointer_button_down_on() {
                pal.pressed
            } else if resp.hovered() {
                pal.hover
            } else {
                pal.field.gamma_multiply(0.85)
            };
            let p = ui.painter();
            p.circle(
                rect.center(),
                20.0,
                fill,
                Stroke::new(1.0, pal.popup_stroke),
            );
            let (text, font) = if self.has_icons {
                (glyph, FontId::new(16.0, FontFamily::Name(ICONS.into())))
            } else {
                (fallback, FontId::proportional(22.0))
            };
            p.text(rect.center(), Align2::CENTER_CENTER, text, font, pal.text);
            if resp.clicked() {
                self.go(step);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Action {
    Open,
    Go(isize),
    GoTo(usize),
    ZoomBy(f32),
    Fit,
    Actual,
    Fullscreen,
    Theme,
    Language,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let pal = theme::pal(ctx.theme());

        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if !dropped.is_empty() {
            self.open(dropped);
        }

        let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        let screen = ui.max_rect();
        let mut area = screen;
        if !full {
            area.min.y += TOOLBAR_H;
            area.max.y -= STATUS_H;
        }

        self.keyboard(&ctx, area);
        self.schedule();
        self.update_title(&ctx);

        if !full {
            let bar = Rect::from_min_max(screen.min, pos2(screen.max.x, area.min.y));
            ui.painter().rect_filled(bar, 0.0, pal.nav);
            ui.painter().hline(
                bar.x_range(),
                bar.bottom() - 0.5,
                Stroke::new(1.0, pal.divider),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| {
                self.toolbar(ui, pal, area)
            });

            let status = Rect::from_min_max(pos2(screen.min.x, area.max.y), screen.max);
            ui.painter().rect_filled(status, 0.0, pal.status);
            ui.painter().hline(
                status.x_range(),
                status.top() + 0.5,
                Stroke::new(1.0, pal.divider),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(status), |ui| {
                self.status_bar(ui, pal)
            });
        }

        let bg = if full { pal.strip } else { pal.content };
        ui.painter().rect_filled(area, 0.0, bg);
        ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
            self.canvas(ui, pal)
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let theme = match self.theme {
            ThemePreference::Dark => "dark",
            ThemePreference::Light => "light",
            ThemePreference::System => "system",
        };
        storage.set_string("theme", theme.into());
        storage.set_string("lang", self.lang.code().into());
    }
}

/// Plain mouse wheel zooms, there is nothing to scroll in a photo viewer.
/// Trackpad pinch arrives as a separate zoom event.
fn wheel_zoom(ctx: &egui::Context) -> f32 {
    ctx.input(|i| {
        let mut factor = 1.0;
        for e in &i.events {
            match e {
                Event::MouseWheel { unit, delta, .. } => {
                    let points = match unit {
                        MouseWheelUnit::Point => delta.y,
                        MouseWheelUnit::Line => delta.y * 40.0,
                        MouseWheelUnit::Page => delta.y * 400.0,
                    };
                    factor *= (points * 0.004).exp();
                }
                Event::Zoom(z) => factor *= z,
                _ => {}
            }
        }
        factor
    })
}

fn size_vec(size: [usize; 2]) -> Vec2 {
    vec2(size[0] as f32, size[1] as f32)
}

fn scale_rect(r: Rect, ppp: f32) -> Rect {
    Rect::from_min_max(
        Pos2::new(r.min.x * ppp, r.min.y * ppp),
        Pos2::new(r.max.x * ppp, r.max.y * ppp),
    )
}

fn human_size(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB", b / 1024.0 / 1024.0)
    } else {
        format!("{:.0} KB", (b / 1024.0).max(1.0))
    }
}
