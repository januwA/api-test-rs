#![allow(warnings, unused)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use anyhow::Result;
use core::f32;
use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use std::collections::BTreeMap;
use futures::stream::{FuturesUnordered, StreamExt as FuturesStreamExt};
use std::io::Read;
use num_format::{Locale, ToFormattedString};
use std::ops::Index;
use std::sync::Arc;
use std::thread; // Add this line
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::{http, Message};
use tokio_tungstenite::{connect_async, tungstenite};
// use tungstenite::{self, http, Message};

use api_test_rs::*;
use eframe::egui::style::Selection;
use eframe::egui::{self, global_theme_preference_buttons};
use eframe::egui::{CollapsingHeader, FontFamily, FontId, TextEdit, TextStyle, Theme};
use eframe::epaint::{vec2, Color32};
use image::{open, EncodableLayout};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, watch, Mutex};
use widget::error_button;

mod util;
mod widget;

/* #region const variables */
const SAVE_DIR: &str = "./_SAVED/";
const METHODS: [Method; 10] = [
    Method::GET,
    Method::POST,
    Method::PUT,
    Method::DELETE,
    Method::HEAD,
    Method::OPTIONS,
    Method::CONNECT,
    Method::TRACE,
    Method::PATCH,
    Method::WS,
];
const REQ_TABS: [RequestTab; 4] = [RequestTab::Params, RequestTab::Headers, RequestTab::Body, RequestTab::Scripts];
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
const WS_BODY_RAW_TYPES: [RequestBodyRawType; 2] =
    [RequestBodyRawType::Text, RequestBodyRawType::BinaryFile];
const COLUMN_WIDTH_INITIAL: f32 = 200.0;
const RESPONSE_TABS: [ResponseTab; 3] = [ResponseTab::Data, ResponseTab::Header, ResponseTab::Stats];
/* #endregion */

fn main() -> eframe::Result {
    env_logger::init();

    let save_dir = std::path::Path::new(SAVE_DIR);
    if !save_dir.exists() {
        std::fs::create_dir_all(save_dir).unwrap();
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_icon(util::load_app_icon())
            .with_maximized(false),
        ..Default::default()
    };

    let config: Option<AppConfig> = AppConfig::load(SAVE_DIR).ok();

    eframe::run_native(
        "api test",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(ApiTestApp::new(cc, config)))
        }),
    )
}

fn setup_custom_style(ctx: &egui::Context) {
    ctx.style_mut_of(Theme::Light, use_light_green_accent);
    ctx.style_mut_of(Theme::Dark, use_dark_purple_accent);
}

fn use_light_green_accent(style: &mut eframe::egui::Style) {
    style.visuals.hyperlink_color = Color32::from_rgb(18, 180, 85);
    style.visuals.text_cursor.stroke.color = Color32::from_rgb(28, 92, 48);
    style.visuals.selection = Selection {
        bg_fill: Color32::from_rgb(157, 218, 169),
        stroke: eframe::egui::Stroke::new(1.0, Color32::from_rgb(28, 92, 48)),
    };
}

fn use_dark_purple_accent(style: &mut eframe::egui::Style) {
    style.visuals.hyperlink_color = Color32::from_rgb(202, 135, 227);
    style.visuals.text_cursor.stroke.color = Color32::from_rgb(234, 208, 244);
    style.visuals.selection = Selection {
        bg_fill: Color32::from_rgb(105, 67, 119),
        stroke: eframe::egui::Stroke::new(1.0, Color32::from_rgb(234, 208, 244)),
    };
}

#[inline]
fn heading2() -> TextStyle {
    TextStyle::Name("Heading2".into())
}

#[inline]
fn heading3() -> TextStyle {
    TextStyle::Name("ContextHeading".into())
}

fn configure_text_styles(ctx: &egui::Context) {
    use FontFamily::{Monospace, Proportional};

    let text_styles: BTreeMap<TextStyle, FontId> = [
        (TextStyle::Heading, FontId::new(25.0, Proportional)),
        (heading2(), FontId::new(22.0, Proportional)),
        (heading3(), FontId::new(19.0, Proportional)),
        (TextStyle::Body, FontId::new(16.0, Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, Monospace)),
        (TextStyle::Button, FontId::new(12.0, Proportional)),
        (TextStyle::Small, FontId::new(12.0, Proportional)),
    ]
    .into();
    ctx.all_styles_mut(move |style| style.text_styles = text_styles.clone());
}

struct ApiTestApp {
    rt: Runtime,
    ws_tx: Option<tokio::sync::mpsc::Sender<WsMessage>>,
    ws_messages: Arc<std::sync::RwLock<Vec<Message>>>,

    http_tx: mpsc::Sender<Result<HttpResponse>>,
    http_rx: mpsc::Receiver<Result<HttpResponse>>,

    // 加载保存的项目文件路径
    project_path: String,
    remove_group: Option<usize>,

    select_test: Option<(usize, usize)>,
    remove_test: Option<(usize, usize)>,
    copy_test: Option<(usize, usize)>,

    new_project_name: String,
    new_group_name: String,

    // 当前项目
    project: Project,

    action_status: String,

    // 已保存的项目 (name, path)
    saved: Vec<(String, String)>,

    // 美化请求的返回结果，如格式化json
    is_pretty: bool,

    pub modal: ModalOptions,
    worker_thread_count: usize,
    search_filter: String,
}

