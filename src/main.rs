// #![allow(warnings, unused)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::ops::Index;

use api_test_rs::*;
use eframe::egui::CollapsingHeader;
use eframe::egui::{self};
use eframe::epaint::{Color32, vec2};
use egui_extras::RetainedImage;
use tokio::runtime::Runtime;

mod util;
mod widget;

/* #region const variables */
const SEND_THREAD_COUN:usize = 2;
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
        std::fs::create_dir_all(save_dir).unwrap();
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

    // 加载保存的项目文件路径
    project_path: String,
    select_api_test_index: Option<(usize, usize)>,

    new_project_name: String,
    new_group_name: String,
    del_group_name: String,

    // 当前项目
    project: Project,

    action_status: String,
    thread_count: String,

    // 已保存的项目 (name, path)
    saved: Vec<(String, String)>,
}

impl Default for ApiTestApp {
    fn default() -> Self {
        Self {
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(SEND_THREAD_COUN)
                .build()
                .unwrap(),
            new_group_name: Default::default(),
            new_project_name: Default::default(),
            action_status: Default::default(),
            saved: Default::default(),
            del_group_name: Default::default(),
            thread_count: SEND_THREAD_COUN.to_string(),
            project_path: Default::default(),
            select_api_test_index: Some((0, 0)),
            project: Project {
                name: "Any".to_owned(),
                groups: vec![{
                        let mut g =   Group::from_name("Group #1".to_owned());
                        let mut t = HttpConfig::from_name("test".to_owned());
                        t.req_cfg.url = "{{base}}/ping".to_owned();
                        g.childrent.push(t);
                        g
                    }
                ],
                variables: vec![PairUi::from_kv("base", "http://127.0.0.1:3000")],
            },
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
            my.select_api_test_index = None;
        }
        my
    }

    /// 保存当前正在操作的项目
    fn save_current_project(&mut self) {
            self.action_status = match util::save_project( SAVE_DIR, &self.project) {
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
            .filter_map(|e| {
                if e.0.starts_with(".") {
                    None
                } else {
                    Some(e.1)
                }
            })
            .map(|e| {
                let file_stem = e.path().file_stem().unwrap().to_str().unwrap().to_string();
                let path = e.path().to_str().unwrap().to_string();

                (file_stem, path)
            })
            .collect())
    }

    /// 创建一个新项目，保存当前正在操作的项目
    fn create_project(&mut self) {
        self.save_current_project();

        self.project = Project::from_name(&self.new_project_name);

        self.select_api_test_index = None;
        self.new_project_name.clear(); // clear input name
        self.project_path.clear(); // new project not save
    }

