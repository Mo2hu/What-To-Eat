//! 界面层：欢迎页 → 5 道题 → 结果页 → 菜品库。
//!
//! 只负责画界面和发消息，所有筛选规则都在 `wte::quiz` 里。

use std::collections::VecDeque;

use eframe::egui::{self, Align, Layout, RichText};
use wte::menu::{Menu, SAMPLE_MENU};
use wte::quiz::{self, Answer, Answers, Recommendation, ALL_QUESTIONS};
use wte::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    Quiz,
    Result,
    MenuInfo,
}

pub struct WteApp {
    screen: Screen,
    step: usize,
    answers: Answers,
    pending: Option<Answer>,
    menu: Menu,
    rec: Option<Recommendation>,
    recent: VecDeque<String>,
    rng: Rng,
    notice: Option<String>,
}

impl WteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_chinese_font(&cc.egui_ctx);
        setup_style(&cc.egui_ctx);

        Self {
            screen: Screen::Welcome,
            step: 0,
            answers: Answers::default(),
            pending: None,
            menu: Menu::load(),
            rec: None,
            recent: VecDeque::new(),
            rng: Rng::from_time(),
            notice: None,
        }
    }

    fn start_quiz(&mut self) {
        self.screen = Screen::Quiz;
        self.step = 0;
        self.answers.clear();
        self.pending = None;
        self.rec = None;
        self.notice = None;
    }

    fn advance(&mut self) {
        let Some(answer) = self.pending else {
            return;
        };
        self.answers.set(ALL_QUESTIONS[self.step], Some(answer));

        if self.step + 1 < ALL_QUESTIONS.len() {
            self.step += 1;
            self.pending = self.answers.get(ALL_QUESTIONS[self.step]);
        } else {
            self.draw_result();
        }
    }

    /// 点选项后直接进入下一题（最后一题直接出结果）。
    fn choose(&mut self, answer: Answer) {
        self.pending = Some(answer);
        self.advance();
    }

    fn back(&mut self) {
        if self.step > 0 {
            self.step -= 1;
            self.pending = self.answers.get(ALL_QUESTIONS[self.step]);
        }
    }

    /// 抽一道菜并展示结果；「换一个」也走这里。
    fn draw_result(&mut self) {
        let recent: Vec<String> = self.recent.iter().cloned().collect();
        match quiz::recommend(&self.menu.dishes, &self.answers, &recent, &mut self.rng) {
            Some(rec) => {
                self.recent.push_front(rec.dish.name.clone());
                while self.recent.len() > 10 {
                    self.recent.pop_back();
                }
                self.rec = Some(rec);
                self.screen = Screen::Result;
                self.notice = None;
            }
            None => {
                self.rec = None;
                self.notice = Some("题库是空的，先到菜品库里生成或加载菜单。".to_owned());
                self.screen = Screen::MenuInfo;
            }
        }
    }

    fn reload_menu(&mut self) {
        self.menu = Menu::load();
        self.notice = Some(format!("已重新加载：{} 道菜", self.menu.dishes.len()));
    }

    fn create_template(&mut self) {
        let Some(path) = wte::menu::default_custom_path() else {
            self.notice = Some("找不到可写入的目录".to_owned());
            return;
        };
        if path.exists() {
            self.notice = Some(format!(
                "{} 已存在，已直接重新加载；可直接编辑该文件。",
                path.display()
            ));
            self.reload_menu();
            return;
        }
        match std::fs::write(&path, SAMPLE_MENU) {
            Ok(()) => {
                self.notice = Some(format!("已生成模板：{}", path.display()));
                self.reload_menu();
            }
            Err(err) => self.notice = Some(format!("写入失败：{err}")),
        }
    }

    fn open_menu_dir(&mut self) {
        let dir = self
            .menu
            .custom_path
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .or_else(|| {
                wte::menu::default_custom_path().and_then(|p| p.parent().map(|d| d.to_path_buf()))
            });
        match dir {
            Some(dir) => {
                if let Err(err) = open_folder(&dir) {
                    self.notice = Some(format!(
                        "无法打开文件夹：{err}（进程完整性：{}）",
                        process_integrity()
                    ));
                }
            }
            None => self.notice = Some("找不到菜单目录".to_owned()),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        match self.screen {
            Screen::Welcome => self.ui_welcome(ui),
            Screen::Quiz => self.ui_quiz(ui),
            Screen::Result => self.ui_result(ui),
            Screen::MenuInfo => self.ui_menu_info(ui),
        }
    }

    fn ui_welcome(&mut self, ui: &mut egui::Ui) {
        ui.add_space(52.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("今天吃什么").size(44.0).strong());
            ui.add_space(4.0);
            ui.label(RichText::new("五个小问题，帮你在外卖里选一道主食").size(17.0));
            ui.add_space(10.0);

            let custom = if self.menu.custom_count > 0 {
                format!("，其中 {} 条来自你的 menu.txt", self.menu.custom_count)
            } else {
                String::new()
            };
            ui.label(
                RichText::new(format!("题库共 {} 道主食{}", self.menu.dishes.len(), custom))
                    .size(14.0)
                    .weak(),
            );

            ui.add_space(40.0);
            if ui
                .add_sized(
                    [240.0, 54.0],
                    egui::Button::new(RichText::new("开始选").size(21.0)),
                )
                .clicked()
            {
                self.start_quiz();
            }
            ui.add_space(10.0);
            if ui
                .add_sized([240.0, 40.0], egui::Button::new("菜品库 / 自定义菜单"))
                .clicked()
            {
                self.notice = None;
                self.screen = Screen::MenuInfo;
            }

            ui.add_space(24.0);
            ui.label(
                RichText::new("完全离线 · 不联网 · 不接 AI")
                    .size(13.0)
                    .weak(),
            );
        });
    }

    fn ui_quiz(&mut self, ui: &mut egui::Ui) {
        let question = ALL_QUESTIONS[self.step];

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            for (i, _) in ALL_QUESTIONS.iter().enumerate() {
                let color = if i == self.step {
                    ui.visuals().selection.bg_fill
                } else if i < self.step {
                    egui::Color32::from_gray(150)
                } else {
                    egui::Color32::from_gray(210)
                };
                ui.label(RichText::new("●").size(12.0).color(color));
            }
            ui.add_space(10.0);
            ui.label(
                RichText::new(format!("第 {} / {} 题", self.step + 1, ALL_QUESTIONS.len()))
                    .size(14.0)
                    .weak(),
            );
        });

        ui.add_space(26.0);
        ui.label(RichText::new(question.prompt()).size(30.0).strong());
        ui.add_space(6.0);
        ui.label(
            RichText::new("点选项会自动进入下一题；想改的话用「上一步」退回")
                .size(14.0)
                .weak(),
        );
        ui.add_space(18.0);

        let width = ui.available_width();
        let mut picked: Option<Answer> = None;
        for choice in question.choices() {
            let selected = self.pending == Some(choice.answer);
            let response = ui.add_sized(
                [width, 46.0],
                egui::Button::selectable(selected, choice.label),
            );
            if response.clicked() {
                picked = Some(choice.answer);
            }
            ui.add_space(2.0);
        }

        if let Some(answer) = picked {
            self.choose(answer);
            return;
        }

        ui.add_space(22.0);
        ui.horizontal(|ui| {
            if self.step > 0
                && ui
                    .add_sized([110.0, 42.0], egui::Button::new("上一步"))
                    .clicked()
            {
                self.back();
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let label = if self.step + 1 == ALL_QUESTIONS.len() {
                    "看看结果"
                } else {
                    "下一步"
                };
                if ui
                    .add_enabled(
                        self.pending.is_some(),
                        egui::Button::new(label).min_size(egui::vec2(120.0, 42.0)),
                    )
                    .clicked()
                {
                    self.advance();
                }
            });
        });
    }

    fn ui_result(&mut self, ui: &mut egui::Ui) {
        let Some(rec) = self.rec.clone() else {
            ui.label("还没有结果");
            if ui.button("回到开始").clicked() {
                self.screen = Screen::Welcome;
            }
            return;
        };

        ui.add_space(12.0);
        ui.label(RichText::new("今天就吃").size(15.0).weak());
        ui.add_space(2.0);
        ui.label(RichText::new(rec.dish.name.as_str()).size(40.0).strong());
        ui.add_space(8.0);
        ui.label(RichText::new(rec.dish.tags()).size(15.0));

        if !rec.dish.note.is_empty() {
            ui.add_space(10.0);
            ui.label(RichText::new(format!("提示：{}", rec.dish.note)).size(15.0));
        }

        if !rec.relaxed.is_empty() {
            let names = rec
                .relaxed
                .iter()
                .map(|q| q.label())
                .collect::<Vec<_>>()
                .join("、");
            ui.add_space(10.0);
            ui.label(
                RichText::new(format!("已放宽：{names}（没有完全符合的，给你最接近的）"))
                    .size(14.0)
                    .color(egui::Color32::from_rgb(190, 110, 20)),
            );
        }

        ui.add_space(16.0);
        ui.label(
            RichText::new(format!("符合条件的候选：{} 道", rec.candidate_count))
                .size(13.0)
                .weak(),
        );

        if !rec.alternatives.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new("备选：").size(14.0).weak());
            for alt in &rec.alternatives {
                ui.label(RichText::new(format!("· {}", alt.name)).size(15.0));
            }
        }

        ui.add_space(24.0);
        ui.horizontal(|ui| {
            if ui
                .add_sized([120.0, 42.0], egui::Button::new("换一个"))
                .clicked()
            {
                self.draw_result();
            }
            if ui
                .add_sized([120.0, 42.0], egui::Button::new("重新选"))
                .clicked()
            {
                self.start_quiz();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_sized([110.0, 42.0], egui::Button::new("菜品库"))
                    .clicked()
                {
                    self.notice = None;
                    self.screen = Screen::MenuInfo;
                }
            });
        });
    }

    fn ui_menu_info(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("菜品库").size(30.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("返回").clicked() {
                    self.screen = if self.rec.is_some() {
                        Screen::Result
                    } else {
                        Screen::Welcome
                    };
                }
            });
        });

        ui.add_space(4.0);
        ui.label(format!(
            "内置 {} 道 · 自定义 {} 条 · 合计 {} 道",
            self.menu.builtin_count,
            self.menu.custom_count,
            self.menu.dishes.len()
        ));

        let path = self
            .menu
            .custom_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "未找到 menu.txt（可选）".to_owned());
        ui.label(RichText::new(format!("菜单文件：{path}")).size(13.0).weak());
        ui.label(
            RichText::new(format!("运行环境完整性：{}", process_integrity()))
                .size(12.0)
                .weak(),
        );

        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "格式：名称|口味(甜/咸/皆)|汤水(汤/干/皆)|主食(饭/面/其他)|辣度(0-2)|餐段(早/午/晚/皆)|备注",
            )
            .size(13.0)
            .weak(),
        );

        ui.add_space(10.0);
        if self.menu.issues.is_empty() {
            ui.label(RichText::new("菜单解析正常，没有发现问题行").size(13.0).weak());
        } else {
            ui.label(
                RichText::new(format!("有 {} 行没有读进来：", self.menu.issues.len()))
                    .size(14.0)
                    .color(egui::Color32::from_rgb(180, 60, 60)),
            );
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for issue in &self.menu.issues {
                        ui.label(RichText::new(issue.describe()).size(13.0));
                    }
                });
        }

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("重新加载").clicked() {
                self.reload_menu();
            }
            if ui.button("生成菜单模板").clicked() {
                self.create_template();
            }
            if ui.button("打开所在文件夹").clicked() {
                self.open_menu_dir();
            }
        });

        if let Some(notice) = &self.notice {
            ui.add_space(10.0);
            ui.label(RichText::new(notice).size(13.0));
        }
    }
}