impl Default for ApiTestApp {
    fn default() -> Self {
        let num_worker_threads = thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or_else(|_| {
                eprintln!("无法获取系统并行度，使用默认值 1");
                1
            });

        let (http_tx, http_rx) = mpsc::channel(100000);

        Self {
            ws_tx: Default::default(),
            http_tx,
            http_rx,
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(num_worker_threads) // Explicitly set the worker threads
                .build()
                .unwrap(),
            new_group_name: Default::default(),
            new_project_name: Default::default(),
            action_status: Default::default(),
            saved: Default::default(),
            project_path: Default::default(),
            select_test: Some((0, 0)),
            remove_test: None,
            copy_test: None,
            project: Project {
                name: "Any".to_owned(),
                groups: vec![{
                    let mut g = Group::from_name("Group #1".to_owned());
                    let mut t = HttpTest::from_name("test".to_owned());
                    t.request.url = "{{base}}/ping".to_owned();
                    g.childrent.push(t);
                    g
                }],
                variables: vec![PairUi::from_kv("base", "http://127.00.1:3000")],
            },
            is_pretty: true,
            remove_group: None,

            modal: Default::default(),
            ws_messages: Default::default(),
            worker_thread_count: num_worker_threads,
            search_filter: String::new(),
        }
    }
}

impl ApiTestApp {
    fn new(cc: &eframe::CreationContext<'_>, config: Option<AppConfig>) -> Self {
        setup_custom_style(&cc.egui_ctx);
        // configure_text_styles(&cc.egui_ctx);
        util::setup_custom_fonts(&cc.egui_ctx);

        let mut my = Self::default();

        if let Some(config) = config {
            my.project_path = config.project_path;
            my.load_project();
            my.select_test = None;
        }

        let (ws_tx, mut ws_rx) = tokio::sync::mpsc::channel::<WsMessage>(32);
        my.ws_tx = Some(ws_tx);
        let ws_msgs = my.ws_messages.clone();

        my.rt.spawn(async move {
            let ws_msgs_c = ws_msgs.clone();
            let mut _tx: Option<tokio::sync::mpsc::Sender<WsMessage>> = None;
            let mut need_init = Arc::new(Mutex::new(true));
            let mut need_init_c = need_init.clone();

            while let Some(msg) = ws_rx.recv().await {
                if !*need_init.lock().await {
                    if let Some(tx) = _tx.as_mut() {
                        tx.send(msg).await;
                    };
                    continue;
                }
                if let WsMessage::Send(cfg, variables) = msg {
                    if *need_init.lock().await {
                        let mut base_url: reqwest::Url =
                            reqwest::Url::parse(&util::parse_var_str(&cfg.url, &variables))
                                .expect("parse url");
                        // 添加查询参数
                        let request_query = util::real_tuple_vec(&cfg.query, &variables);
                        request_query.iter().for_each(|(k, v)| {
                            base_url.query_pairs_mut().append_pair(k, v);
                        });
                        let socket_uri =
                            base_url.as_str().parse::<http::Uri>().expect("parse url 2");

                        let authority = socket_uri.authority().unwrap().as_str();

                        let host = authority
                            .find('@')
                            .map(|idx| authority.split_at(idx + 1).1)
                            .unwrap_or_else(|| authority);

                        let mut req_builder = http::Request::builder()
                            .method("GET")
                            .header("Host", host)
                            .header("Connection", "Upgrade")
                            .header("Upgrade", "websocket")
                            .header("Sec-WebSocket-Version", "13")
                            .header("Sec-WebSocket-Key", generate_key())
                            .uri(socket_uri);

                        // 添加自定义header
                        let request_header = util::real_tuple_vec(&cfg.header, &variables);
                        for (k, v) in &request_header {
                            req_builder = req_builder.header(k, v);
                        }

                        let req: http::Request<()> = req_builder.body(()).unwrap();

                        match connect_async(req).await {
                            Ok((socket, _)) => {
                                let mut ni = need_init.lock().await;
                                *ni = false;

                                let (tx_w, mut rx_w) = tokio::sync::mpsc::channel::<WsMessage>(32);
                                let tx_w2 = tx_w.clone();
                                _tx = Some(tx_w);

                                let (mut w, mut r) = socket.split();

                                let ws_msgs_r = ws_msgs_c.clone();
                                let need_init_r = need_init_c.clone();
                                tokio::spawn(async move {
                                    while let Some(message) = r.next().await {
                                        match message {
                                            Ok(msg) => {
                                                ws_msgs_r.write().unwrap().push(msg);
                                            }
                                            Err(err) => {
                                                ws_msgs_r.write().unwrap().push(Message::text(
                                                    format!("> Read Error: {}", err).to_owned(),
                                                ));
                                                ws_msgs_r
                                                    .write()
                                                    .unwrap()
                                                    .push(Message::text("> Send Error: ws 已断开"));
                                                break;
                                            }
                                        }
                                    }
                                    println!("读取断开");
                                    let mut ni = need_init_r.lock().await;
                                    *ni = true;
                                    tx_w2.send(WsMessage::Close).await;
                                });

                                let ws_msgs_w = ws_msgs_c.clone();
                                let need_init_w = need_init.clone();

                                tokio::spawn(async move {
                                    while let Some(msg) = rx_w.recv().await {
                                        match msg {
                                            WsMessage::Init(http_request_config, vec) => {}
                                            WsMessage::Send(http_request_config, vec) => {
                                                let send_msg = if cfg.body_raw_type
                                                    == RequestBodyRawType::Text
                                                {
                                                    let data = &cfg.body_raw;
                                                    tungstenite::Message::Text(data.into())
                                                } else {
                                                    let dat = util::read_binary(&cfg.body_raw)
                                                        .await
                                                        .unwrap();
                                                    tungstenite::Message::Binary(dat.into())
                                                };
                                                match w.send(send_msg).await {
                                                    Ok(_) => {}
                                                    Err(err) => {
                                                        dbg!(&err);
                                                        ws_msgs_w.write().unwrap().push(
                                                            Message::text(format!(
                                                                "> Send Error: {}",
                                                                err
                                                            )),
                                                        );
                                                        break;
                                                    }
                                                }
                                            }
                                            WsMessage::Close => {
                                                break;
                                            }
                                            WsMessage::ReadMessage => {}
                                        }
                                    }
                                    println!("写入断开");
                                    let mut ni = need_init_w.lock().await;
                                    *ni = true;
                                });
                            }
                            Err(err) => {
                                ws_msgs
                                    .write()
                                    .unwrap()
                                    .push(Message::text(format!("> Connect Error: {}", err)));
                            }
                        }
                    }
                } else {
                    if let Some(tx) = _tx.as_mut() {
                        tx.send(msg).await;
                    };
                }
            }
        });
        my
    }

