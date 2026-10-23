//! The main window: state, the Downloads page, header and footer.

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use egui::{Align, Color32, CornerRadius, Layout, Margin, Rect, UiBuilder, Vec2, vec2};
use ytdl_core::command::{preview, shell_quote};
use ytdl_core::config::{Config, Theme as ThemeSetting, default_dirs};
use ytdl_core::info::human_duration;
use ytdl_core::options::{AudioFormat, AudioQuality, Browser, Container, Mode, Resolution};
use ytdl_core::progress::{ErrorKind, PostStep, human_bytes, human_eta};
use ytdl_core::queue::{Item, ItemId, Status};
use ytdl_runner::update::UpdateOutcome;
use ytdl_runner::{Downloader, Finished};

use crate::icons::{self, Icon};
use crate::paths;
use crate::system::{self, AppUpdateEvent, ToolEvent, ToolStatus, browser_name};
use crate::theme::{self, Weight, palette, radius};
use crate::views::{self, Action, DetailsView, Fix, RowState, RowView, Stage};
use crate::widgets::{self as w, Kind, Ring};

/// A newer version of the app, from the check at start.
enum AppUpdate {
    None,
    Available(String),
    Installing(String),
    /// The AppImage was replaced; a restart runs the new version.
    Installed(String),
    Failed(String, String),
    Dismissed,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Audio,
    Video,
}

