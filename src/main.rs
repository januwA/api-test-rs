#![allow(warnings, unused)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::net::SocketAddr;

use anyhow::anyhow;
use eframe::egui::output::OpenUrl;
use eframe::egui::{Hyperlink, Ui};
use eframe::{
    egui::{self, RichText},
    epaint::Color32,
};
use egui_extras::RetainedImage;
use poll_promise::Promise;
use reqwest::header::HeaderMap;
use tokio::runtime::Runtime;

mod api;
mod cache;
mod util;
mod widget;

const K_IMAGE_MAX_WIDTH: f32 = 200.0;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    let mut options = eframe::NativeOptions::default();
    options.icon_data = Some(util::load_app_icon());

    // options.initial_window_pos = Some([0f32, 0f32].into());
    // options.min_window_size = Some([600f32, 400f32].into());

    // options.fullscreen = true;

    options.maximized = true;

    eframe::run_native(
        "sm ms",
        options,
        Box::new(|cc| Box::new(ApiTestApp::new(cc, cache::SmMsCacheData::get_or_create()))),
    )
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

struct ApiTestApp {
    upload_path: String,
    uplaod_res_msg: String,

    request_query: Vec<(String, String, bool)>,

    method_idx: usize,
    methods: Vec<reqwest::Method>,
    url: String,
    response_promise: Option<Promise<anyhow::Result<HttpResponse>>>,
    response_text: Option<String>,
    download_path: String,

    /* #region tab */
    tab: Vec<String>,
    tab_index: usize,
    /* #endregion */
    rt: Runtime,
}

impl Default for ApiTestApp {
    fn default() -> Self {
        Self {
            upload_path: Default::default(),
            uplaod_res_msg: Default::default(),
            tab: vec![
                String::from("Params"),
                String::from("Headers"),
                String::from("Body"),
            ],
            tab_index: Default::default(),
            rt: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
            method_idx: 0,
            methods: vec![
                reqwest::Method::GET,
                reqwest::Method::POST,
                reqwest::Method::DELETE,
            ],
            url: "http://127.0.0.1:8080".to_string(),
            response_text: Default::default(),
            response_promise: Default::default(),
            download_path: Default::default(),
            request_query: Default::default(),
        }
    }
}

/* #region MyApp constructor */
impl ApiTestApp {
    fn new(cc: &eframe::CreationContext<'_>, cache_data: Option<cache::SmMsCacheData>) -> Self {
        util::setup_custom_fonts(&cc.egui_ctx);
        let mut my = Self::default();
        if let Some(cache_data) = cache_data {
            if let Some(token) = cache_data.token {}
        }

        my.init();
        my
    }

    fn init(&mut self) {}
}
/* #endregion */

/* #region MyApp panel */
impl ApiTestApp {
    fn tabs_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            for (i, label) in self.tab.iter().enumerate() {
                if ui.selectable_label(self.tab_index == i, label).clicked() {
                    self.tab_index = i;
                }
            }
        });
    }

    fn tab_content_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        match self.tab_index {
            0 => {
                ui.vertical(|ui| {
                    if ui.button("添加").clicked() {
                        self.request_query.push(("".into(), "".into(), false));
                    }
                });
                egui::Grid::new("params")
                    .min_col_width(120.)
                    .show(ui, |ui| {
                        ui.label("禁用");
                        ui.label("Key");
                        ui.label("Value");
                        ui.end_row();
                        self.request_query.retain_mut(|el| {
                            ui.checkbox(&mut el.2, "");
                            ui.add(egui::TextEdit::singleline(&mut el.0).desired_width(200f32));
                            ui.add(egui::TextEdit::singleline(&mut el.1).desired_width(200f32));
                            if ui.button("删除").clicked() {
                                return false;
                            }
                            ui.end_row();
                            return true;
                        });
                    });
            }
            1 => {
                ui.label("headers");
            }
            2 => {
                ui.label("body");
            }
            _ => {
                todo!();
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

        egui::CentralPanel::default()
            // .frame(my_frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("method").show_index(
                        ui,
                        &mut self.method_idx,
                        self.methods.len(),
                        |i| self.methods[i].to_string(),
                    );

                    ui.add(egui::TextEdit::singleline(&mut self.url).desired_width(300.));

                    if ui
                        .add_enabled(!self.url.is_empty(), egui::Button::new("发送"))
                        .clicked()
                    {
                        let (sender, response_promise) = Promise::new();
                        self.response_promise = Some(response_promise);

                        let method = self.methods[self.method_idx].clone();
                        let url: String = self.url.clone();
                        let request_query: Vec<(String, String)> = self
                            .request_query
                            .clone()
                            .into_iter()
                            .filter_map(|el| {
                                if el.2 || el.0.is_empty() {
                                    None
                                } else {
                                    Some((el.0, el.1))
                                }
                            })
                            .collect();

                        self.rt.spawn(async move {
                            let mut client = reqwest::Client::new();
                            let mut request_builder = client.request(method, &url);

                            request_builder = request_builder.query(&request_query);

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
                            let data_vec =
                                response.bytes().await.and_then(|bs| Ok(bs.to_vec())).ok();

                            if let Some(ct) = headers.get("content-type") {
                                if let Ok(ct) = ct.to_str() {
                                    if ct.starts_with("image/") {
                                        if let Some(img_vec) = &data_vec {
                                            img = Some(RetainedImage::from_image_bytes(
                                                &url,
                                                img_vec.as_ref(),
                                            ));
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
                                ui.heading("Response Headers:");
                                egui::Grid::new("response header").show(ui, |ui| {
                                    response.headers.iter().for_each(|(name, val)| {
                                        ui.label(name.as_str());
                                        ui.label(val.to_str().unwrap_or(""));
                                        ui.end_row();
                                    });
                                });

                                ui.separator();
                                ui.heading("Response Data:");
                                ui.separator();

                                if let Some(data_vec) = &response.data_vec {
                                    ui.horizontal(|ui| {
                                        ui.label("下载到:");
                                        // ui.text_edit_singleline(&mut self.download_path);

                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.download_path)
                                                .hint_text(r#"like: c:\o.jpg c:\o.mp4 c:\o.pdf"#),
                                        );
                                        ui.label("文件");
                                        if ui
                                            .add_enabled(
                                                !self.download_path.is_empty(),
                                                egui::Button::new("下载"),
                                            )
                                            .clicked()
                                        {
                                            let download_path = self.download_path.clone();
                                            let contents: Vec<u8> = data_vec.clone();
                                            self.rt.spawn(async move {
                                                let contents: &[u8] = contents.as_ref();
                                                if let Err(err) =
                                                    tokio::fs::write(&download_path, contents).await
                                                {
                                                    println!("download error: {}", err.to_string());
                                                };
                                            });
                                        }
                                    });
                                    ui.separator();
                                }

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
                                    ui.vertical(|ui| {
                                        if ui.button("复制到剪切板").clicked() {
                                            ui.output_mut(|o| o.copied_text = text_data.clone());
                                        };
                                        egui::ScrollArea::vertical()
                                            .always_show_scroll(true)
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                ui.label(text_data);
                                            });
                                    });
                                } else {
                                    widget::error_label(ui, "其他类型");
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