    /// 保存当前正在操作的项目
    fn save_current_project(&mut self) {
        self.action_status = match util::save_project(SAVE_DIR, &self.project) {
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

        self.select_test = None;
        self.new_project_name.clear(); // clear input name
        self.project_path.clear(); // new project not save
    }

    /// 加载一个项目
    fn load_project(&mut self) {
        match util::load_project(&self.project_path) {
            Ok(project) => {
                self.project = project;
                self.select_test = None;
                self.action_status = "Load project success".to_owned();
            }
            Err(err) => {
                self.action_status = err.to_string();
            }
        }
    }

    // top menus
    fn ui_top_menus(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Project", |ui| {
                    ui.horizontal(|ui| {
                        let input = ui.add(
                            egui::TextEdit::singleline(&mut self.new_project_name)
                                .hint_text("Enter Create Project")
                                .desired_width(100.0),
                        );

                        if input.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !self.new_project_name.is_empty()
                        {
                            self.create_project();
                        }
                    });

                    ui.separator();
                    if ui.add(egui::Button::new("Save Project")).clicked() {
                        self.save_current_project();
                        ui.close_menu();
                    }

                    ui.separator();
                    if ui.add(egui::Button::new("Load Project")).clicked() {
                        self.modal.open = true;
                        self.modal.title = "Load Project".to_owned();
                        self.modal.r#type = ModalType::LoadProject;
                        if let Ok(saved) = self.load_saved_project() {
                            self.saved = saved;
                        }
                    }
                });