pub struct App {
    pub(crate) config: Config,
    config_path: Option<PathBuf>,
    /// Theme forced from the command line; not saved.
    theme_override: Option<ThemeSetting>,
    pub(crate) default_dirs: (PathBuf, PathBuf),
    link: String,
    link_error: Option<String>,
    pub(crate) downloader: Downloader,
    pub(crate) tools: ToolStatus,
    tool_events: Option<Receiver<ToolEvent>>,
    app_update: AppUpdate,
    app_update_events: Option<Receiver<AppUpdateEvent>>,
    browsers: Vec<Browser>,
    selected: Option<ItemId>,
    /// Row under the pointer in the previous frame, so its background is
    /// drawn behind the content instead of over it.
    hovered: Option<ItemId>,
    hovered_next: Option<ItemId>,
    log_for: Option<ItemId>,
    demo: bool,
    pub(crate) page: Page,
    pub(crate) history: ytdl_core::history::History,
    pub(crate) history_search: String,
    pub(crate) ctx: egui::Context,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        demo: bool,
        theme_override: Option<ThemeSetting>,
    ) -> Self {
        let ctx = cc.egui_ctx.clone();
        theme::install(&ctx);
        egui_extras::install_image_loaders(&ctx);
        let config_path = paths::config_file();
        let config = config_path
            .as_deref()
            .and_then(|p| Config::load(p).ok())
            .unwrap_or_default();
        theme::apply_setting(&ctx, theme_override.unwrap_or(config.theme));

        let mut tools = system::find_tools(
            config.ytdlp_override.clone(),
            config.ffmpeg_override.clone(),
        );
        let wake_ctx = ctx.clone();
        let downloader = Downloader::new(config.max_parallel, tools.tools(), move || {
            wake_ctx.request_repaint()
        });
        let wake_ctx = ctx.clone();
        tools.updating = config.auto_update_ytdlp && !demo;
        let tool_events = Some(system::check_ytdlp(
            tools.ytdlp.clone(),
            tools.updating,
            move || wake_ctx.request_repaint(),
        ));

        let wake_ctx = ctx.clone();
        let app_update_events = (config.check_app_updates && !demo)
            .then(|| system::check_app_update(move || wake_ctx.request_repaint()));

        Self {
            app_update: AppUpdate::None,
            app_update_events,
            default_dirs: default_dirs(&paths::home(), cfg!(target_os = "macos")),
            config,
            config_path,
            theme_override,
            link: String::new(),
            link_error: None,
            downloader,
            tools,
            tool_events,
            browsers: system::installed_browsers(),
            selected: None,
            hovered: None,
            hovered_next: None,
            log_for: None,
            demo,
            page: Page::Downloads,
            history: paths::history_file()
                .map(|p| ytdl_core::history::History::load(&p))
                .unwrap_or_default(),
            history_search: String::new(),
            ctx,
        }
    }

    pub(crate) fn save_config(&self) {
        if let Some(path) = &self.config_path
            && let Err(e) = self.config.save(path)
        {
            eprintln!("could not save settings: {e}");
        }
    }

    pub(crate) fn set_theme(&mut self, setting: ThemeSetting) {
        self.theme_override = None;
        self.config.theme = setting;
        theme::apply_setting(&self.ctx, setting);
        self.save_config();
    }

    /// Re-reads tool locations, e.g. after an update or a changed override.
    pub(crate) fn refresh_tools(&mut self) {
        let version = self.tools.ytdlp_version.take();
        let mut fresh = system::find_tools(
            self.config.ytdlp_override.clone(),
            self.config.ffmpeg_override.clone(),
        );
        fresh.ytdlp_version = if fresh.ytdlp == self.tools.ytdlp {
            version
        } else {
            None
        };
        let changed = fresh.ytdlp != self.tools.ytdlp;
        self.tools = fresh;
        self.downloader.tools = self.tools.tools();
        if changed {
            let ctx = self.ctx.clone();
            self.tool_events = Some(system::check_ytdlp(
                self.tools.ytdlp.clone(),
                false,
                move || ctx.request_repaint(),
            ));
        }
    }

    pub(crate) fn update_ytdlp_now(&mut self) {
        self.tools.updating = true;
        self.tools.update_error = None;
        let ctx = self.ctx.clone();
        self.tool_events = Some(system::check_ytdlp(
            self.tools.ytdlp.clone(),
            true,
            move || ctx.request_repaint(),
        ));
    }

    fn poll_tools(&mut self) {
        let Some(rx) = &self.tool_events else { return };
        let events: Vec<ToolEvent> = rx.try_iter().collect();
        for ev in events {
            match ev {
                ToolEvent::Version(v) => self.tools.ytdlp_version = v,
                ToolEvent::Updated(result) => {
                    self.tools.updating = false;
                    match result {
                        Ok(UpdateOutcome::UpToDate(v)) => self.tools.ytdlp_version = Some(v),
                        Ok(UpdateOutcome::Updated { to, .. }) => {
                            self.refresh_tools();
                            self.tools.ytdlp_version = Some(to);
                        }
                        Err(e) => self.tools.update_error = Some(e),
                    }
                }
            }
        }
    }

    fn poll_app_update(&mut self) {
        let Some(rx) = &self.app_update_events else {
            return;
        };
        for ev in rx.try_iter().collect::<Vec<_>>() {
            self.app_update = match (ev, &self.app_update) {
                (_, AppUpdate::Dismissed) => AppUpdate::Dismissed,
                (AppUpdateEvent::Available(v), _) => AppUpdate::Available(v),
                (AppUpdateEvent::Installed(v), _) => AppUpdate::Installed(v),
                (AppUpdateEvent::Failed(e), AppUpdate::Installing(v)) => {
                    AppUpdate::Failed(v.clone(), e)
                }
                (AppUpdateEvent::Failed(_), _) => AppUpdate::None,
            };
        }
    }

    /// The bar above the link field that offers a newer version.
    fn app_update_bar(&mut self, ui: &mut egui::Ui) {
        let (version, message) = match &self.app_update {
            AppUpdate::None | AppUpdate::Dismissed => return,
            AppUpdate::Available(v) => (v.clone(), format!("Version {v} is available")),
            AppUpdate::Installing(v) => (v.clone(), format!("Downloading version {v}…")),
            AppUpdate::Installed(v) => (v.clone(), format!("Version {v} is installed")),
            AppUpdate::Failed(v, e) => (v.clone(), format!("Update to {v} failed: {e}")),
        };
        let p = palette(ui.ctx());
        let appimage = ytdl_runner::self_update::running_appimage();
        w::card(p.surface, p.border, radius::CARD)
            .inner_margin(Margin::symmetric(16, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    w::text(ui, &message, 13.0, Weight::Medium, p.text);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !matches!(self.app_update, AppUpdate::Installing(_))
                            && w::icon_button(ui, Icon::Close, "Not now").clicked()
                        {
                            self.app_update = AppUpdate::Dismissed;
                        }
                        if matches!(self.app_update, AppUpdate::Installing(_)) {
                            w::progress_ring(ui, Ring::Spinner, 20.0);
                            return;
                        }
                        let installed = matches!(self.app_update, AppUpdate::Installed(_));
                        let (label, icon) = match (&appimage, installed) {
                            (Some(_), true) => ("Restart now", None),
                            (Some(_), false) => ("Update", Some(Icon::Download)),
                            (None, _) => ("Download", Some(Icon::Download)),
                        };
                        if !w::button(ui, Kind::Primary, icon, label, 32.0).clicked() {
                            return;
                        }
                        match appimage {
                            Some(path) if installed => restart(&path, ui.ctx()),
                            Some(path) => {
                                let ctx = self.ctx.clone();
                                self.app_update_events = Some(system::install_app_update(
                                    version.clone(),
                                    path,
                                    move || ctx.request_repaint(),
                                ));
                                self.app_update = AppUpdate::Installing(version.clone());
                            }
                            None => system::open(std::path::Path::new(
                                ytdl_core::app_release::RELEASES_PAGE,
                            )),
                        }
                    });
                });
            });
        ui.add_space(16.0);
    }

    pub(crate) fn add_links(&mut self, text: &str) {
        let out_dir = self.config.out_dir(&self.default_dirs);
        let ids = self
            .downloader
            .add_text(text, &self.config.options, &out_dir);
        if ids.is_empty() {
            self.link_error = Some(
                "No link found. Copy the address of a video or playlist and paste it here.".into(),
            );
        } else {
            self.link_error = None;
            self.link.clear();
            if self.selected.is_none() {
                self.selected = ids.first().copied();
            }
        }
    }

    fn on_finished(&mut self, finished: Vec<Finished>) {
        crate::history_page::record(self, &finished);
    }

    fn apply(&mut self, action: Action) {
        let item = |app: &Self, id| app.downloader.queue().get(id).cloned();
        match action {
            Action::Select(id) => self.selected = Some(id),
            Action::Cancel(id) => self.downloader.cancel(id),
            Action::Remove(id) => {
                self.downloader.remove(id);
                if self.selected == Some(id) {
                    self.selected = None;
                }
            }
            Action::Retry(id) => self.downloader.retry(id, None),
            Action::UseCookies(id, browser) => {
                // Remember the choice and retry every download that failed for
                // want of a signed-in browser, not only the one clicked.
                self.config.options.cookies_from = Some(browser);
                self.save_config();
                let blocked: Vec<Item> = self
                    .downloader
                    .queue()
                    .items()
                    .iter()
                    .filter(|it| {
                        it.id == id || matches!(&it.status, Status::Failed(k) if needs_cookies(k))
                    })
                    .cloned()
                    .collect();
                for it in blocked {
                    let mut options = it.options.clone();
                    options.cookies_from = Some(browser);
                    self.downloader.retry(it.id, Some(options));
                }
            }
            Action::Reveal(id) => {
                if let Some(it) = item(self, id) {
                    system::reveal(it.final_path.as_deref().unwrap_or(&it.out_dir));
                }
            }
            Action::Open(id) => {
                if let Some(path) = item(self, id).and_then(|it| it.final_path) {
                    system::open(&path);
                }
            }
            Action::CopyCommand(id) => {
                if let Some(it) = item(self, id) {
                    let cmd = preview(&it.options).replace("<url>", &shell_quote(&it.url));
                    self.ctx.copy_text(cmd);
                }
            }
            Action::ShowLog(id) => self.log_for = Some(id),
        }
    }

    // ---- view data ----

    fn row_view(&self, it: &Item) -> RowView {
        let title = it
            .info
            .as_ref()
            .and_then(|i| i.title.clone())
            .unwrap_or_else(|| short_url(&it.url));
        let summary = it.options.summary();
        let mut meta = vec![summary.clone()];
        if let Some((i, n)) = it.playlist {
            meta.push(format!("Video {i} of {n}"));
        }
        let state = match &it.status {
            Status::Queued => {
                meta.push("Waiting for a free slot".into());
                RowState::Waiting
            }
            Status::Running => match &it.progress {
                Some(p) => {
                    if let Some(total) = p.total {
                        meta.push(format!(
                            "{} of {}",
                            human_bytes(p.downloaded as f64),
                            human_bytes(total as f64)
                        ));
                    }
                    if let Some(s) = p.speed {
                        meta.push(format!("{}/s", human_bytes(s)));
                    }
                    if let Some(e) = p.eta {
                        meta.push(format!("{} left", human_eta(e)));
                    }
                    match p.fraction() {
                        Some(f) => RowState::Active {
                            ring: Ring::Progress(f),
                            percent: Some(format!("{:.0}%", f * 100.0)),
                        },
                        None => RowState::Active {
                            ring: Ring::Spinner,
                            percent: None,
                        },
                    }
                }
                None => {
                    meta.push("Starting…".into());
                    RowState::Active {
                        ring: Ring::Spinner,
                        percent: None,
                    }
                }
            },
            Status::PostProcessing(step) => {
                meta.push(match step {
                    PostStep::Merging => "Merging video and audio…".into(),
                    PostStep::ExtractingAudio => format!(
                        "Converting to {}…",
                        summary.split(" · ").next().unwrap_or("audio")
                    ),
                    PostStep::Finishing => "Adding cover art and tags…".into(),
                });
                RowState::Active {
                    ring: Ring::Spinner,
                    percent: None,
                }
            }
            Status::Done => {
                if let Some(total) = it.progress.as_ref().and_then(|p| p.total) {
                    meta.push(human_bytes(total as f64));
                }
                meta.push("finished".into());
                RowState::Done
            }
            Status::Cancelled => {
                meta.push("Cancelled".into());
                RowState::Cancelled
            }
            Status::Failed(kind) => {
                let (headline, hint, fix) =
                    problem_text(kind, it, &self.browsers, self.config.options.cookies_from);
                let browsers = if needs_cookies(kind) {
                    self.browsers.clone()
                } else {
                    Vec::new()
                };
                return RowView {
                    id: it.id,
                    title: headline,
                    meta: format!("{hint} · {title}"),
                    state: RowState::Problem { fix, browsers },
                };
            }
        };
        RowView {
            id: it.id,
            title,
            meta: meta.join(" · "),
            state,
        }
    }

    fn details_view(&self, it: &Item) -> DetailsView {
        let info = it.info.clone().unwrap_or_default();
        let mut facts = vec![("Format".to_string(), it.options.summary())];
        let active = it.status.is_active();
        if let Some(p) = &it.progress {
            if let Some(total) = p.total {
                facts.push(("Size".into(), human_bytes(total as f64)));
            }
            if active {
                if let Some(s) = p.speed {
                    facts.push(("Speed".into(), format!("{}/s", human_bytes(s))));
                }
                if let Some(e) = p.eta {
                    facts.push(("Time left".into(), human_eta(e)));
                }
            }
        }
        if let Some(n) = info.playlist_count {
            facts.push(("Videos".into(), n.to_string()));
        }
        if let Status::Failed(ErrorKind::Other(msg)) = &it.status {
            facts.push(("Error".into(), msg.clone()));
        }
        let (path_label, path) = match &it.final_path {
            Some(p) if it.status == Status::Done => ("Saved to", paths::display(p)),
            Some(p) => ("Saving to", paths::display(p)),
            None => ("Saving to", paths::display(&it.out_dir)),
        };
        DetailsView {
            id: it.id,
            title: info.title.clone().unwrap_or_else(|| short_url(&it.url)),
            channel: info.uploader.clone(),
            duration: info.duration.map(human_duration),
            thumbnail: info.thumbnail.clone(),
            fraction: if active {
                it.progress.as_ref().and_then(|p| p.fraction())
            } else {
                None
            },
            facts,
            path_label,
            path,
            audio: matches!(it.options.mode, Mode::Audio { .. }),
            stage: match it.status {
                Status::Done => Stage::Done,
                Status::Failed(_) | Status::Cancelled => Stage::Stopped,
                _ => Stage::Working,
            },
        }
    }

    // ---- drawing ----

    fn header(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        let full = ui.max_rect();
        ui.horizontal(|ui| {
            ui.set_height(44.0);
            let (logo, resp) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::click());
            ui.painter()
                .rect_filled(logo, CornerRadius::same(8), p.text);
            icons::paint(ui.painter(), logo.shrink(7.0), Icon::Download, p.bg);
            if resp.clicked() {
                self.page = Page::Downloads;
            }
            w::text(ui, "YouTube Downloader", 15.0, Weight::SemiBold, p.text);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if w::icon_button(ui, Icon::Sliders, "Settings").clicked() {
                    self.page = toggle(self.page, Page::Settings);
                }
                if w::icon_button(ui, Icon::History, "History").clicked() {
                    self.page = toggle(self.page, Page::History);
                }
                self.theme_button(ui);
            });
        });

        // The Audio | Video switch (or the page title) in the true center.
        let center = Rect::from_center_size(full.center(), vec2(320.0, 44.0));
        ui.scope_builder(UiBuilder::new().max_rect(center), |ui| {
            w::centered_row(ui, "tabs", |ui| match self.page {
                Page::Downloads => {
                    let mut tab = if self.config.is_video() {
                        Tab::Video
                    } else {
                        Tab::Audio
                    };
                    let changed = w::segmented(
                        ui,
                        &mut tab,
                        &[
                            (Tab::Audio, Icon::Music, "Audio"),
                            (Tab::Video, Icon::Video, "Video"),
                        ],
                    );
                    if changed {
                        self.config.set_video(tab == Tab::Video);
                        self.save_config();
                    }
                }
                Page::History | Page::Settings => {
                    if w::button(
                        ui,
                        Kind::Secondary,
                        Some(Icon::Download),
                        "Back to downloads",
                        40.0,
                    )
                    .clicked()
                    {
                        self.page = Page::Downloads;
                    }
                }
            });
        });
    }

    /// Cycles System → Light → Dark. Settings offers the same choice.
    fn theme_button(&mut self, ui: &mut egui::Ui) {
        let current = self.theme_override.unwrap_or(self.config.theme);
        let (icon, name) = match current {
            ThemeSetting::System => (Icon::Monitor, "System"),
            ThemeSetting::Light => (Icon::Sun, "Light"),
            ThemeSetting::Dark => (Icon::Moon, "Dark"),
        };
        let label = format!("Theme: {name} (click to change)");
        if w::icon_button(ui, icon, &label).clicked() {
            self.set_theme(match current {
                ThemeSetting::System => ThemeSetting::Light,
                ThemeSetting::Light => ThemeSetting::Dark,
                ThemeSetting::Dark => ThemeSetting::System,
            });
        }
    }

    fn add_section(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        let paste_hint = if cfg!(target_os = "macos") {
            "⌘V adds instantly"
        } else {
            "Ctrl+V adds instantly"
        };
        ui.vertical_centered(|ui| {
            ui.set_max_width(820.0);
            let field = w::link_field(ui, &mut self.link, paste_hint);
            if field.submitted {
                let text = std::mem::take(&mut self.link);
                if !text.trim().is_empty() {
                    self.add_links(&text);
                    if self.link_error.is_some() {
                        self.link = text;
                    }
                }
            }
            if let Some(err) = &self.link_error {
                ui.add_space(6.0);
                w::text(ui, err, 12.0, Weight::Regular, p.danger);
            }
        });
        ui.add_space(14.0);

        let mut changed = false;
        match self.config.options.mode {
            Mode::Video {
                resolution,
                container,
            } => {
                w::centered_row(ui, "video-choices", |ui| {
                    w::chip_label(ui, "Quality");
                    for r in Resolution::CHOICES {
                        let label = r.0.map_or("Best".to_string(), |h| format!("{h}p"));
                        if w::chip(ui, &label, r == resolution).clicked() {
                            self.config.options.mode = Mode::Video {
                                resolution: r,
                                container,
                            };
                            changed = true;
                        }
                    }
                    w::divider(ui);
                    w::chip_label(ui, "Format");
                    for c in Container::ALL {
                        let label = if c == Container::Webm {
                            "WebM".into()
                        } else {
                            c.as_str().to_uppercase()
                        };
                        if w::chip(ui, &label, c == container).clicked() {
                            self.config.options.mode = Mode::Video {
                                resolution,
                                container: c,
                            };
                            changed = true;
                        }
                    }
                });
            }
            Mode::Audio { format, quality } => {
                w::centered_row(ui, "audio-choices", |ui| {
                    w::chip_label(ui, "Format");
                    for f in AudioFormat::ALL {
                        // Vorbis stays selectable in Settings but is rarely wanted.
                        if f == AudioFormat::Vorbis && format != AudioFormat::Vorbis {
                            continue;
                        }
                        let label = match f {
                            AudioFormat::Best => "Original".to_string(),
                            AudioFormat::Opus => "Opus".to_string(),
                            AudioFormat::Vorbis => "Vorbis".to_string(),
                            other => other.as_str().to_uppercase(),
                        };
                        if w::chip(ui, &label, f == format).clicked() {
                            self.config.options.mode = Mode::Audio { format: f, quality };
                            changed = true;
                        }
                    }
                    if !format.is_lossless() && format != AudioFormat::Best {
                        w::divider(ui);
                        w::chip_label(ui, "Bitrate");
                        for q in
                            [None, Some(320), Some(256), Some(192), Some(128)].map(AudioQuality)
                        {
                            let label = q.0.map_or("Best".to_string(), |k| format!("{k}k"));
                            if w::chip(ui, &label, q == quality).clicked() {
                                self.config.options.mode = Mode::Audio { format, quality: q };
                                changed = true;
                            }
                        }
                    }
                });
            }
        }
        ui.add_space(6.0);
        let video = self.config.is_video();
        w::centered_row(ui, "extras", |ui| {
            let o = &mut self.config.options;
            let mut toggles = vec![
                ("Single video, skip playlist", &mut o.no_playlist),
                ("Cover art", &mut o.embed_thumbnail),
            ];
            if video {
                toggles.push(("Subtitles", &mut o.embed_subs));
            }
            // On/off options look selected when on, like the other choices.
            for (label, value) in toggles {
                if w::chip(ui, label, *value).clicked() {
                    *value = !*value;
                    changed = true;
                }
            }
        });
        if changed {
            self.save_config();
        }
    }

    fn rows(&self) -> Vec<RowView> {
        if self.demo {
            return crate::demo::rows();
        }
        self.downloader
            .queue()
            .items()
            .iter()
            .map(|it| self.row_view(it))
            .collect()
    }

    fn details(&self) -> Option<DetailsView> {
        if self.demo {
            return Some(crate::demo::details());
        }
        let id = self.selected?;
        self.downloader
            .queue()
            .get(id)
            .map(|it| self.details_view(it))
    }

    fn queue(&mut self, ui: &mut egui::Ui, rows: &[RowView], actions: &mut Vec<Action>) {
        let p = palette(ui.ctx());
        let counts = self.downloader.queue().counts();
        let (active, waiting) = if self.demo {
            (2, 1)
        } else {
            (counts.active, counts.queued)
        };
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            w::text(ui, "Queue", 14.0, Weight::SemiBold, p.text);
            w::text(
                ui,
                &format!(
                    "{active} of {} downloading · {waiting} waiting",
                    self.config.max_parallel
                ),
                12.0,
                Weight::Regular,
                p.faint,
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if active + waiting > 0
                    && w::button(ui, Kind::Ghost, None, "Cancel all", 36.0).clicked()
                {
                    self.downloader.cancel_all();
                }
                let finished = counts.done + counts.failed + counts.cancelled;
                if finished > 0
                    && w::button(ui, Kind::Ghost, None, "Clear finished", 36.0).clicked()
                {
                    self.downloader.clear_finished();
                    if self
                        .selected
                        .is_some_and(|id| self.downloader.queue().get(id).is_none())
                    {
                        self.selected = None;
                    }
                }
            });
        });
        ui.add_space(4.0);

        if rows.is_empty() {
            empty_state(ui);
            return;
        }
        let selected = self
            .selected
            .or(if self.demo { Some(rows[0].id) } else { None });
        for row in rows {
            let resp = views::row(
                ui,
                row,
                selected == Some(row.id),
                self.hovered == Some(row.id),
                actions,
            );
            if resp.clicked() {
                actions.push(Action::Select(row.id));
            }
            if resp.hovered() {
                self.hovered_next = Some(row.id);
                if self.hovered != self.hovered_next {
                    ui.ctx().request_repaint();
                }
            }
            ui.add_space(4.0);
        }
    }

    fn downloads_page(&mut self, ui: &mut egui::Ui) {
        self.app_update_bar(ui);
        self.add_section(ui);
        ui.add_space(24.0);
        let rows = self.rows();
        let details = self.details();
        let mut actions = Vec::new();
        let wide = ui.available_width() >= 900.0 && details.is_some();
        if wide {
            let side = 340.0;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 20.0;
                // Leave room for the scroll bar on the right.
                ui.allocate_ui_with_layout(
                    vec2(ui.available_width() - side - 32.0, 0.0),
                    Layout::top_down(Align::Min),
                    |ui| self.queue(ui, &rows, &mut actions),
                );
                ui.allocate_ui_with_layout(vec2(side, 0.0), Layout::top_down(Align::Min), |ui| {
                    ui.set_width(side);
                    if let Some(d) = &details {
                        views::details(ui, d, &mut actions);
                    }
                });
            });
        } else {
            self.queue(ui, &rows, &mut actions);
            if let Some(d) = &details {
                ui.add_space(16.0);
                views::details(ui, d, &mut actions);
            }
        }
        if !self.demo {
            for a in actions {
                self.apply(a);
            }
        }
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        let p = palette(ui.ctx());
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
            icons::paint(ui.painter(), r, Icon::Folder, p.muted);
            let dir = self.config.out_dir(&self.default_dirs);
            let resp = ui.add(
                egui::Label::new(
                    egui::RichText::new(paths::display(&dir))
                        .font(theme::font(12.0, Weight::Regular))
                        .color(p.muted),
                )
                .sense(egui::Sense::click()),
            );
            if resp
                .on_hover_text("Open folder")
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                system::reveal(&dir);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let log_target = self
                    .selected
                    .or_else(|| self.downloader.queue().items().last().map(|i| i.id));
                if let Some(id) = log_target
                    && ui
                        .link(
                            egui::RichText::new("Show log").font(theme::font(12.0, Weight::Medium)),
                        )
                        .clicked()
                {
                    self.log_for = Some(id);
                }
                ui.add_space(10.0);
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let t = &self.tools;
                    let ytdlp = if t.updating {
                        (false, "Updating yt-dlp…".to_string())
                    } else if t.ytdlp.is_none() {
                        (false, "yt-dlp missing".to_string())
                    } else if t.update_error.is_some() {
                        (false, "yt-dlp update failed".to_string())
                    } else {
                        (
                            true,
                            t.ytdlp_version
                                .as_ref()
                                .map_or("yt-dlp ready".into(), |v| format!("yt-dlp {v}")),
                        )
                    };
                    w::status_dot(ui, ytdlp.0, &ytdlp.1);
                    ui.add_space(10.0);
                    let ff = self.tools.ffmpeg.is_some();
                    w::status_dot(ui, ff, if ff { "ffmpeg ready" } else { "ffmpeg missing" });
                });
            });
        });
    }

    fn log_window(&mut self, ctx: &egui::Context) {
        let Some(id) = self.log_for else { return };
        let p = palette(ctx);
        let lines: Vec<String> = self.downloader.log(id).cloned().collect();
        let mut open = true;
        egui::Window::new("Log")
            .open(&mut open)
            .default_size([760.0, 420.0])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if w::button(ui, Kind::Secondary, Some(Icon::Copy), "Copy all", 34.0).clicked()
                    {
                        ctx.copy_text(lines.join("\n"));
                    }
                    w::text(
                        ui,
                        &format!("{} lines", lines.len()),
                        12.0,
                        Weight::Regular,
                        p.faint,
                    );
                });
                ui.add_space(6.0);
                egui::ScrollArea::both()
                    .stick_to_bottom(true)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        if lines.is_empty() {
                            w::text(ui, "Nothing yet.", 12.0, Weight::Regular, p.faint);
                        }
                        for l in &lines {
                            ui.label(
                                egui::RichText::new(l)
                                    .font(theme::mono(12.0))
                                    .color(p.muted),
                            );
                        }
                    });
            });
        if !open {
            self.log_for = None;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Downloads,
    History,
    Settings,
}