    /// 加载一个项目
    fn load_project(&mut self) {
        match util::load_project(&self.project_path) {
            Ok(project) => {
                self.project = project;
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
                        ui.style_mut().visuals.override_text_color = Some(Color32::GREEN);
                        let input_add = ui.add(
                            egui::TextEdit::singleline(&mut self.new_group_name)
                                .hint_text("Enter Add Group"),
                        );

                        if input_add.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !self.new_group_name.is_empty()
                        {
                            let name = self.new_group_name.to_owned();
                            let name_exists = self.project.groups.iter().any(|el| el.name == name);

                            if !name_exists {
                                self.project.groups
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
                        ui.style_mut().visuals.override_text_color = Some(Color32::RED);
                        let input_del = ui.add(
                            egui::TextEdit::singleline(&mut self.del_group_name)
                                .hint_text("Enter Del Group"),
                        );

                        if input_del.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !self.del_group_name.is_empty()
                        {
                            let name = self.del_group_name.to_owned();
                            let name_exists = self.project.groups.iter().position(|el| el.name == name);

                            if let Some(index) = name_exists {
                                self.select_api_test_index = None;
                                self.project.groups.remove(index);
                                self.del_group_name.clear();

                                self.action_status = "delete success".to_owned();
                                input_del.request_focus();
                            } else {
                                self.action_status = "name not exists".to_owned();
                            }
                        }
                    });

                    ui.separator();

                   
                        if ui.add(egui::Button::new("Save Current Project").min_size(vec2( ui.max_rect().width(), 30.0))).clicked() {
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
            .width_range(30.0..=600.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(&self.project.name);
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    CollapsingHeader::new("Variables")
                    .default_open(false)
                    .show(ui, |ui| {

                        ui.vertical(|ui| {
                            if ui.button("Add").clicked() {
                                self.project.variables.push(PairUi::default());
                            }
                        });
                    
                        ui.separator();
                    
                        egui_extras::StripBuilder::new(ui)
                            .size(egui_extras::Size::remainder().at_most(120.0))
                            .vertical(|mut strip| {
                                strip.cell(|ui| {
                                        let  table = egui_extras::TableBuilder::new(ui)
                                            .striped(true)
                                            .resizable(true)
                                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                            .column(egui_extras::Column::auto())
                                            .column(egui_extras::Column::auto().range(100.0..=400.0))
                                            .column(egui_extras::Column::auto().range(100.0..=400.0))
                                            .column(egui_extras::Column::auto())
                                            .min_scrolled_height(10.0)
                                            // .scroll_to_row(1, Some(egui::Align::BOTTOM))
                                            ;
                    
                                        table
                                            .header(20.0, |mut header| {
                                                header.col(|ui| {
                                                    ui.strong("");
                                                });
                                                header.col(|ui| {
                                                    ui.strong("Key");
                                                });
                                                header.col(|ui| {
                                                    ui.strong("Value");
                                                });
                                            })
                                            .body(|mut body| {
                                                self.project.variables.retain_mut(|el| {
                                                    let mut r = true;
                    
                                                    body.row(30.0, |mut row| {
                                                        row.col(|ui| {
                                                            ui.checkbox(&mut el.disable, "");
                                                        });
                    
                                                        row.col(|ui| {
                                                            ui.add(
                                                                egui::TextEdit::singleline(&mut el.key)
                                                                    .desired_width(f32::INFINITY),
                                                            );
                                                        });
                    
                                                        row.col(|ui| {
                                                            ui.add(
                                                                egui::TextEdit::singleline(&mut el.value)
                                                                    .desired_width(f32::INFINITY),
                                                            );
                                                        });
                    
                                                        row.col(|ui| {
                                                            if ui.button("Del").clicked() {
                                                                r = false;
                                                            }
                                                        });
                                                    });
                                                    r
                                                });
                                            })
                                });
                            });
                    });
                    ui.separator();

                    self.project.groups
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
        //     .default_width(120.0)
        //     .width_range(0.0..=400.0)
        //     .show(ctx, |ui| {
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
                let group = &mut self.project.groups[ii.0];
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
                        // get send count
                        hc.send_count = hc.send_count_ui.parse().unwrap_or(0);
                        let capacity = hc.send_count;

                        // clear old data
                        hc.response_promise = None;
                        hc.response_promise_vec.clear();
                        hc.s_e_r = (0, 0, 0);

                        // init result vec size
                        hc.response_promise_vec = Vec::with_capacity(capacity);

                        for _ in 0..capacity {
                            hc.response_promise_vec.push(util::http_send_promise(&self.rt, hc.req_cfg.to_owned(), self.project.variables.to_owned() ));
                        }
                    }

                    ui.separator();

                    // request result count
                    let (s, e, r) = hc.get_request_reper();
                    ui.label(format!("s:{s}, e:{e}, r:{r}"));

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
                                            REQ_BODY_RAW_TYPES.iter().for_each(|raw_type| {
                                                ui.radio_value(
                                                    &mut hc.req_cfg.body_raw_type,
                                                    raw_type.to_owned(),
                                                    raw_type.as_ref(),
                                                );
                                            });
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

                if let Some(promise) = &mut hc.response_promise {
                    match promise.read_mut() {
                        PromiseStatus::PADING => {
                            ui.spinner();
                        }
                        PromiseStatus::Rejected(err) => {
                            widget::error_label(ui, &err.to_string());
                        }
                        PromiseStatus::Fulfilled(response) => match response {
                            Ok(response) => {
                                // 初始化图片或则字符串数据
                                if let Some(data_vec) = &response.data_vec {
                                    if response.content_type_image() {
                                        response.img.get_or_insert_with(|| RetainedImage::from_image_bytes("", data_vec.as_ref()) );
                                    } else {
                                        response.data.get_or_insert_with(||
                                            std::str::from_utf8(data_vec.as_ref())
                                                .unwrap_or("")
                                                .to_owned()
                                        );
                                    }
                                }

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
                                    ResponseTab::Data => match &response.data_vec {
                                        Some(_) => {
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
                                        _ => {
                                            widget::error_label(ui, "NOT DATA");
                                        }
                                    },
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
                    }
                };
            }
        });
        /* #endregion */
    }
}
