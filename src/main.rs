#![allow(warnings, unused)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::borrow::BorrowMut;
use std::fmt::Display;
use std::net::SocketAddr;

use anyhow::anyhow;
use eframe::egui::output::OpenUrl;
use eframe::epaint::vec2;
use eframe::{
    egui::{self, Hyperlink, RichText, Ui},
    epaint::Color32,
};
use egui_extras::RetainedImage;
use poll_promise::Promise;
use reqwest::header::HeaderMap;
use serde_json::json;
use tokio::runtime::Runtime;

mod cache;
mod util;
mod widget;

const K_IMAGE_MAX_WIDTH: f32 = 200.0;
const K_REQ_METHODS: [reqwest::Method; 8] = [
    reqwest::Method::GET,
    reqwest::Method::POST,
    reqwest::Method::PUT,
    reqwest::Method::DELETE,
    reqwest::Method::HEAD,
    reqwest::Method::OPTIONS,
    reqwest::Method::CONNECT,
    reqwest::Method::TRACE,
];
const K_REQ_TABS: [RequestTab; 3] = [RequestTab::Params, RequestTab::Headers, RequestTab::Body];
const K_REQ_BODY_TABS: [&str; 3] = ["Raw", "Form", "form-data"];
/// binary 输入文件路径
const K_REQ_BODY_RAW_TYPES: [&str; 5] = ["text", "json", "form", "xml", "binary file"];

const K_COLUMN_WIDTH_INITIAL: f32 = 200.0;

const K_RESPONSE_TABS: [&str; 2] = ["Data", "Header"];

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    let mut options = eframe::NativeOptions::default();
    options.icon_data = Some(util::load_app_icon());

    // options.initial_window_pos = Some([0f32, 0f32].into());
    options.min_window_size = Some([1400.0, 800.0].into());

    // options.fullscreen = true;
    options.maximized = false;

    eframe::run_native(
        "api test",
        options,
        Box::new(|cc| Box::new(ApiTestApp::new(cc))),
    )
}

struct ApiTestApp {
    method_idx: usize,
    url: String,

    request_query: Vec<PairUi>,
    request_header: Vec<PairUi>,
    request_body_form: Vec<PairUi>,
    request_body_form_data: Vec<PairUi>,
    request_body_raw: String,
    request_body_raw_type_idx: usize,

    response_promise: Option<Promise<anyhow::Result<HttpResponse>>>,
    response_data_download_path: String,

    req_tab: RequestTab,
    req_body_tab_idx: usize,
    rt: Runtime,

    response_tab_idx: usize,
}

impl Default for ApiTestApp {
    fn default() -> Self {
        Self {
            req_body_tab_idx: Default::default(),
            request_body_raw: Default::default(),
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
            url: "http://127.0.0.1:3000/ping".to_string(),
            response_promise: Default::default(),
            response_data_download_path: Default::default(),
            request_body_raw_type_idx: Default::default(),
            request_query: vec![PairUi::default()],
            request_header: vec![PairUi::default()],
            request_body_form: vec![PairUi::default()],
            request_body_form_data: vec![PairUi::default()],
            req_tab: RequestTab::Params,
            method_idx: 0,
            response_tab_idx: Default::default(),
        }
    }
}

impl ApiTestApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        util::setup_custom_fonts(&cc.egui_ctx);
        Self::default()
    }
}