fn toggle(current: Page, target: Page) -> Page {
    if current == target {
        Page::Downloads
    } else {
        target
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_tools();
        self.poll_app_update();
        let finished = self.downloader.tick();
        if !finished.is_empty() {
            self.on_finished(finished);
        }

        // Paste anywhere (when no text field has focus) adds the links at once.
        if self.page == Page::Downloads && ctx.memory(|m| m.focused().is_none()) {
            let pasted: Vec<String> = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::Paste(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect()
            });
            for text in pasted {
                self.add_links(&text);
            }
        }

        let p = palette(ctx);
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(24, 12)),
            )
            .show(ctx, |ui| self.header(ui));
        egui::TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(Margin::symmetric(24, 10)),
            )
            .show(ctx, |ui| self.footer(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg).inner_margin(Margin {
                left: 24,
                right: 24,
                top: 32,
                bottom: 20,
            }))
            .show(ctx, |ui| {
                self.hovered = self.hovered_next.take();
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| match self.page {
                        Page::Downloads => self.downloads_page(ui),
                        Page::History => crate::history_page::show(self, ui),
                        Page::Settings => crate::settings_page::show(self, ui),
                    });
            });
        self.log_window(ctx);
    }
}

/// Starts the updated AppImage and closes this window. Running downloads
/// stop, so the button is only offered after the user asked to update.
fn restart(appimage: &std::path::Path, ctx: &egui::Context) {
    match std::process::Command::new(appimage).spawn() {
        Ok(_) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        Err(e) => eprintln!("could not start {}: {e}", appimage.display()),
    }
}

