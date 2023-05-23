#![allow(warnings, unused)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::borrow::BorrowMut;
use std::fmt::Display;
use std::net::SocketAddr;

use anyhow::anyhow;
use eframe::egui::output::OpenUrl;
use eframe::egui::TextBuffer;
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

mod cache;
mod util;
mod widget;

/* #region const variables */
const IMAGE_MAX_WIDTH: f32 = 200.0;
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
    let mut options = eframe::NativeOptions::default();
    options.icon_data = Some(util::load_app_icon());

    // options.initial_window_pos = Some([0f32, 0f32].into());
    options.min_window_size = Some([900.0, 600.0].into());

    // options.fullscreen = true;
    options.maximized = false;

    eframe::run_native(
        "api test",
        options,
        Box::new(|cc| Box::new(ApiTestApp::new(cc))),
    )
}

/* #region App */
struct ApiTestApp {
    http_config: HttpConfig,
    rt: Runtime,
}

impl Default for ApiTestApp {
    fn default() -> Self {
        let mut http_config: Option<HttpConfig> = None;

        if let Ok(save_json) = std::fs::read("./save.json") {
            http_config = Some(serde_json::from_slice(save_json.as_ref()).unwrap());
        };

        Self {
            http_config: http_config.unwrap_or(HttpConfig::default()),
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        }
    }
}

