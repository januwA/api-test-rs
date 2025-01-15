#![allow(warnings, unused)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use anyhow::Result;
use core::f32;
use futures_util::{SinkExt, StreamExt};
use std::collections::BTreeMap;
use std::io::Read;
use std::ops::Index;
use std::sync::Arc;
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
use tokio::sync::{mpsc, watch};
use widget::error_button;

mod util;
mod widget;

/* #region const variables */
const SEND_THREAD_COUN: usize = 2;
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
const WS_BODY_RAW_TYPES: [RequestBodyRawType; 2] =
    [RequestBodyRawType::Text, RequestBodyRawType::BinaryFile];
const COLUMN_WIDTH_INITIAL: f32 = 200.0;
const RESPONSE_TABS: [ResponseTab; 2] = [ResponseTab::Data, ResponseTab::Header];
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
    tx: mpsc::Sender<Result<HttpResponse>>,
    rx: mpsc::Receiver<Result<HttpResponse>>,
    ws_tx: Option<tokio::sync::mpsc::Sender<WsMessage>>,
    ws_msgs: Arc<std::sync::RwLock<Vec<Message>>>,

    // 加载保存的项目文件路径
    project_path: String,
    remove_group: Option<usize>,

    select_test: Option<(usize, usize)>,
    remove_test: Option<(usize, usize)>,

    new_project_name: String,
    new_group_name: String,

    // 当前项目
    project: Project,

    action_status: String,
    thread_count: String,

    // 已保存的项目 (name, path)
    saved: Vec<(String, String)>,

    // 美化请求的返回结果，如格式化json
    is_pretty: bool,

    pub modal: ModalOptions,
}