fn empty_state(ui: &mut egui::Ui) {
    let p = palette(ui.ctx());
    w::card(Color32::TRANSPARENT, p.border, radius::CARD)
        .inner_margin(Margin::symmetric(24, 40))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                icons::paint(ui.painter(), r, Icon::Download, p.faint);
                ui.add_space(6.0);
                w::text(
                    ui,
                    "Your downloads will appear here",
                    15.0,
                    Weight::SemiBold,
                    p.text,
                );
                w::text(
                    ui,
                    "Paste one or more links above to get started.",
                    13.0,
                    Weight::Regular,
                    p.muted,
                );
            });
        });
}

/// `https://www.youtube.com/watch?v=x` → `youtube.com/watch?v=x`, for rows without a title yet.
fn short_url(url: &str) -> String {
    let s = url.split_once("://").map_or(url, |(_, rest)| rest);
    s.strip_prefix("www.").unwrap_or(s).to_string()
}

/// Headline, hint and suggested fix for a failed download. For sign-in
/// problems it accounts for the browser already tried and what yt-dlp
/// reported about its cookies.
fn problem_text(
    kind: &ErrorKind,
    it: &Item,
    browsers: &[Browser],
    preferred: Option<Browser>,
) -> (String, String, Fix) {
    let tried = it.options.cookies_from;
    let next = || match tried {
        None => preferred
            .filter(|b| browsers.contains(b))
            .or_else(|| browsers.first().copied()),
        Some(t) => browsers.iter().copied().find(|b| *b != t),
    };
    let cookie_fix = || next().map_or(Fix::Retry, Fix::UseCookies);
    let s = |x: &str| x.to_string();
    match kind {
        ErrorKind::BotCheck | ErrorKind::SignInRequired => match tried {
            None if *kind == ErrorKind::BotCheck => (
                s("YouTube wants you signed in"),
                s("Use the cookies of a browser where you're signed in to YouTube"),
                cookie_fix(),
            ),
            None => (
                s("This video needs a signed-in account"),
                s("Sign in to YouTube in your browser, then use its cookies"),
                cookie_fix(),
            ),
            Some(b) => {
                let name = browser_name(b);
                let hint = match &it.cookies {
                    None => format!(
                        "{name}'s cookies could not be read. Open the log, or try another browser"
                    ),
                    Some(r) if r.count == 0 => {
                        format!(
                            "No cookies found in {name}. Sign in to YouTube in {name}, or try another browser"
                        )
                    }
                    Some(r) if r.undecrypted > 0 && r.undecrypted * 2 >= r.count => format!(
                        "{} of {} {name} cookies couldn't be decrypted. Unlock your keyring or try another browser",
                        r.undecrypted, r.count
                    ),
                    Some(_) => format!(
                        "Open youtube.com in {name}, make sure you're signed in, then retry"
                    ),
                };
                (
                    format!("Still blocked with {name} cookies"),
                    hint,
                    cookie_fix(),
                )
            }
        },
        ErrorKind::CookiesUnreadable(msg) => {
            let name = tried.map_or("the browser", browser_name);
            let m = msg.to_lowercase();
            let hint = if m.contains("database") {
                format!("{name} has no profile on this computer")
            } else if m.contains("secretstorage") || m.contains("keyring") {
                s("The desktop keyring is locked or unavailable")
            } else {
                s("See the log for details")
            };
            (format!("Couldn't read {name} cookies"), hint, cookie_fix())
        }
        ErrorKind::Private => (
            s("This video is private"),
            s("Only its owner can share it"),
            Fix::None,
        ),
        ErrorKind::Unavailable => (
            s("This video is unavailable"),
            s("It may have been removed"),
            Fix::None,
        ),
        ErrorKind::UnsupportedUrl => (
            s("This link isn't supported"),
            s("Check the address and try again"),
            Fix::None,
        ),
        ErrorKind::MissingFfmpeg => (
            s("ffmpeg is missing"),
            s("Reinstall the app, or set ffmpeg in Settings"),
            Fix::Retry,
        ),
        ErrorKind::MissingYtDlp => (
            s("yt-dlp is missing"),
            s("Update it from Settings, then retry"),
            Fix::Retry,
        ),
        ErrorKind::Network => (
            s("Network problem"),
            s("Check your connection and retry"),
            Fix::Retry,
        ),
        ErrorKind::Other(_) => (
            s("Download failed"),
            s("See the log for details"),
            Fix::Retry,
        ),
    }
}

/// Failures that browser cookies can fix.
fn needs_cookies(kind: &ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::BotCheck | ErrorKind::SignInRequired | ErrorKind::CookiesUnreadable(_)
    )
}