impl ApiTestApp {
    fn http_send(&mut self) {
        let (sender, response_promise) = Promise::new();
        self.response_promise = Some(response_promise);

        let method = K_REQ_METHODS[self.method_idx].clone();
        let url: String = self.url.clone();

        let request_query: Vec<(String, String)> = self
            .request_query
            .clone()
            .into_iter()
            .filter_map(|el| el.pair())
            .collect();

        let request_header: Vec<(String, String)> = self
            .request_header
            .clone()
            .into_iter()
            .filter_map(|el| el.pair())
            .collect();

        let request_body_form: Vec<(String, String)> = self
            .request_body_form
            .clone()
            .into_iter()
            .filter_map(|el| el.pair())
            .collect();

        let request_body_form_data: Vec<(String, String)> = self
            .request_body_form_data
            .clone()
            .into_iter()
            .filter_map(|el| el.pair())
            .collect();

        let req_body_tab_idx = self.req_body_tab_idx;
        let body_raw_type_idx = self.request_body_raw_type_idx;
        let body_raw = self.request_body_raw.clone();

        self.rt.spawn(async move {
            let mut client = reqwest::Client::new();
            let mut request_builder = client.request(method, &url);

            // add query
            request_builder = request_builder.query(&request_query);

            // add header
            let mut has_content_type = false;
            for (k, v) in request_header {
                if k.to_lowercase() == "content-type" {
                    has_content_type = true;
                }
                request_builder = request_builder.header(k, v);
            }

            // add body
            match req_body_tab_idx {
                // Raw
                0 => {
                    if !body_raw.is_empty() {
                        match body_raw_type_idx {
                            // text
                            0 => {
                                if !has_content_type {
                                    request_builder = request_builder.header("Content-Type", "text/plain");
                                }

                                request_builder = request_builder.body(body_raw);
                            }

                            // json
                            1 => {
                                if !has_content_type {
                                    request_builder = request_builder.header("Content-Type", "application/json");
                                }

                                request_builder = request_builder.body(body_raw);
                            }

                            // form
                            2 => {
                                if !has_content_type {
                                    request_builder = request_builder.header("Content-Type", "application/x-www-form-urlencoded");
                                }

                                request_builder = request_builder.body(body_raw);
                            }

                            // xml
                            3 => {
                                if !has_content_type {
                                    request_builder = request_builder.header("Content-Type", "text/xml");
                                }

                                request_builder = request_builder.body(body_raw);
                            }

                            // binary file
                            4 => {
                                let binary_file_path = &body_raw;

                                if !std::path::Path::new(binary_file_path).exists() {
                                    sender.send(Err(anyhow!("binary file not exists")));
                                    return;
                                }

                                let Ok(dat) = std::fs::read(binary_file_path) else {
                                    sender.send(Err(anyhow!("binary file read error")));
                                    return;
                                };

                                if !has_content_type {
                                    request_builder = request_builder.header("Content-Type", "application/octet-stream");
                                }

                                request_builder = request_builder.body(dat);
                            }

                            _ => todo!(),
                        }
                    }
                }

                // Form
                1 => {
                    request_builder = request_builder.header("Content-Type", "application/x-www-form-urlencoded").form( &request_body_form );
                }

                // form-data
                2 => {
                    let mut form = reqwest::multipart::Form::new();

                    // name  bar
                    // file  @a.jpg
                    // files @a.jpg @b.jpg
                    for (k, v) in request_body_form_data  {
                        if !v.is_empty() && v.contains('@') {
                            let filepaths: Vec<_> = v.split('@').filter(|e| !e.is_empty()).map(|e| e.trim()).collect();
                            for filepath in filepaths {
                                let upload_file_path_p = std::path::Path::new(filepath);

                                let Ok(filename) = upload_file_path_p .file_name() .unwrap() .to_os_string() .into_string() else {
                                        sender.send(Err(anyhow!("get filename {} error", filepath)));
                                        return;
                                };

                                let Ok(file_body) = std::fs::read(filepath) else {
                                    sender.send(Err(anyhow!("read file {} error", filepath)));
                                    return;
                                };

                                form =  form.part(k.clone(), reqwest::multipart::Part::bytes(file_body).file_name(filename));
                            }
                        } else {
                            form =  form.text(k.clone(), v);
                        }
                    }

                    request_builder = request_builder.multipart(form);

                }

                _ => todo!(),
            };

            let response = match request_builder.send().await {
                Ok(r) => r,
                 Err(err) => {
                    sender.send(Err(anyhow!(err)));
                    return;
                }
            };

            let status = response.status();
            let version = response.version();
            let headers = response.headers().clone();
            let remote_addr = response.remote_addr();

            let mut data: Option<String> = None;
            let mut img: Option<Result<RetainedImage, String>> = None;
            let data_vec = response.bytes().await.and_then(|bs| Ok(bs.to_vec())).ok();

            if let Some(ct) = headers.get("content-type") {
                if let Ok(ct) = ct.to_str() {
                    if ct.starts_with("image/") {
                        if let Some(img_vec) = &data_vec {
                            img = Some(RetainedImage::from_image_bytes(&url, img_vec.as_ref()));
                        }
                    } else {
                        data = util::get_utf8_date(&data_vec).await;
                    }
                } else {
                    data = util::get_utf8_date(&data_vec).await;
                }
            } else {
                data = util::get_utf8_date(&data_vec).await;
            }

            let result = HttpResponse {
                url,
                data_vec,
                remote_addr,
                headers,
                version,
                status,
                data,
                img,
            };

            sender.send(Ok(result));
        });
    }
}