                ui.menu_button("Setting", |ui| {
                    ui.vertical(|ui| {
                        ui.separator();
                        global_theme_preference_buttons(ui);
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui|
                    {ui.label(format!("Worker Threads: {}", self.worker_thread_count));});
            });
        });
    }
    fn ui_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(220.0)
            .width_range(30.0..=600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("📁").size(18.0));
                    ui.heading(&self.project.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("💾").on_hover_text("快速保存项目").clicked() {
                            self.save_current_project();
                        }
                    });
                });
                ui.separator();

                // 搜索框
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    let search_response = ui.add(
                        egui::TextEdit::singleline(&mut self.search_filter)
                            .hint_text("搜索 Group/Test...")
                            .desired_width(f32::INFINITY),
                    );
                    if !self.search_filter.is_empty() {
                        if ui.button("❌").on_hover_text("清除搜索").clicked() {
                            self.search_filter.clear();
                        }
                    }
                });
                ui.separator();

                egui::ScrollArea::both().show(ui, |ui| {
                    let var_count = self.project.variables.len();
                    CollapsingHeader::new(format!("Variables ({})", var_count))
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("➕ Add").clicked() {
                                    self.project.variables.push(PairUi::default());
                                }
                                if var_count > 0 {
                                    if ui.button("🗑️ Clear All").on_hover_text("清除所有变量").clicked() {
                                        self.project.variables.clear();
                                    }
                                }
                            });

                            ui.add_space(5.0);

                            egui_extras::TableBuilder::new(ui)
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .column(egui_extras::Column::auto())
                                .column(egui_extras::Column::auto())
                                .column(egui_extras::Column::auto().range(100.0..=400.0))
                                .column(egui_extras::Column::auto())
                                .min_scrolled_height(10.0)
                                // .scroll_to_row(1, Some(egui::Align::BOTTOM))
                                .header(20.0, |mut header| {
                                    header.col(|ui| {
                                        ui.label("启用").on_hover_text("勾选以启用该变量");
                                    });
                                    header.col(|ui| {
                                        ui.label("Key");
                                    });
                                    header.col(|ui| {
                                        ui.label("Value");
                                    });
                                    header.col(|ui| {
                                        ui.label("");
                                    });
                                })
                                .body(|mut body| {
                                    self.project.variables.retain_mut(|el| {
                                        let mut is_retain = true;
                                        body.row(30.0, |mut row| {
                                            row.col(|ui| {
                                                let mut enabled = !el.disable;
                                                if ui.checkbox(&mut enabled, "").changed() {
                                                    el.disable = !enabled;
                                                }
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
                                                if error_button(ui, "Del").clicked() {
                                                    is_retain = false;
                                                }
                                            });
                                        });
                                        is_retain
                                    });
                                });
                        });
                    ui.add_space(5.0);

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("➕");
                            let input_add = ui.add(
                                egui::TextEdit::singleline(&mut self.new_group_name)
                                    .hint_text("输入组名并按回车添加...")
                                    .desired_width(f32::INFINITY),
                            );

                            if input_add.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                && !self.new_group_name.is_empty()
                            {
                                let name = self.new_group_name.to_owned();
                                let name_exists = self.project.groups.iter().any(|el| el.name == name);
                                if !name_exists {
                                    self.project.groups.push(Group::from_name(name));
                                    self.new_group_name.clear();
                                } else {
                                    self.action_status = format!("组名 '{}' 已存在", name);
                                }
                            }
                        });
                    });

                    ui.add_space(5.0);

                    let search_lower = self.search_filter.to_lowercase();

                    self.project
                        .groups
                        .iter_mut()
                        .enumerate()
                        .for_each(|(group_index, group)| {
                            let test_count = group.childrent.len();

                            let group_matches = group.name.to_lowercase().contains(&search_lower);
                            let test_matches: Vec<usize> = group.childrent.iter().enumerate()
                                .filter(|(_, test)| test.name.to_lowercase().contains(&search_lower))
                                .map(|(i, _)| i)
                                .collect();

                            let should_show = self.search_filter.is_empty() || group_matches || !test_matches.is_empty();

                            if should_show {
                                CollapsingHeader::new(format!("{} ({})", group.name, test_count))
                                    .default_open(!self.search_filter.is_empty())
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            if ui.button("⚙️").on_hover_text("编辑组").clicked() {
                                                self.modal.open = true;
                                                self.modal.title = "Group Edit".to_owned();
                                                self.select_test = Some((group_index, 0));
                                                self.modal.r#type = ModalType::HandleGroup;
                                            }
                                        });

                                        ui.with_layout(
                                            egui::Layout::top_down_justified(egui::Align::Min),
                                            |ui| {
                                                group.childrent.iter_mut().enumerate().for_each(
                                                |(cfg_i, cfg)| {
                                                    let test_match = self.search_filter.is_empty() ||
                                                        cfg.name.to_lowercase().contains(&search_lower);

                                                    if test_match {
                                                        let checked = match self.select_test {
                                                            Some((i, j)) => {
                                                                i == group_index && j == cfg_i
                                                            }
                                                            _ => false,
                                                        };

                                                        ui.horizontal(|ui| {
                                                            if ui
                                                                .selectable_label(checked, &cfg.name)
                                                                .clicked()
                                                            {
                                                                self.select_test =
                                                                    Some((group_index, cfg_i));
                                                            }

                                                            if ui.button("📋").on_hover_text("复制测试").clicked() {
                                                                self.copy_test = Some((group_index, cfg_i));
                                                            }

                                                            if ui.button("✏️").on_hover_text("编辑测试").clicked() {
                                                                self.modal.open = true;
                                                                self.modal.title =
                                                                    "Test Edit".to_owned();
                                                                self.select_test =
                                                                    Some((group_index, cfg_i));
                                                                self.modal.r#type =
                                                                    ModalType::HandleTest;
                                                            }
                                                        });
                                                    }
                                                },
                                            );
                                        },
                                    );
                                });
                            }
                        });
                });
            });
    }
    fn ui_right_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::TopBottomPanel::bottom("bottom_panel")
                .resizable(false)
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Action:");
                        ui.label(&self.action_status);
                    });
                });

            egui::ScrollArea::both()
                .id_salt("right panel")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let Some((i, ii)) = self.select_test else {
                        return;
                    };
                    let Some(group) = self.project.groups.get_mut(i) else {
                        return;
                    };
                    let Some(http_test) = group.childrent.get_mut(ii) else {
                        return;
                    };

                    if let Some(ws_tx) = &self.ws_tx {
                        let tx: mpsc::Sender<WsMessage> = ws_tx.clone();
                        self.rt.spawn(async move {
                            tx.send(WsMessage::ReadMessage).await;
                        });
                    };


                    // 请求方式 - 第一行：输入控件
                    let is_running = http_test.stats.sending > 0;

                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("method")
                            .selected_text(http_test.request.method.as_ref())
                            .show_ui(ui, |ui| {
                                for m in &METHODS {
                                    ui.selectable_value(
                                        &mut http_test.request.method,
                                        m.to_owned(),
                                        m.as_ref(),
                                    );
                                }
                            });

                        ui.add_sized(
                            ui.available_size() - egui::vec2(
                                if http_test.request.method != Method::WS { 150.0 } else { 70.0 },
                                0.0
                            ),
                            egui::TextEdit::singleline(&mut http_test.request.url)
                                .hint_text("url"),
                        );

                        if http_test.request.method != Method::WS {
                            let count_input = ui.add(
                                egui::TextEdit::singleline(&mut http_test.send_count_ui)
                                    .desired_width(80.)
                                    .hint_text("Count"),
                            );

                            if let Ok(count) = http_test.send_count_ui.parse::<usize>() {
                                if count > 10_000_000 {
                                    count_input.on_hover_text("警告: 超过1000万可能导致性能问题");
                                } else if count > 100_000 {
                                    count_input.on_hover_text("提示: 超过10万可能需要较长时间");
                                }
                            }
                        }

                        if ui
                            .add_enabled(
                                !http_test.request.url.is_empty() && !is_running,
                                egui::Button::new("Send"),
                            )
                            .clicked()
                        {
                            if http_test.request.method == Method::WS {
                                if let Some(ws_tx) = &self.ws_tx {
                                    let cfg = http_test.request.to_owned();
                                    let variables = self.project.variables.to_owned();
                                    let tx = ws_tx.clone();
                                    self.rt.spawn(async move {
                                        tx.send(WsMessage::Send(cfg, variables)).await;
                                    });
                                }
                            } else {
                                http_test.send_before_init();
                                if http_test.send_count <= 0 {
                                    return;
                                }

                                http_test.stats.pending = 0;
                                http_test.stats.sending = http_test.send_count;

                                let cfg = Arc::new(http_test.request.to_owned());
                                let variables = Arc::new(self.project.variables.to_owned());
                                let tx = self.http_tx.clone();
                                let ctx_clone = ctx.clone();
                                let send_count = http_test.send_count;

                                self.rt.spawn(async move {
                                    Self::send_http_batch(cfg, variables, tx, ctx_clone, send_count).await;
                                });
                            }
                        }

                        if is_running {
                            if ui.button("Cancel").clicked() {
                                http_test.stats.sending = 0;
                                http_test.stats.total_end_time = Some(std::time::Instant::now());
                            }
                        }
                    });

                    // 第二行：统计信息和进度条
                    if http_test.request.method != Method::WS {
                        let stats = &http_test.stats;
                        let total = stats.total_requests() + stats.sending;

                        if total > 0 {
                            ui.horizontal(|ui| {
                                let completed = stats.total_requests();
                                let progress = completed as f32 / total as f32;

                                ui.add(
                                    egui::ProgressBar::new(progress)
                                        .desired_width(200.0)
                                        .show_percentage()
                                );

                                ui.label(format!(
                                    "完成: {} / {} ({:.1}%)",
                                    completed.to_formatted_string(&Locale::en),
                                    total.to_formatted_string(&Locale::en),
                                    progress * 100.0
                                ));

                                ui.separator();
                                ui.label(format!(
                                    "成功:{} 失败:{}",
                                    stats.success, stats.failed
                                ));

                                if stats.sending > 0 {
                                    ui.separator();
                                    if let Some(qps) = stats.realtime_qps() {
                                        ui.label(format!("实时QPS: {:.0}", qps));
                                    }
                                    if let Some(up) = stats.realtime_upload_throughput_mbps() {
                                        ui.label(format!("上传: {:.2} MB/s", up));
                                    }
                                    if let Some(down) = stats.realtime_download_throughput_mbps() {
                                        ui.label(format!("下载: {:.2} MB/s", down));
                                    }
                                } else if stats.total_requests() > 0 {
                                    ui.separator();
                                    if let Some(qps) = stats.qps() {
                                        ui.label(format!("平均QPS: {:.0}", qps));
                                    }
                                    if let Some(up) = stats.upload_throughput_mbps() {
                                        ui.label(format!("上传: {:.2} MB/s", up));
                                    }
                                    if let Some(down) = stats.download_throughput_mbps() {
                                        ui.label(format!("下载: {:.2} MB/s", down));
                                    }
                                }
                            });
                        }
                    }
                    ui.separator();

                    // 请求数据
                    widget::horizontal_tabs(ui, REQ_TABS.iter(), &mut http_test.tab_ui);
                    ui.separator();

                    match http_test.tab_ui {
                        RequestTab::Params => {
                            widget::pair_table(ui, "param scroll", &mut http_test.request.query);
                        }
                        RequestTab::Headers => {
                            widget::pair_table(ui, "header scroll", &mut http_test.request.header);
                        }
                        RequestTab::Body => {
                            widget::horizontal_tabs(
                                ui,
                                REQ_BODY_TABS.iter(),
                                &mut http_test.request.body_tab_ui,
                            );
                            ui.separator();

                            match http_test.request.body_tab_ui {
                                RequestBodyTab::Raw => {
                                    ui.vertical(|ui| {
                                        ui.group(|ui| {
                                            ui.horizontal(|ui| {
                                                if http_test.request.method == Method::WS {
                                                    WS_BODY_RAW_TYPES.iter()
                                                } else {
                                                    REQ_BODY_RAW_TYPES.iter()
                                                }
                                                .for_each(|raw_type| {
                                                    ui.radio_value(
                                                        &mut http_test.request.body_raw_type,
                                                        raw_type.to_owned(),
                                                        raw_type.as_ref(),
                                                    );
                                                });
                                            });
                                        });

                                        egui::ScrollArea::both()
                                            .id_salt("row data scroll")
                                            .max_height(120.0)
                                            .show(ui, |ui| {
                                                ui.add(
                                                    egui::TextEdit::multiline(
                                                        &mut http_test.request.body_raw,
                                                    )
                                                    .desired_rows(6),
                                                );
                                            });
                                    });
                                }

                                RequestBodyTab::Form => {
                                    if http_test.request.method == Method::WS {
                                        return;
                                    }
                                    widget::pair_table(
                                        ui,
                                        "body_form scroll",
                                        &mut http_test.request.body_form,
                                    );
                                }

                                RequestBodyTab::FormData => {
                                    if http_test.request.method == Method::WS {
                                        return;
                                    }
                                    widget::pair_table(
                                        ui,
                                        "body_form scroll",
                                        &mut http_test.request.body_form_data,
                                    );
                                }
                            }
                        }
                        RequestTab::Scripts => {
                            ui.vertical(|ui| {
                                ui.checkbox(&mut http_test.request.script_enabled, "启用脚本 (Enable Scripts)");

                                ui.add_space(5.0);
                                ui.separator();

                                ui.label("Pre-Request Script (请求前脚本):");
                                ui.label("在发送请求前执行,可修改 URL、Headers、Body 等");
                                ui.add_space(3.0);
                                egui::ScrollArea::vertical()
                                    .id_salt("pre_request_script_scroll")
                                    .max_height(200.0)
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::TextEdit::multiline(&mut http_test.request.pre_request_script)
                                                .font(egui::TextStyle::Monospace)
                                                .code_editor()
                                                .desired_rows(10)
                                                .desired_width(f32::INFINITY),
                                        );
                                    });

                                ui.add_space(10.0);
                                ui.separator();

                                ui.label("Post-Response Script (响应后脚本):");
                                ui.label("在收到响应后执行,可验证业务状态码、提取数据到变量等");
                                ui.add_space(3.0);
                                egui::ScrollArea::vertical()
                                    .id_salt("post_response_script_scroll")
                                    .max_height(200.0)
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::TextEdit::multiline(&mut http_test.request.post_response_script)
                                                .font(egui::TextStyle::Monospace)
                                                .code_editor()
                                                .desired_rows(10)
                                                .desired_width(f32::INFINITY),
                                        );
                                    });

                                ui.add_space(10.0);

                                // 帮助提示
                                ui.collapsing("📖 脚本帮助", |ui| {
                                    ui.label("可用对象:");
                                    ui.monospace("  request.url, request.method, request.headers, request.params, request.body");
                                    ui.monospace("  response.status, response.headers, response.body, response.duration");
                                    ui.monospace("  vars - 环境变量");

                                    ui.add_space(5.0);
                                    ui.label("常用函数:");
                                    ui.monospace("  parse_json() - JSON解析");
                                    ui.monospace("  md5(), sha256(), hmac_sha256()");
                                    ui.monospace("  base64_encode(), base64_decode()");
                                    ui.monospace("  timestamp(), uuid(), random_string(len)");

                                    ui.add_space(5.0);
                                    ui.label("示例 - 判断业务状态码:");
                                    ui.code("let result = parse_json(response.body);");
                                    ui.code("vars[\"test_result\"] = if result.code == 0 { \"PASS\" } else { \"FAIL\" };");
                                });
                            });
                        }
                    };

                    ui.separator();

                    if http_test.request.method == Method::WS {
                        ui.horizontal(|ui| {
                            if ui.button("Clear").clicked() {
                                self.ws_messages.write().unwrap().clear();
                            }
                            if ui.button("WS Clone").clicked() {
                                if let Some(ws_tx) = &self.ws_tx {
                                    let tx: mpsc::Sender<WsMessage> = ws_tx.clone();
                                    self.rt.spawn(async move {
                                        tx.send(WsMessage::Close).await;
                                    });
                                }
                            }
                        });

                        if let Ok(ws_msgs) = self.ws_messages.read() {
                            ui.separator();

                            egui::ScrollArea::both()
                                .hscroll(true)
                                .vscroll(true)
                                .id_salt("ws messages")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ws_msgs.iter().for_each(|msg| {
                                        match msg {
                                            Message::Text(utf8_bytes) => {
                                                ui.label(utf8_bytes.as_str());
                                            }
                                            Message::Binary(bytes) => {
                                                ui.label("[Binary]");
                                            }
                                            Message::Ping(bytes) => {}
                                            Message::Pong(bytes) => {}
                                            Message::Close(close_frame) => {
                                                ui.label("[close]");
                                            }
                                            Message::Frame(frame) => {}
                                        }
                                        ui.separator();
                                    });
                                });
                        }
                    }

                    // 请求结果
                    let Some(ref response) = http_test.response else {
                        return;
                    };
                    // 从字节码中初始化数据
                    let (processed_text, has_img) = ApiTestApp::process_response_data(self.is_pretty, ui.ctx(), response);

                    // 请求返回状态
                    ui.horizontal(|ui| {
                        ui.heading(format!(
                            "Response Status: {:?} {}  {}ms",
                            response.version, response.status, response.duration
                        ));

                        ui.separator();

                        if let Some(data_vec) = &response.data_vec {
                            ui.add(
                                egui::TextEdit::singleline(&mut http_test.download_path)
                                    .hint_text(r#"c:/out.(jpg|txt)"#),
                            );
                            if http_test.response_tab_ui != ResponseTab::Stats {
                                if ui
                                    .add_enabled(
                                        !http_test.download_path.is_empty(),
                                        egui::Button::new(match http_test.response_tab_ui {
                                            ResponseTab::Data => "Download Data",
                                            ResponseTab::Header => "Download Header",
                                            ResponseTab::Stats => "",
                                        }),
                                    )
                                    .clicked()
                                {
                                    match util::download(
                                        &http_test.request.url,
                                        &http_test.download_path,
                                        match http_test.response_tab_ui {
                                            ResponseTab::Data => data_vec,
                                            ResponseTab::Header => response.headers_str.as_bytes(),
                                            ResponseTab::Stats => &[],
                                        },
                                    ) {
                                        Ok(_) => {
                                            self.action_status = "Downlaod Ok".to_owned();
                                        }
                                        Err(err) => {
                                            self.action_status = err.to_string();
                                        }
                                    }
                                }
                            }
                        }
                    });
                    ui.separator();

                    // 查看请求返回的数据和header
                    widget::horizontal_tabs(
                        ui,
                        RESPONSE_TABS.iter(),
                        &mut http_test.response_tab_ui,
                    );
                    ui.separator();

                    match http_test.response_tab_ui {
                        ResponseTab::Data => match &response.data_vec {
                            Some(data_vec) => {
                                if !has_img {
                                    if ui.radio(self.is_pretty, "Pretty").clicked() {
                                        self.is_pretty = !self.is_pretty;
                                    }
                                }
                                ui.separator();
                                egui::ScrollArea::both()
                                    .hscroll(true)
                                    .vscroll(true)
                                    .id_salt("response data scroll")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        if has_img {
                                            ui.add(
                                                egui::Image::from_bytes(
                                                    "bytes://",
                                                    data_vec.as_bytes().to_owned(),
                                                )
                                                .rounding(5.0),
                                            );
                                        } else if let Some(text_data) = &processed_text {
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
                            egui::ScrollArea::both()
                                .hscroll(true)
                                .vscroll(true)
                                .id_salt("response heaer scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        widget::code_view_ui(ui, &response.headers_str);
                                    });
                                });
                        }
                        ResponseTab::Stats => {
                            let stats = &http_test.stats;
                            if stats.total_requests() > 0 {
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.columns(2, |columns| {
                                            // 左列：请求统计
                                            columns[0].group(|ui| {
                                                ui.heading("📊 请求统计");
                                                ui.separator();

                                                ui.horizontal(|ui| {
                                                    ui.label("总请求数:");
                                                    ui.strong(format!("{}", stats.total_requests().to_formatted_string(&Locale::en)));
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.label("成功:");
                                                    ui.colored_label(egui::Color32::GREEN, format!("{}", stats.success.to_formatted_string(&Locale::en)));
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.label("失败:");
                                                    ui.colored_label(egui::Color32::RED, format!("{}", stats.failed.to_formatted_string(&Locale::en)));
                                                });

                                                ui.add_space(5.0);

                                                // 成功率进度条
                                                let success_rate = stats.success_rate() / 100.0;
                                                ui.horizontal(|ui| {
                                                    ui.label("成功率:");
                                                    ui.add(
                                                        egui::ProgressBar::new(success_rate as f32)
                                                            .desired_width(150.0)
                                                            .text(format!("{:.2}%", stats.success_rate()))
                                                    );
                                                });
                                            });

                                            // 右列：响应时间统计
                                            columns[1].group(|ui| {
                                                ui.heading("⏱️ 响应时间统计");
                                                ui.separator();

                                                if let Some(min) = stats.min_response_time() {
                                                    ui.horizontal(|ui| {
                                                        ui.label("最小 (Min):");
                                                        ui.strong(format!("{} ms", min));
                                                    });
                                                }

                                                if let Some(avg) = stats.avg_response_time() {
                                                    ui.horizontal(|ui| {
                                                        ui.label("平均 (Avg):");
                                                        ui.strong(format!("{:.2} ms", avg));
                                                    });
                                                }

                                                if let Some(max) = stats.max_response_time() {
                                                    ui.horizontal(|ui| {
                                                        ui.label("最大 (Max):");
                                                        ui.strong(format!("{} ms", max));
                                                    });
                                                }

                                                ui.add_space(5.0);
                                                ui.label("百分位数:");

                                                if let Some(p50) = stats.percentile(50.0) {
                                                    ui.horizontal(|ui| {
                                                        ui.label("  P50:");
                                                        ui.label(format!("{} ms", p50));
                                                    });
                                                }

                                                if let Some(p95) = stats.percentile(95.0) {
                                                    ui.horizontal(|ui| {
                                                        ui.label("  P95:");
                                                        ui.label(format!("{} ms", p95));
                                                    });
                                                }

                                                if let Some(p99) = stats.percentile(99.0) {
                                                    ui.horizontal(|ui| {
                                                        ui.label("  P99:");
                                                        ui.label(format!("{} ms", p99));
                                                    });
                                                }
                                            });
                                        });

                                        ui.separator();

                                        // 性能统计和吞吐量
                                        ui.columns(2, |columns| {
                                            columns[0].group(|ui| {
                                                ui.heading("🚀 性能统计");
                                                ui.separator();

                                                if let Some(total_dur) = stats.total_duration() {
                                                    ui.horizontal(|ui| {
                                                        ui.label("总耗时:");
                                                        ui.strong(format!("{:.3} s", total_dur));
                                                    });
                                                }

                                                if let Some(qps) = stats.qps() {
                                                    ui.horizontal(|ui| {
                                                        ui.label("QPS:");
                                                        ui.colored_label(egui::Color32::from_rgb(0, 150, 255), format!("{:.0}", qps));
                                                    });
                                                }
                                            });

                                            columns[1].group(|ui| {
                                                ui.heading("📦 数据吞吐量");
                                                ui.separator();

                                                ui.horizontal(|ui| {
                                                    ui.label("上传:");
                                                    ui.strong(format!("{:.2} MB", stats.total_upload_bytes as f64 / 1024.0 / 1024.0));
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.label("下载:");
                                                    ui.strong(format!("{:.2} MB", stats.total_download_bytes as f64 / 1024.0 / 1024.0));
                                                });

                                                ui.add_space(5.0);

                                                if let Some(up) = stats.upload_throughput_mbps() {
                                                    ui.horizontal(|ui| {
                                                        ui.label("上传速度:");
                                                        ui.label(format!("{:.2} MB/s", up));
                                                    });
                                                }

                                                if let Some(down) = stats.download_throughput_mbps() {
                                                    ui.horizontal(|ui| {
                                                        ui.label("下载速度:");
                                                        ui.label(format!("{:.2} MB/s", down));
                                                    });
                                                }
                                            });
                                        });
                                    });
                            } else {
                                ui.label("暂无统计数据");
                            }
                        }
                    }
                });
        });
    }

    fn ui_modal(&mut self, ctx: &egui::Context) {
        if self.modal.open {
            let enabled = ctx.input(|i| i.time) - &self.modal.disabled_time > 2.0;
            if !enabled {
                ctx.request_repaint();
            }

            egui::Window::new(&self.modal.title)
                .id(egui::Id::new("Window Model")) // required since we change the title
                // .open(&mut self.modal.open)
                .open(&mut self.modal.open)
                .resizable(true)
                .collapsible(true)
                .title_bar(true)
                .scroll([true; 2])
                .enabled(enabled)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| match self.modal.r#type {
                    ModalType::None => {}
                    ModalType::HandleGroup => {
                        let Some((i, _)) = &self.select_test else {
                            return;
                        };
                        let Some(group) = self.project.groups.get_mut(*i) else {
                            return;
                        };
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Edit Name");
                                egui::TextEdit::singleline(&mut group.name).show(ui);
                            });
                            ui.separator();
                            if error_button(ui, format!("Del Group({})", &group.name)).clicked() {
                                self.remove_group = Some(*i);
                            }
                            ui.separator();
                            let input_add = ui.add(
                                egui::TextEdit::singleline(&mut group.new_child_name)
                                    .hint_text("Enter Add Test"),
                            );
                            if input_add.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                && !group.new_child_name.is_empty()
                            {
                                group.create_child();
                                input_add.request_focus();
                            }
                        });
                    }
                    ModalType::HandleTest => {
                        let Some((i, ii)) = &self.select_test else {
                            return;
                        };
                        let Some(group) = self.project.groups.get_mut(*i) else {
                            return;
                        };
                        let Some(http_test) = group.childrent.get_mut(*ii) else {
                            return;
                        };
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Edit Name");
                                egui::TextEdit::singleline(&mut http_test.name).show(ui);
                            });
                            ui.separator();
                            if error_button(ui, format!("Del Test({})", &http_test.name)).clicked()
                            {
                                self.remove_test = Some((*i, *ii));
                            }
                        });
                    }
                    ModalType::LoadProject => {
                        ui.vertical(|ui| {
                            for i in 0..self.saved.len() {
                                let (name, path) = self.saved.index(i);
                                if ui.button(name).clicked() {
                                    self.project_path = path.to_owned();
                                    match util::load_project(&self.project_path) {
                                        Ok(project) => {
                                            self.project = project;
                                            self.select_test = None;
                                            self.action_status = "Load project success".to_owned();
                                        }
                                        Err(err) => {
                                            self.action_status = err.to_string();
                                        }
                                    }
                                }
                                ui.separator();
                            }
                        });
                    }
                });
        }
    }
}

