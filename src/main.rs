#![allow(warnings, unused)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::borrow::BorrowMut;
use std::fmt::Display;
use std::net::SocketAddr;
use std::ops::{Index, IndexMut};

use anyhow::anyhow;
use api_test_rs::{
    AppConfig, Group, HttpConfig, Method, PairUi, RequestBodyRawType, RequestBodyTab, RequestTab,
    ResponseTab,
};
use eframe::egui::output::OpenUrl;
use eframe::egui::{CollapsingHeader, TextBuffer};
use eframe::epaint::vec2;
use eframe::{
    egui::{self, Hyperlink, RichText, Ui},
    epaint::Color32,
};
use egui_extras::RetainedImage;
use poll_promise::Promise;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::Runtime;

mod util;
mod widget;

/* #region const variables */
const SAVE_DIR: &str = "./_SAVED/";
const METHODS: [Method; 9] = [
    Method::GET,
    Method::POST,
    Method::PUT,
    Method::DELETE,
    Method::HEAD,
    Method::OPTIONS,
    Method::CONNECT,
    Method::TRACE,
    Method::PATCH,
];
const REQ_TABS: [RequestTab; 3] = [RequestTab::Params, RequestTab::Headers, RequestTab::Body];
const REQ_BODY_TABS: [RequestBodyTab; 3] = [
    RequestBodyTab::Raw,
    RequestBodyTab::Form,
    RequestBodyTab::FormData,
];
const REQ_BODY_RAW_TYPES: [RequestBodyRawType; 5] = [
    RequestBodyRawType::Json,
    RequestBodyRawType::Text,
    RequestBodyRawType::Form,
    RequestBodyRawType::XML,
    RequestBodyRawType::BinaryFile,
];
const COLUMN_WIDTH_INITIAL: f32 = 200.0;
const RESPONSE_TABS: [ResponseTab; 2] = [ResponseTab::Data, ResponseTab::Header];
/* #endregion */

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let save_dir = std::path::Path::new(SAVE_DIR);
    if !save_dir.exists() {
        std::fs::create_dir_all(save_dir);
    }

    let mut options = eframe::NativeOptions::default();
    options.icon_data = Some(util::load_app_icon());

    // options.initial_window_pos = Some([0f32, 0f32].into());
    options.min_window_size = Some([900.0, 600.0].into());

    // options.fullscreen = true;
    options.maximized = false;

    let config: Option<AppConfig> = AppConfig::load(SAVE_DIR).ok();

    eframe::run_native(
        "api test",
        options,
        Box::new(|cc| Box::new(ApiTestApp::new(cc, config))),
    )
}

struct ApiTestApp {
    rt: Runtime,

    // 项目名称
    project_name: String,

    // 加载保存的项目文件路径
    project_path: String,
    select_api_test_index: Option<(usize, usize)>,

    new_project_name: String,
    new_group_name: String,
    del_group_name: String,
    groups: Vec<Group>,
    action_status: String,
    thread_count: String,

    // 已保存的项目
    saved: Vec<(String, String)>,
}

impl Default for ApiTestApp {
    fn default() -> Self {
        let init_thread_count = 1;

        Self {
            project_name: Default::default(),
            project_path: Default::default(),
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(init_thread_count)
                .build()
                .unwrap(),
            groups: vec![],
            new_group_name: Default::default(),
            new_project_name: Default::default(),
            select_api_test_index: None,
            action_status: Default::default(),
            saved: Default::default(),
            del_group_name: Default::default(),
            thread_count: init_thread_count.to_string(),
        }
    }
}

impl ApiTestApp {
    fn new(cc: &eframe::CreationContext<'_>, config: Option<AppConfig>) -> Self {
        util::setup_custom_fonts(&cc.egui_ctx);
        let mut my = Self::default();

        if let Some(config) = config {
            my.project_path = config.project_path;
            my.load_project();
        }
        my
    }

    /// 保存当前正在操作的项目
    fn save_current_project(&mut self) {
        self.action_status =
            match util::save_current_project(SAVE_DIR, &self.project_name, &self.groups) {
                Ok(_) => "save sucsess".to_owned(),
                Err(err) => err.to_string(),
            };
    }