/* #region MyApp panel */
impl ApiTestApp {
    fn tabs_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            for (i, label) in K_REQ_TABS.iter().enumerate() {
                ui.selectable_value(&mut self.req_tab, label.clone(), label.to_string());
            }
        });
    }

    fn req_header_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.vertical(|ui| {
            if ui.button("添加").clicked() {
                self.request_header.push(PairUi::default());
            }
        });

        ui.separator();

        egui_extras::StripBuilder::new(ui)
            .size(egui_extras::Size::remainder().at_least(50.0).at_most(120.0))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    egui::ScrollArea::vertical()
                        .id_source("header scroll")
                        .show(ui, |ui| {
                            let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

                            let mut table = egui_extras::TableBuilder::new(ui)
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .column(egui_extras::Column::auto())
                                .column(
                                    egui_extras::Column::initial(K_COLUMN_WIDTH_INITIAL)
                                        .range(100.0..=400.0),
                                )
                                .column(
                                    egui_extras::Column::initial(K_COLUMN_WIDTH_INITIAL)
                                        .range(100.0..=400.0),
                                )
                                .column(
                                    egui_extras::Column::initial(100.0)
                                        .at_least(40.0)
                                        .at_most(400.0),
                                )
                                .min_scrolled_height(10.0);

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
                                    self.request_header.retain_mut(|el| {
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
                                                if ui.button("删除").clicked() {
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
    }

    fn req_query_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.vertical(|ui| {
            if ui.button("添加").clicked() {
                self.request_query.push(PairUi::default());
            }
        });

        ui.separator();

        egui_extras::StripBuilder::new(ui)
            .size(egui_extras::Size::remainder().at_least(50.0).at_most(120.0)) // for the table
            // .size(egui_extras::Size::initial(200.0)) // for the table
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    egui::ScrollArea::vertical().id_source("param scroll").show(ui, |ui| {
                        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

                        let mut table = egui_extras::TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(egui_extras::Column::auto())
                            .column(egui_extras::Column::initial(K_COLUMN_WIDTH_INITIAL).range(100.0..=400.0))
                            .column(egui_extras::Column::initial(K_COLUMN_WIDTH_INITIAL).range(100.0..=400.0))
                            .column(egui_extras::Column::initial(100.0).at_least(40.0).at_most(400.0))
                            // .column(egui_extras::Column::initial(100.0).range(40.0..=300.0))
                            // .column( egui_extras::Column::initial(100.0).at_least(40.0), )
                            // .column(egui_extras::Column::remainder())
                            // .max_scroll_height(200.0)
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
                                self.request_query.retain_mut(|el| {
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
                                            if ui.button("删除").clicked() {
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
    }

    fn req_body_raw_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.vertical(|ui| {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    for (i, raw_type) in K_REQ_BODY_RAW_TYPES.iter().enumerate() {
                        ui.radio_value(&mut self.request_body_raw_type_idx, i, raw_type.to_owned());
                    }
                });
            });
            ui.text_edit_multiline(&mut self.request_body_raw);
        });
    }

    fn req_body_form_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.vertical(|ui| {
            if ui.button("添加").clicked() {
                self.request_body_form.push(PairUi::default());
            }
        });

        ui.separator();

        egui_extras::StripBuilder::new(ui)
            .size(egui_extras::Size::remainder().at_least(50.0).at_most(120.0))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    egui::ScrollArea::vertical()
                        .id_source("body_form scroll")
                        .show(ui, |ui| {
                            let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

                            let mut table = egui_extras::TableBuilder::new(ui)
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .column(egui_extras::Column::auto())
                                .column(
                                    egui_extras::Column::initial(K_COLUMN_WIDTH_INITIAL)
                                        .range(100.0..=400.0),
                                )
                                .column(
                                    egui_extras::Column::initial(K_COLUMN_WIDTH_INITIAL)
                                        .range(100.0..=400.0),
                                )
                                .column(
                                    egui_extras::Column::initial(100.0)
                                        .at_least(40.0)
                                        .at_most(400.0),
                                )
                                .min_scrolled_height(10.0);

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
                                    self.request_body_form.retain_mut(|el| {
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
                                                if ui.button("删除").clicked() {
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
    }

    fn req_body_form_data_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.vertical(|ui| {
            if ui.button("添加").clicked() {
                self.request_body_form_data.push(PairUi::default());
            }
        });

        ui.separator();

        egui_extras::StripBuilder::new(ui)
            .size(egui_extras::Size::remainder().at_least(50.0).at_most(120.0))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    egui::ScrollArea::vertical()
                        .id_source("body_form_data scroll")
                        .show(ui, |ui| {
                            let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

                            let mut table = egui_extras::TableBuilder::new(ui)
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .column(egui_extras::Column::auto())
                                .column(
                                    egui_extras::Column::initial(K_COLUMN_WIDTH_INITIAL)
                                        .range(100.0..=400.0),
                                )
                                .column(
                                    egui_extras::Column::initial(K_COLUMN_WIDTH_INITIAL)
                                        .range(100.0..=400.0),
                                )
                                .column(
                                    egui_extras::Column::initial(100.0)
                                        .at_least(40.0)
                                        .at_most(400.0),
                                )
                                .min_scrolled_height(10.0);

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
                                    self.request_body_form_data.retain_mut(|el| {
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
                                                if ui.button("删除").clicked() {
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
    }

    fn tab_content_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        match self.req_tab {
            RequestTab::Params => {
                self.req_query_panel(ui, ctx);
            }
            RequestTab::Headers => {
                self.req_header_panel(ui, ctx);
            }
            RequestTab::Body => {
                ui.horizontal(|ui| {
                    for (i, label) in K_REQ_BODY_TABS.iter().enumerate() {
                        ui.selectable_value(&mut self.req_body_tab_idx, i, label.to_owned());
                    }
                });
                ui.painter();
                match self.req_body_tab_idx {
                    // row
                    0 => {
                        self.req_body_raw_panel(ui, ctx);
                    }

                    // Form
                    1 => {
                        self.req_body_form_panel(ui, ctx);
                    }

                    // form-data
                    2 => {
                        self.req_body_form_data_panel(ui, ctx);
                    }

                    _ => {
                        println!("??");
                    }
                }
            }
            _ => {
                panic!("?? req_tab_idx");
            }
        };
    }

    fn menu_panel(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 顶部菜单栏
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });
            });
        });
    }
}
/* #endregion */

impl eframe::App for ApiTestApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.menu_panel(ctx, frame);

        let _my_frame = egui::containers::Frame {
            inner_margin: egui::style::Margin {
                left: 10.,
                right: 10.,
                top: 10.,
                bottom: 10.,
            },
            outer_margin: egui::style::Margin {
                left: 10.,
                right: 10.,
                top: 10.,
                bottom: 10.,
            },
            rounding: egui::Rounding {
                nw: 1.0,
                ne: 1.0,
                sw: 1.0,
                se: 1.0,
            },
            shadow: eframe::epaint::Shadow {
                extrusion: 1.0,
                color: Color32::YELLOW,
            },
            fill: Color32::LIGHT_BLUE,
            stroke: egui::Stroke::new(2.0, Color32::GOLD),
        };

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Left Panel");
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label("text");
                });
            });

        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Right Panel");
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label("text");
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_source("method").show_index(
                    ui,
                    &mut self.method_idx,
                    K_REQ_METHODS.len(),
                    |i| K_REQ_METHODS[i].to_string(),
                );

                ui.add(egui::TextEdit::singleline(&mut self.url).desired_width(300.));

                let send_btn = egui::Button::new("发送").min_size(vec2(100.0, 40.0));
                if ui.add_enabled(!self.url.is_empty(), send_btn).clicked() {
                    self.http_send();
                }
            });

            ui.separator();

            self.tabs_panel(ui, ctx);
            ui.separator();

            self.tab_content_panel(ui, ctx);
            ui.separator();

            if let Some(response_promise) = &self.response_promise {
                match response_promise.ready() {
                    Some(response_r) => match response_r {
                        Ok(response) => {
                            ui.horizontal(|ui| {
                                ui.heading("Response Status:");

                                ui.label(format!("{}", response.status));

                                if let Some(remote_addr) = &response.remote_addr {
                                    ui.label(format!("{}", remote_addr));
                                };
                            });
                            ui.separator();

                            ui.horizontal(|ui| {
                                for (i, label) in K_RESPONSE_TABS.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.response_tab_idx,
                                        i,
                                        label.to_owned(),
                                    );
                                }
                            });
                            ui.separator();

                            match self.response_tab_idx {
                                // Data
                                0 => {
                                    ui.horizontal(|ui| {
                                        if let Some(data_vec) = &response.data_vec {
                                            if let Some(text_data) = &response.data {
                                                if ui.button("复制到剪切板").clicked() {
                                                    ui.output_mut(|o| {
                                                        o.copied_text = text_data.clone()
                                                    });
                                                };
                                            };

                                            ui.separator();

                                            ui.add(
                                                egui::TextEdit::singleline(
                                                    &mut self.response_data_download_path,
                                                )
                                                .hint_text(r#"c:\o.jpg"#),
                                            );
                                            if ui
                                                .add_enabled(
                                                    !self.response_data_download_path.is_empty(),
                                                    egui::Button::new("下载"),
                                                )
                                                .clicked()
                                            {
                                                let download_path =
                                                    self.response_data_download_path.clone();

                                                let p = std::path::Path::new(&download_path);
                                                let p_dir = p.parent();

                                                if let Some(p_dir) = p_dir {
                                                    if p_dir.is_dir() && p_dir.exists() {
                                                        let contents: Vec<u8> = data_vec.clone();
                                                        self.rt.spawn(async move {
                                                            let contents: &[u8] = contents.as_ref();
                                                            if let Err(err) = tokio::fs::write(
                                                                &download_path,
                                                                contents,
                                                            )
                                                            .await
                                                            {
                                                                println!(
                                                                    "download error: {}",
                                                                    err.to_string()
                                                                );
                                                            };
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    });

                                    if let Some(img_data) = &response.img {
                                        match img_data {
                                            Ok(image) => {
                                                image.show_max_size(
                                                    ui,
                                                    [K_IMAGE_MAX_WIDTH, K_IMAGE_MAX_WIDTH].into(),
                                                );
                                            }
                                            Err(err) => {
                                                ui.label(err.as_str());
                                            }
                                        }
                                    } else if let Some(text_data) = &response.data {
                                        egui::ScrollArea::vertical()
                                            .id_source("data scroll")
                                            .always_show_scroll(true)
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                ui.label(text_data);
                                            });
                                    } else {
                                        widget::error_label(ui, "其他类型");
                                    }
                                }

                                // Header
                                1 => {
                                    egui::Grid::new("response header").show(ui, |ui| {
                                        response.headers.iter().for_each(|(name, val)| {
                                            ui.label(name.as_str());
                                            ui.label(val.to_str().unwrap_or(""));
                                            ui.end_row();
                                        });
                                    });
                                }

                                _ => {
                                    todo!();
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
        });
    }
}

struct HttpResponse {
    url: String,
    remote_addr: Option<SocketAddr>,
    headers: HeaderMap,
    version: reqwest::Version,
    status: reqwest::StatusCode,
    data: Option<String>,
    img: Option<Result<egui_extras::RetainedImage, String>>,
    data_vec: Option<Vec<u8>>,
}

#[derive(Default, Clone)]
pub struct PairUi {
    pub key: String,
    pub value: String,
    pub disable: bool,
}

impl PairUi {
    pub fn bad(&self) -> bool {
        self.key.is_empty() || self.disable
    }

    pub fn pair(self) -> Option<(String, String)> {
        if self.bad() {
            None
        } else {
            Some((self.key, self.value))
        }
    }
}

#[derive(PartialEq, Clone)]
enum RequestTab {
    Params,
    Headers,
    Body,
}

impl Display for RequestTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RequestTab::Params => "Params",
                RequestTab::Headers => "Headers",
                RequestTab::Body => "Body",
            }
        )
    }
}