impl ApiTestApp {
    async fn send_http_batch(
        cfg: Arc<HttpRequestConfig>,
        variables: Arc<Vec<PairUi>>,
        tx: tokio::sync::mpsc::Sender<Result<HttpResponse>>,
        ctx_clone: egui::Context,
        send_count: usize
    ) {
        let max_concurrent = 10000;
        let mut futures = FuturesUnordered::new();
        let mut sent = 0;

        while sent < send_count || !futures.is_empty() {
            while sent < send_count && futures.len() < max_concurrent {
                let req_cfg = cfg.clone();
                let vars = variables.clone();
                let tx = tx.clone();

                futures.push(async move {
                    let result = util::http_send(&*req_cfg, &*vars).await;
                    let _ = tx.send(result).await;
                });
                sent += 1;
            }

            if futures.next().await.is_some() {
            }
        }
        ctx_clone.request_repaint();
    }

    fn process_response_data(is_pretty: bool, ctx: &egui::Context, response: &HttpResponse) -> (Option<String>, bool) {
        let Some(data_vec) = &response.data_vec else {
            return (None, false);
        };

        if response.content_type_image() {
            ctx.forget_image("bytes://");
            return (None, true);
        }

        let mut data = std::str::from_utf8(data_vec.as_ref())
            .unwrap_or("")
            .to_owned();

        if is_pretty && response.content_type_json() {
            if let Ok(j) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Ok(pretty_data) = serde_json::to_string_pretty(&j) {
                    data = pretty_data;
                }
            }
        }