    /// 获取保存的project文件列表
    fn load_saved_project(&mut self) -> anyhow::Result<Vec<(String, String)>> {
        let dir = std::fs::read_dir(SAVE_DIR)?;
        Ok(dir
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(|e| if e.path().is_file() { Some(e) } else { None })
            .filter_map(|e| match e.file_name().into_string() {
                Ok(file_name) => Some((file_name, e)),
                Err(_) => None,
            })
            .filter_map(|e| if e.0.starts_with(".") { None } else { Some(e) })
            .map(|(file_name, e)| {
                let file_stem = e.path().file_stem().unwrap().to_str().unwrap().to_string();
                let path = e.path().to_str().unwrap().to_string();

                (file_stem, path)
            })
            .collect())
    }

    /// 创建一个新项目，保存当前正在操作的项目
    fn create_project(&mut self) {
        if !self.project_name.is_empty() {
            self.save_current_project();
        }

        self.project_name = self.new_project_name.to_owned();
        self.groups.clear();
        self.select_api_test_index = None;
        self.new_project_name.clear(); // clear input name
        self.project_path.clear(); // new project not save
    }

    /// 加载一个项目
    fn load_project(&mut self) {
        match util::load_project(&self.project_path) {
            Ok((project_name, data)) => {
                self.groups = data;
                self.project_name = project_name;
                self.select_api_test_index = None;
                self.action_status = "Load project success".to_owned();
            }
            Err(err) => {
                self.action_status = err.to_string();
            }
        }
    }
}