impl eframe::App for WteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| self.ui(ui));
    }
}

fn setup_style(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::light());

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (egui::TextStyle::Heading, egui::FontId::proportional(30.0)),
        (egui::TextStyle::Body, egui::FontId::proportional(17.0)),
        (egui::TextStyle::Button, egui::FontId::proportional(17.0)),
        (egui::TextStyle::Small, egui::FontId::proportional(13.0)),
        (egui::TextStyle::Monospace, egui::FontId::monospace(15.0)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 10.0);
    style.spacing.button_padding = egui::vec2(16.0, 9.0);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(235, 146, 58);
    ctx.set_style(style);
}

/// 读取当前进程的完整性级别，用于排查「打开文件夹」被拒绝的问题。
#[cfg(windows)]
fn process_integrity() -> &'static str {
    use std::ffi::c_void;
    use std::ptr;

    #[repr(C)]
    struct SidAndAttributes {
        sid: *mut c_void,
        attributes: u32,
    }
    #[repr(C)]
    struct TokenMandatoryLabel {
        label: SidAndAttributes,
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn OpenProcessToken(
            process_handle: *mut c_void,
            desired_access: u32,
            token_handle: *mut *mut c_void,
        ) -> i32;
        fn GetTokenInformation(
            token_handle: *mut c_void,
            class: u32,
            info: *mut c_void,
            len: u32,
            ret_len: *mut u32,
        ) -> i32;
        fn GetSidSubAuthorityCount(sid: *mut c_void) -> *mut u8;
        fn GetSidSubAuthority(sid: *mut c_void, index: u32) -> *mut u32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
    }

    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_INTEGRITY_LEVEL: u32 = 25;

    unsafe {
        let mut token: *mut c_void = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return "未知";
        }

        let mut buffer = [0u64; 32];
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TOKEN_INTEGRITY_LEVEL,
            buffer.as_mut_ptr() as *mut c_void,
            std::mem::size_of_val(&buffer) as u32,
            &mut returned,
        );

        let mut label = "未知";
        if ok != 0 {
            let info = &*(buffer.as_ptr() as *const TokenMandatoryLabel);
            let count = *GetSidSubAuthorityCount(info.label.sid);
            if count > 0 {
                let rid = *GetSidSubAuthority(info.label.sid, (count - 1) as u32);
                label = match rid {
                    0x0000 => "Untrusted",
                    0x1000 => "Low",
                    0x2000 => "Medium",
                    0x3000 => "High",
                    0x4000 => "System",
                    _ => "Other",
                };
            }
        }

        let _ = CloseHandle(token);
        label
    }
}