impl Default for ApiTestApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel::<Result<HttpResponse>>(32);
        Self {
            tx,
            rx,
            ws_tx: Default::default(),
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(SEND_THREAD_COUN)
                .build()
                .unwrap(),
            new_group_name: Default::default(),
            new_project_name: Default::default(),
            action_status: Default::default(),
            saved: Default::default(),
            thread_count: SEND_THREAD_COUN.to_string(),
            project_path: Default::default(),
            select_test: Some((0, 0)),
            remove_test: None,
            project: Project {
                name: "Any".to_owned(),
                groups: vec![{
                    let mut g = Group::from_name("Group #1".to_owned());
                    let mut t = HttpTest::from_name("test".to_owned());
                    t.request.url = "{{base}}/ping".to_owned();
                    g.childrent.push(t);
                    g
                }],
                variables: vec![PairUi::from_kv("base", "http://127.0.0.1:3000")],
            },
            is_pretty: true,
            remove_group: None,

            modal: Default::default(),
            ws_msgs: Default::default(),
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
        let ws_msgs = my.ws_msgs.clone();

        my.rt.spawn(async move {
            let ws_msgs_c = ws_msgs.clone();
            let mut _tx: Option<tokio::sync::mpsc::Sender<WsMessage>> = None;
            let mut need_init = true;

            while let Some(msg) = ws_rx.recv().await {
                if !need_init {
                    if let Some(tx) = _tx.as_mut() {
                        tx.send(msg).await;
                    };
                    continue;
                }
                if let WsMessage::Send(cfg, variables) = msg {
                    if need_init {
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
                                need_init = false;

                                let (mut w, mut r) = socket.split();

                                let ws_msgs_r = ws_msgs_c.clone();
                                tokio::spawn(async move {
                                    while let Some(message) = r.next().await {
                                        match message {
                                            Ok(msg) => {
                                                ws_msgs_r.write().unwrap().push(msg);
                                            }
                                            Err(err) => {
                                                dbg!(err);
                                            }
                                        }
                                    }
                                });

                                let ws_msgs_w = ws_msgs_c.clone();
                                let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(32);
                                _tx = Some(tx);

                                tokio::spawn(async move {
                                    while let Some(msg) = rx.recv().await {
                                        if let WsMessage::Send(cfg, variables) = msg {
                                            let send_msg = if cfg.body_raw_type
                                                == RequestBodyRawType::Text
                                            {
                                                let data = &cfg.body_raw;
                                                tungstenite::Message::Text(data.into())
                                            } else {
                                                let dat =
                                                    util::read_binary(&cfg.body_raw).await.unwrap();
                                                tungstenite::Message::Binary(dat.into())
                                            };
                                            match w.send(send_msg).await {
                                                Ok(_) => {}
                                                Err(err) => {
                                                    ws_msgs_w.write().unwrap().push(Message::text(
                                                        format!("> Send Error: {}", err),
                                                    ));
                                                }
                                            }
                                        }
                                    }
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
                        ui.horizontal(|ui| {
                            ui.label("Thread Count");
                            // ui.text_edit_singleline(&mut self.thread_count);
                            TextEdit::singleline(&mut self.thread_count)
                                .desired_width(50.0)
                                .show(ui);
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
                        ui.separator();
                        global_theme_preference_buttons(ui);
                    });
                });
            });
        });
    }
    fn ui_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(220.0)
            .width_range(30.0..=600.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(&self.project.name);
                });

                egui::ScrollArea::both().show(ui, |ui| {
                    CollapsingHeader::new("Variables")
                        .default_open(false)
                        .show(ui, |ui| {
                            if ui.button("Add").clicked() {
                                self.project.variables.push(PairUi::default());
                            }

                            ui.separator();

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
                                        ui.label("");
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
                                                if error_button(ui, "Del").clicked() {
                                                    is_retain = false;
                                                }
                                            });
                                        });
                                        is_retain
                                    });
                                });
                        });
                    ui.separator();

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
                            self.project.groups.push(Group::from_name(name));
                            self.new_group_name.clear();
                        }
                    }

                    self.project
                        .groups
                        .iter_mut()
                        .enumerate()
                        .for_each(|(group_index, group)| {
                            ui.separator();
                            CollapsingHeader::new(&group.name)
                                .default_open(false)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("...").clicked() {
                                            self.modal.open = true;
                                            self.modal.title = "Group Edit".to_owned();
                                            self.select_test = Some((group_index, 0));
                                            self.modal.r#type = ModalType::HandleGroup;
                                        }
                                    });
                                    ui.separator();

                                    ui.with_layout(
                                        egui::Layout::top_down_justified(egui::Align::Min),
                                        |ui| {
                                            group.childrent.iter_mut().enumerate().rev().for_each(
                                                |(cfg_i, cfg)| {
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

                                                        if ui.button("...").clicked() {
                                                            self.modal.open = true;
                                                            self.modal.title =
                                                                "Test Edit".to_owned();
                                                            self.select_test =
                                                                Some((group_index, cfg_i));
                                                            self.modal.r#type =
                                                                ModalType::HandleTest;
                                                        }
                                                    });

                                                    ui.separator();
                                                },
                                            );
                                        },
                                    );
                                });
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

                    // 渲染时尝试获取请求返回值，如果不渲染就不会去获取，其它方法使用Arc+Mutex
                    match self.rx.try_recv() {
                        Ok(data) => match data {
                            Ok(res) => {
                                http_test.s_e.0 += 1;
                                // TODO: 使用lua脚本让使用者自行判断该请求是成功还是失败
                                if http_test.response.is_none() {
                                    http_test.response = Some(res);
                                } else {
                                    // httpConfig.response_vec.push(res);
                                }
                            }
                            Err(_) => {
                                http_test.s_e.1 += 1;
                            }
                        },
                        Err(_) => {
                            // 没有消息，或其他错误
                        }
                    }

                    // 请求方式
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

                        ui.add(
                            egui::TextEdit::singleline(&mut http_test.request.url)
                                .desired_width(300.)
                                .hint_text("url"),
                        );

                        if http_test.request.method != Method::WS {
                            ui.add(
                                egui::TextEdit::singleline(&mut http_test.send_count_ui)
                                    .desired_width(60.)
                                    .hint_text("Count"),
                            );
                        }

                        if ui
                            .add_enabled(
                                !http_test.request.url.is_empty(),
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
                                let cfg = Arc::new(http_test.request.to_owned());
                                let variables = Arc::new(self.project.variables.to_owned());
                                for _ in 0..http_test.send_count {
                                    let req_cfg = cfg.clone();
                                    // TODO:现在每次发送变量都是固定的，可以使用lua脚本在发送前改变一些变量
                                    let vars = variables.clone();
                                    let tx = self.tx.clone();
                                    self.rt.spawn(async move {
                                        match tx
                                            .send(util::http_send(&*req_cfg, &*vars).await)
                                            .await
                                        {
                                            Ok(_) => {
                                                // println!("send ok");
                                            }
                                            Err(_) => {
                                                println!("send err");
                                            }
                                        };
                                    });
                                }
                            }
                        }

                        if http_test.request.method != Method::WS {
                            ui.separator();
                            // request result count
                            let (s, e) = &http_test.s_e;
                            ui.label(format!("s:{s}, e:{e}"));
                        }
                    });
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
                    };

                    ui.separator();

                    if let Ok(ws_msgs) = self.ws_msgs.read() {
                        let msgs = &*ws_msgs;
                        for msg in msgs {
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
                        }
                    }

                    // 请求结果
                    let Some(response) = &mut http_test.response else {
                        return;
                    };
                    // 从字节码中初始化数据
                    if let Some(data_vec) = &response.data_vec {
                        let isjson = response.content_type_json();
                        let isimg = response.content_type_image();

                        // 初始化图片或字符串数据
                        if isimg {
                            response.img.get_or_insert_with(|| {
                                ui.ctx().forget_image("bytes://");
                                ()
                            });
                        } else {
                            response.text.get_or_insert_with(|| {
                                let mut data = std::str::from_utf8(data_vec.as_ref())
                                    .unwrap_or("")
                                    .to_owned();

                                if self.is_pretty && isjson {
                                    let j: serde_json::Value = serde_json::from_str(&data).unwrap();
                                    data = serde_json::to_string_pretty(&j).unwrap();
                                }

                                data
                            });
                        }
                    }

                    // 请求返回状态
                    ui.horizontal(|ui| {
                        ui.heading(format!(
                            "Response Status: {:?} {}",
                            response.version, response.status
                        ));

                        ui.separator();

                        if let Some(data_vec) = &response.data_vec {
                            ui.add(
                                egui::TextEdit::singleline(&mut http_test.download_path)
                                    .hint_text(r#"c:/out.(jpg|txt)"#),
                            );
                            if ui
                                .add_enabled(
                                    !http_test.download_path.is_empty(),
                                    egui::Button::new(match http_test.response_tab_ui {
                                        ResponseTab::Data => "Download Data",
                                        ResponseTab::Header => "Download Header",
                                    }),
                                )
                                .clicked()
                            {
                                match util::download(
                                    &http_test.download_path,
                                    match http_test.response_tab_ui {
                                        ResponseTab::Data => data_vec,
                                        ResponseTab::Header => response.headers_str.as_bytes(),
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
                                let isimg = response.content_type_image();
                                if !isimg {
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
                                        if let Some(_) = &response.img {
                                            ui.add(
                                                egui::Image::from_bytes(
                                                    "bytes://",
                                                    data_vec.as_bytes().to_owned(),
                                                )
                                                .rounding(5.0),
                                            );
                                        } else if let Some(text_data) = response.text.as_ref() {
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

impl eframe::App for ApiTestApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_current_project();
        self.action_status = "auto save".to_owned();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 删除group
        if let Some(i) = self.remove_group {
            self.project.groups.remove(i);
            self.remove_group = None
        }

        // 删除group.children
        if let Some((i, ii)) = self.remove_test {
            self.project.groups[i].childrent.remove(ii);
            self.remove_test = None
        }

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