        (Some(data), false)
    }

    fn process_http_responses(&mut self, ctx: &egui::Context) {
        const MAX_PROCESS_PER_FRAME: usize = 1000;
        let mut processed = 0;

        while processed < MAX_PROCESS_PER_FRAME {
            let result = match self.http_rx.try_recv() {
                Ok(result) => result,
                Err(_) => break,
            };

            self.handle_http_response(result);
            processed += 1;
        }

        if processed > 0 {
            ctx.request_repaint();
        }
    }

    fn handle_http_response(&mut self, result: Result<HttpResponse>) {
        let Some((group_idx, test_idx)) = self.select_test else {
            return;
        };

        let Some(group) = self.project.groups.get_mut(group_idx) else {
            return;
        };

        let Some(http_test) = group.childrent.get_mut(test_idx) else {
            return;
        };

        match result {
            Ok(response) => {
                http_test.stats.add_response_time(response.duration);
                http_test.stats.total_upload_bytes += response.request_size;
                http_test.stats.total_download_bytes += response.response_size;

                let is_success = response.status.is_success();

                // 应用脚本修改的变量到项目
                if let Some(modified_vars) = &response.modified_vars {
                    for var in modified_vars {
                        if let Some(existing) = self.project.variables.iter_mut().find(|v| v.key == var.key) {
                            existing.value = var.value.clone();
                        } else {
                            self.project.variables.push(var.clone());
                        }
                    }
                }

                http_test.response = Some(response);
                http_test.stats.sending -= 1;

                if is_success {
                    http_test.stats.success += 1;
                } else {
                    http_test.stats.failed += 1;
                }
            }
            Err(_) => {
                http_test.stats.sending -= 1;
                http_test.stats.failed += 1;
            }
        }

        if http_test.stats.sending == 0 {
            http_test.stats.total_end_time = Some(std::time::Instant::now());
        }
    }

    fn cleanup_ui_state(&mut self) {
        // 删除group
        if let Some(i) = self.remove_group {
            self.project.groups.remove(i);
            self.remove_group = None;
        }

        // 删除group.children
        if let Some((i, ii)) = self.remove_test {
            self.project.groups[i].childrent.remove(ii);
            self.remove_test = None;
        }

        // 复制test
        if let Some((i, ii)) = self.copy_test {
            if let Some(group) = self.project.groups.get_mut(i) {
                if let Some(test) = group.childrent.get(ii) {
                    let mut cloned_test = test.clone();
                    cloned_test.name = format!("{} - Copy", test.name);
                    group.childrent.insert(ii + 1, cloned_test);
                }
            }
            self.copy_test = None;
        }
    }
}

impl eframe::App for ApiTestApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_current_project();
        self.action_status = "auto save".to_owned();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_http_responses(ctx);
        self.cleanup_ui_state();
        self.ui_modal(ctx);
        self.ui_top_menus(ctx);
        self.ui_left_panel(ctx);
        self.ui_right_panel(ctx);
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum ModalType {
    None,
    HandleGroup,
    HandleTest,
    LoadProject,
}

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModalOptions {
    pub open: bool,
    pub title: String,
    pub disabled_time: f64,
    pub r#type: ModalType,
}

impl Default for ModalOptions {
    fn default() -> Self {
        Self {
            open: false,
            title: "Model".to_owned(),
            disabled_time: f64::NEG_INFINITY,
            r#type: ModalType::None,
        }
    }
}