#[cfg(not(windows))]
fn process_integrity() -> &'static str {
    "n/a"
}

/// 用系统 Shell 打开文件夹。
///
/// 不要直接 `Command::new("explorer").arg(dir)`：在某些环境（例如进程带提权令牌）下，
/// 新起的 explorer.exe 会初始化失败，弹出「应用程序无法正常启动 (0xc0000142)」。
/// `ShellExecuteW` 会把请求交给已经在运行的资源管理器，更稳。
#[cfg(windows)]
fn open_folder(path: &std::path::Path) -> std::io::Result<()> {
    use std::ffi::{c_void, OsStr};
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show_cmd: i32,
        ) -> isize;
    }

    fn wide(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    let operation = wide(OsStr::new("open"));
    let file = wide(path.as_os_str());

    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            ptr::null(),
            ptr::null(),
            1,
        )
    };

    if result > 32 {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "ShellExecuteW 失败（代码 {result}）"
        )))
    }
}

#[cfg(not(windows))]
fn open_folder(path: &std::path::Path) -> std::io::Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(path)
        .spawn()
        .map(|_| ())
}

fn install_chinese_font(ctx: &egui::Context) {
    // egui 自带字体不含中文，这里加载系统字体；找不到时退回黑体。
    let candidates = [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyh.ttf",
        "C:\\Windows\\Fonts\\simhei.ttf",
    ];

    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "cjk".to_owned(),
                std::sync::Arc::new(egui::FontData {
                    font: bytes.into(),
                    index: 0,
                    tweak: Default::default(),
                }),
            );
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .insert(0, "cjk".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}