impl eframe::App for ApiTestApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_current_project();
        self.action_status = "auto save".to_owned();
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        /* #region top menus */
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });

                ui.menu_button("Project", |ui| {
                    ui.horizontal(|ui| {
                        let input = ui.add(
                            egui::TextEdit::singleline(&mut self.new_project_name)
                                .hint_text("Enter Create Project"),
                        );

                        if input.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !self.new_project_name.is_empty()
                        {
                            self.create_project();
                        }
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        let input_add = ui.add(
                            egui::TextEdit::singleline(&mut self.new_group_name)
                                .hint_text("Enter Add Group"),
                        );

                        if input_add.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !self.new_group_name.is_empty()
                        {
                            let name = self.new_group_name.to_owned();
                            let name_exists = self.groups.iter().any(|el| el.name == name);

                            if !name_exists {
                                self.groups
                                    .push(Group::from_name(self.new_group_name.to_owned()));
                                self.new_group_name.clear();
                                self.action_status = "create success".to_owned();
                                input_add.request_focus();
                            } else {
                                self.action_status = "name exists".to_owned();
                            }
                        }
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        let input_del = ui.add(
                            egui::TextEdit::singleline(&mut self.del_group_name)
                                .hint_text("Enter Del Group"),
                        );

                        if input_del.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !self.del_group_name.is_empty()
                        {
                            let name = self.del_group_name.to_owned();
                            let name_exists = self.groups.iter().position(|el| el.name == name);

                            if let Some(index) = name_exists {
                                self.select_api_test_index = None;
                                self.groups.remove(index);
                                self.del_group_name.clear();

                                self.action_status = "delete success".to_owned();
                                input_del.request_focus();
                            } else {
                                self.action_status = "name not exists".to_owned();
                            }
                        }
                    });

                    ui.separator();

                    if ui.button("Save Current Project").clicked() {
                        self.save_current_project();
                        ui.close_menu();
                    }
                });

                let saved_menu = ui.menu_button("Saved", |ui| {
                    ui.vertical(|ui| {
                        for i in 0..self.saved.len() {
                            let (name, path) = self.saved.index(i);
                            if ui.selectable_label(false, name).clicked() {
                                self.project_path = path.to_owned();
                                self.load_project();
                                ui.close_menu();
                            }

                            ui.separator();
                        }
                    });
                });

                if saved_menu.response.clicked() {
                    if let Ok(saved) = self.load_saved_project() {
                        self.saved = saved;
                    }
                }

                ui.menu_button("Setting", |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Thread Count");
                            ui.text_edit_singleline(&mut self.thread_count);
                            if ui.button("Set").clicked() {
                                match self.thread_count.parse::<usize>() {
                                    Ok(count) => {
                                        match tokio::runtime::Builder::new_multi_thread()
                                            .enable_all()
                                            .worker_threads(count)
                                            .build()
                                        {
                                            Ok(rt) => {
                                                self.rt = rt;
                                                self.action_status =
                                                    "thread count set success".to_owned();
                                                ui.close_menu();
                                            }
                                            Err(err) => {
                                                self.action_status = err.to_string();
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        self.action_status = err.to_string();
                                    }
                                }
                            }
                        });
                    });
                });
            });
        });
        /* #endregion */

        /* #region left panel */
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(220.0)
            .width_range(80.0..=600.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(&self.project_name);
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.groups
                        .iter_mut()
                        .enumerate()
                        .for_each(|(group_index, group)| {
                            ui.separator();
                            CollapsingHeader::new(&group.name)
                                .default_open(false)
                                .show(ui, |ui| {
                                    ui.menu_button("...", |ui| {
                                        let input_add = ui.add(
                                            egui::TextEdit::singleline(&mut group.new_child_name)
                                                .desired_width(120.0)
                                                .hint_text("Enter Add Test"),
                                        );

                                        if input_add.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                            && !group.new_child_name.is_empty()
                                        {
                                            group.create_child();
                                            self.action_status = "添加成功".to_owned();
                                            input_add.request_focus();
                                        }

                                        ui.separator();

                                        let input_del = ui.add(
                                            egui::TextEdit::singleline(&mut group.del_child_name)
                                                .desired_width(120.0)
                                                .hint_text("Enter Del Test"),
                                        );

                                        if input_del.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                            && !group.del_child_name.is_empty()
                                        {
                                            self.select_api_test_index = None;
                                            group.del_child();
                                            input_del.request_focus();
                                        }

                                        // TODO:
                                        ui.text_edit_singleline(&mut group.name);
                                    });
                                    ui.separator();

                                    ui.with_layout(
                                        egui::Layout::top_down_justified(egui::Align::Min),
                                        |ui| {
                                            group.childrent.iter().enumerate().rev().for_each(
                                                |(cfg_i, cfg)| {
                                                    let checked = match self.select_api_test_index {
                                                        Some((i, j)) => {
                                                            i == group_index && j == cfg_i
                                                        }
                                                        _ => false,
                                                    };

                                                    if ui
                                                        .selectable_label(checked, &cfg.name)
                                                        .clicked()
                                                    {
                                                        self.select_api_test_index =
                                                            Some((group_index, cfg_i));
                                                    }
                                                    ui.separator();
                                                },
                                            );
                                        },
                                    );
                                });
                        });
                });
            });
        /* #endregion */

        // egui::SidePanel::right("right_panel")
        //     .resizable(true)
        //     .default_width(150.0)
        //     .width_range(80.0..=200.0)
        //     .show(ctx, |ui| {
        //         ui.vertical_centered(|ui| {
        //             ui.heading("Right Panel");
        //         });
        //         egui::ScrollArea::vertical().show(ui, |ui| {
        //             ui.label("text");
        //         });
        //     });

        /* #region center panel */
        egui::CentralPanel::default().show(ctx, |ui| {
            /* #region action bar */
            egui::TopBottomPanel::bottom("bottom_panel")
                .resizable(false)
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Action Status:");
                        ui.label(&self.action_status);
                    });
                });
            /* #endregion */

            if let Some(ii) = self.select_api_test_index {
                let group = &mut self.groups[ii.0];
                let hc = &mut group.childrent[ii.1];

                ui.horizontal(|ui| {
                    egui::TextEdit::singleline(&mut hc.name)
                        .desired_width(100.0)
                        .show(ui);
                    egui::ComboBox::from_id_source("method")
                        .selected_text(hc.req_cfg.method.as_ref())
                        .show_ui(ui, |ui| {
                            for m in &METHODS {
                                ui.selectable_value(
                                    &mut hc.req_cfg.method,
                                    m.to_owned(),
                                    m.as_ref(),
                                );
                            }
                        });

                    ui.add(
                        egui::TextEdit::singleline(&mut hc.req_cfg.url)
                            .desired_width(400.)
                            .hint_text("http url"),
                    );

                    ui.add(
                        egui::TextEdit::singleline(&mut hc.send_count_ui)
                            .desired_width(60.)
                            .hint_text("Count"),
                    );

                    if ui
                        .add_enabled(!hc.req_cfg.url.is_empty(), egui::Button::new("Send"))
                        .clicked()
                    {
                        hc.response_promise = None;
                        hc.response_promise_vec.clear();
                        hc.s_e_r = (0, 0, 0);
                        hc.send_count = hc.send_count_ui.parse().unwrap_or(0);
                        for _ in 0..hc.send_count {
                            hc.response_promise_vec
                                .push(HttpConfig::http_send_promise(&self.rt, hc.req_cfg.clone()));
                        }
                    }

                    ui.separator();

                    let (s, e, r) = hc.get_request_reper();
                    ui.label(format!("s:{}, e:{}, r:{}", s, e, r,));

                    ui.separator();
                });
                ui.separator();

                widget::horizontal_tabs(ui, REQ_TABS.iter(), &mut hc.tab_ui);
                ui.separator();

                match hc.tab_ui {
                    RequestTab::Params => {
                        widget::pair_table(ui, "param scroll", &mut hc.req_cfg.query);
                    }
                    RequestTab::Headers => {
                        widget::pair_table(ui, "header scroll", &mut hc.req_cfg.header);
                    }
                    RequestTab::Body => {
                        widget::horizontal_tabs(
                            ui,
                            REQ_BODY_TABS.iter(),
                            &mut hc.req_cfg.body_tab_ui,
                        );
                        ui.separator();

                        match hc.req_cfg.body_tab_ui {
                            RequestBodyTab::Raw => {
                                ui.vertical(|ui| {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            for (i, raw_type) in
                                                REQ_BODY_RAW_TYPES.iter().enumerate()
                                            {
                                                ui.radio_value(
                                                    &mut hc.req_cfg.body_raw_type,
                                                    raw_type.to_owned(),
                                                    raw_type.as_ref(),
                                                );
                                            }
                                        });
                                    });

                                    egui::ScrollArea::vertical()
                                        .id_source("row data scroll")
                                        .max_height(120.0)
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::TextEdit::multiline(&mut hc.req_cfg.body_raw)
                                                    .desired_rows(6),
                                            );
                                        });
                                });
                            }

                            RequestBodyTab::Form => {
                                widget::pair_table(
                                    ui,
                                    "body_form scroll",
                                    &mut hc.req_cfg.body_form,
                                );
                            }

                            RequestBodyTab::FormData => {
                                widget::pair_table(
                                    ui,
                                    "body_form scroll",
                                    &mut hc.req_cfg.body_form_data,
                                );
                            }
                        }
                    }
                };

                ui.separator();

                if let Some(response_promise) = &hc.response_promise {
                    match response_promise.ready() {
                        // if !hc.response_promise_vec.is_empty() {
                        //     match hc.response_promise_vec.first().unwrap().ready() {
                        Some(response) => match response {
                            Ok(response) => {
                                ui.horizontal(|ui| {
                                    ui.heading(format!(
                                        "Response Status: {:?} {}",
                                        response.version, response.status
                                    ));

                                    ui.separator();

                                    if let Some(data_vec) = &response.data_vec {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut hc.download_path)
                                                .hint_text(r#"c:/out.jpg"#),
                                        );
                                        if ui
                                            .add_enabled(
                                                !hc.download_path.is_empty(),
                                                egui::Button::new("Download Body"),
                                            )
                                            .clicked()
                                        {
                                            match util::download(&hc.download_path, data_vec) {
                                                Ok(_) => {
                                                    self.action_status = "Downlaod Ok".to_owned();
                                                }
                                                Err(err) => {
                                                    self.action_status = err.to_string();
                                                }
                                            }
                                        }
                                    }
                                });
                                ui.separator();

                                widget::horizontal_tabs(
                                    ui,
                                    RESPONSE_TABS.iter(),
                                    &mut hc.response_tab_ui,
                                );
                                ui.separator();

                                match hc.response_tab_ui {
                                    ResponseTab::Data => {
                                        egui::ScrollArea::vertical()
                                            .hscroll(true)
                                            .id_source("response data scroll")
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                if let Some(img_data) = &response.img {
                                                    match img_data {
                                                        Ok(image) => {
                                                            image.show(ui);
                                                        }
                                                        Err(err) => {
                                                            widget::error_label(
                                                                ui,
                                                                &err.to_string(),
                                                            );
                                                        }
                                                    }
                                                } else if let Some(text_data) = &response.data {
                                                    widget::code_view_ui(ui, text_data);
                                                } else {
                                                    widget::error_label(ui, "其他类型");
                                                }
                                            });
                                    }
                                    ResponseTab::Header => {
                                        egui::ScrollArea::vertical()
                                            .hscroll(true)
                                            .id_source("response heaer scroll")
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                ui.vertical(|ui| {
                                                    response.headers.iter().for_each(
                                                        |(name, val)| {
                                                            let name = name.as_str();
                                                            let value = val.to_str().unwrap_or("");
                                                            widget::code_view_ui(
                                                                ui,
                                                                &format!("{}: {}", name, value),
                                                            );
                                                        },
                                                    );
                                                });
                                            });
                                    }
                                }
                            }
                            Err(err) => {
                                widget::error_label(ui, &err.to_string());
                            }
                        },
                        _ => {
                            ui.spinner();
                        }
                    }
                };
            }
        });
        /* #endregion */
    }
}