impl ApiTestApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        util::setup_custom_fonts(&cc.egui_ctx);
        Self::default()
    }

    fn http_send(&mut self) {
        let (sender, response_promise) = Promise::new();
        self.http_config.response_promise = Some(response_promise);

        let method =
            reqwest::Method::from_bytes(self.http_config.method.as_ref().as_bytes()).unwrap();

        let url: String = self.http_config.url.clone();

        let request_query: Vec<(String, String)> = self
            .http_config
            .request_query
            .clone()
            .into_iter()
            .filter_map(|el| el.pair())
            .collect();

        let request_header: Vec<(String, String)> = self
            .http_config
            .request_header
            .clone()
            .into_iter()
            .filter_map(|el| el.pair())
            .collect();

        let request_body_form: Vec<(String, String)> = self
            .http_config
            .request_body_form
            .clone()
            .into_iter()
            .filter_map(|el| el.pair())
            .collect();

        let request_body_form_data: Vec<(String, String)> = self
            .http_config
            .request_body_form_data
            .clone()
            .into_iter()
            .filter_map(|el| el.pair())
            .collect();

        let req_body_tab_idx = self.http_config.request_body_tab.to_owned();
        let body_raw_type_idx = self.http_config.request_body_raw_type.to_owned();
        let body_raw = self.http_config.request_body_raw.clone();

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
                RequestBodyTab::Raw => {
                    if !body_raw.is_empty() {
                        match body_raw_type_idx {
                            RequestBodyRawType::Text => {
                                if !has_content_type {
                                    request_builder = request_builder.header("Content-Type", "text/plain");
                                }

                                request_builder = request_builder.body(body_raw);
                            }

                            RequestBodyRawType::Json => {
                                if !has_content_type {
                                    request_builder = request_builder.header("Content-Type", "application/json");
                                }

                                request_builder = request_builder.body(body_raw);
                            }

                            RequestBodyRawType::Form => {
                                if !has_content_type {
                                    request_builder = request_builder.header("Content-Type", "application/x-www-form-urlencoded");
                                }

                                request_builder = request_builder.body(body_raw);
                            }

                            RequestBodyRawType::XML => {
                                if !has_content_type {
                                    request_builder = request_builder.header("Content-Type", "text/xml");
                                }

                                request_builder = request_builder.body(body_raw);
                            }

                            RequestBodyRawType::BinaryFile => {
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

                RequestBodyTab::Form => {
                    request_builder = request_builder.header("Content-Type", "application/x-www-form-urlencoded").form( &request_body_form );
                }

                RequestBodyTab::FormData => {
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

impl ApiTestApp {
    fn tabs_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            for (i, label) in REQ_TABS.iter().enumerate() {
                let text = label.as_ref();
                ui.selectable_value(&mut self.http_config.request_tab, label.to_owned(), text);
            }
        });
    }

    fn req_header_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.vertical(|ui| {
            if ui.button("添加").clicked() {
                self.http_config.request_header.push(PairUi::default());
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
                                    egui_extras::Column::initial(COLUMN_WIDTH_INITIAL)
                                        .range(100.0..=400.0),
                                )
                                .column(
                                    egui_extras::Column::initial(COLUMN_WIDTH_INITIAL)
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
                                    self.http_config.request_header.retain_mut(|el| {
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
                self.http_config.request_query.push(PairUi::default());
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
                            .column(egui_extras::Column::initial(COLUMN_WIDTH_INITIAL).range(100.0..=400.0))
                            .column(egui_extras::Column::initial(COLUMN_WIDTH_INITIAL).range(100.0..=400.0))
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
                                self.http_config.request_query.retain_mut(|el| {
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
                    for (i, raw_type) in REQ_BODY_RAW_TYPES.iter().enumerate() {
                        ui.radio_value(
                            &mut self.http_config.request_body_raw_type,
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
                        egui::TextEdit::multiline(&mut self.http_config.request_body_raw)
                            .desired_rows(6),
                    );
                });
        });
    }

    fn req_body_form_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.vertical(|ui| {
            if ui.button("添加").clicked() {
                self.http_config.request_body_form.push(PairUi::default());
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
                                    egui_extras::Column::initial(COLUMN_WIDTH_INITIAL)
                                        .range(100.0..=400.0),
                                )
                                .column(
                                    egui_extras::Column::initial(COLUMN_WIDTH_INITIAL)
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
                                    self.http_config.request_body_form.retain_mut(|el| {
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
                self.http_config
                    .request_body_form_data
                    .push(PairUi::default());
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
                                    egui_extras::Column::initial(COLUMN_WIDTH_INITIAL)
                                        .range(100.0..=400.0),
                                )
                                .column(
                                    egui_extras::Column::initial(COLUMN_WIDTH_INITIAL)
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
                                    self.http_config.request_body_form_data.retain_mut(|el| {
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
        match self.http_config.request_tab {
            RequestTab::Params => {
                self.req_query_panel(ui, ctx);
            }
            RequestTab::Headers => {
                self.req_header_panel(ui, ctx);
            }
            RequestTab::Body => {
                ui.horizontal(|ui| {
                    for (i, label) in REQ_BODY_TABS.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.http_config.request_body_tab,
                            label.to_owned(),
                            label.as_ref(),
                        );
                    }
                });
                ui.painter();
                match self.http_config.request_body_tab {
                    RequestBodyTab::Raw => {
                        self.req_body_raw_panel(ui, ctx);
                    }

                    RequestBodyTab::Form => {
                        self.req_body_form_panel(ui, ctx);
                    }

                    RequestBodyTab::FormData => {
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
                    if ui.button("Save").clicked() {
                        if let Ok(save_json) = serde_json::to_vec(&self.http_config) {
                            if let Err(err) = std::fs::write("./save.json", save_json) {
                                println!("save error: {}", err);
                            }
                        }
                    }
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });

                ui.menu_button("Test Request", |ui| {
                    if ui.button("Add").clicked() {
                       todo!("add ad request")
                    }
                });
            });
        });
    }
}

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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_source("method")
                    .selected_text(self.http_config.method.as_ref())
                    .show_ui(ui, |ui| {
                        for m in &METHODS {
                            ui.selectable_value(
                                &mut self.http_config.method,
                                m.to_owned(),
                                m.as_ref(),
                            );
                        }
                    });

                ui.add(egui::TextEdit::singleline(&mut self.http_config.url).desired_width(500.));

                if ui
                    .add_enabled(!self.http_config.url.is_empty(), egui::Button::new("发送"))
                    .clicked()
                {
                    self.http_send();
                }
            });

            ui.separator();

            self.tabs_panel(ui, ctx);
            ui.separator();

            self.tab_content_panel(ui, ctx);
            ui.separator();

            if let Some(response_promise) = &self.http_config.response_promise {
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
                                for (i, label) in RESPONSE_TABS.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.http_config.response_tab,
                                        label.to_owned(),
                                        label.as_ref(),
                                    );
                                }
                            });
                            ui.separator();

                            match self.http_config.response_tab {
                                ResponseTab::Data => {
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
                                                    &mut self
                                                        .http_config
                                                        .response_data_download_path,
                                                )
                                                .hint_text(r#"c:\o.jpg"#),
                                            );
                                            if ui
                                                .add_enabled(
                                                    !self
                                                        .http_config
                                                        .response_data_download_path
                                                        .is_empty(),
                                                    egui::Button::new("下载"),
                                                )
                                                .clicked()
                                            {
                                                let download_path = self
                                                    .http_config
                                                    .response_data_download_path
                                                    .clone();

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
                                                    [IMAGE_MAX_WIDTH, IMAGE_MAX_WIDTH].into(),
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

                                ResponseTab::Header => {
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

/* #endregion */

/* #region other */

#[derive(Serialize, Deserialize)]
struct HttpConfig {
    method: Method,
    url: String,

    request_tab: RequestTab,
    request_body_tab: RequestBodyTab,

    request_query: Vec<PairUi>,
    request_header: Vec<PairUi>,
    request_body_form: Vec<PairUi>,
    request_body_form_data: Vec<PairUi>,
    request_body_raw: String,
    request_body_raw_type: RequestBodyRawType,

    #[serde(skip)]
    response_promise: Option<Promise<anyhow::Result<HttpResponse>>>,

    #[serde(skip)]
    response_data_download_path: String,

    #[serde(skip)]
    response_tab: ResponseTab,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            method: Method::GET,
            url: "http://127.0.0.1:3000/ping".to_string(),
            request_body_tab: RequestBodyTab::Raw,
            request_body_raw: Default::default(),
            response_promise: Default::default(),
            response_data_download_path: Default::default(),
            request_body_raw_type: RequestBodyRawType::Json,
            request_query: vec![PairUi::default()],
            request_header: vec![PairUi::default()],
            request_body_form: vec![PairUi::default()],
            request_body_form_data: vec![PairUi::default()],
            request_tab: RequestTab::Params,
            response_tab: ResponseTab::Data,
        }
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

#[derive(strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
enum RequestTab {
    Params,
    Headers,
    Body,
}
impl Default for RequestTab {
    fn default() -> Self {
        RequestTab::Params
    }
}

#[derive(strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
enum RequestBodyTab {
    Raw,
    Form,
    FormData,
}

impl Default for RequestBodyTab {
    fn default() -> Self {
        RequestBodyTab::Raw
    }
}

#[derive(strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
enum RequestBodyRawType {
    /// 出入json文本
    Json,
    /// 字符串文本
    Text,
    /// foo=bar&foo=bar
    Form,
    /// xml 文本
    XML,

    /// 本地文件路径，或则http/https开始的文件
    BinaryFile,
}
impl Default for RequestBodyRawType {
    fn default() -> Self {
        RequestBodyRawType::Json
    }
}

#[derive(strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
enum ResponseTab {
    Data,
    Header,
}

impl Default for ResponseTab {
    fn default() -> Self {
        ResponseTab::Data
    }
}

#[derive(strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
enum Method {
    OPTIONS,
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    TRACE,
    CONNECT,
    PATCH,
}

impl Default for Method {
    fn default() -> Self {
        Method::GET
    }
}
/* #endregion */
