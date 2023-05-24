use anyhow::bail;
use egui_extras::RetainedImage;
use poll_promise::Promise;
use reqwest::{header::HeaderMap, RequestBuilder};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
mod util;

const CONTENT_TYPE: &str = "Content-Type";
const TEXT_PLAIN: &str = "text/plain";
const TEXT_XML: &str = "text/xml";
const APPLICATION_JSON: &str = "application/json";
const APPLICATION_FORM: &str = "application/x-www-form-urlencoded";
const APPLICATION_STREAM: &str = "application/octet-stream";

#[derive(Serialize, Deserialize)]
pub struct HttpRequestConfig {
    pub method: Method,
    pub url: String,
    pub body_tab_ui: RequestBodyTab,
    pub query: Vec<PairUi>,
    pub header: Vec<PairUi>,
    pub body_raw: String,
    pub body_form: Vec<PairUi>,
    pub body_form_data: Vec<PairUi>,
    pub body_raw_type: RequestBodyRawType,
}

impl Clone for HttpRequestConfig {
    fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            url: self.url.clone(),
            body_tab_ui: self.body_tab_ui.clone(),
            query: self.query.clone(),
            header: self.header.clone(),
            body_form: self.body_form.clone(),
            body_form_data: self.body_form_data.clone(),
            body_raw: self.body_raw.clone(),
            body_raw_type: self.body_raw_type.clone(),
        }
    }
}

impl Default for HttpRequestConfig {
    fn default() -> Self {
        Self {
            method: Method::GET,
            url: Default::default(),
            body_tab_ui: RequestBodyTab::Raw,
            body_raw: Default::default(),
            body_raw_type: RequestBodyRawType::Json,
            query: Default::default(),
            header: Default::default(),
            body_form: Default::default(),
            body_form_data: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct HttpConfig {
    pub name: String,
    pub tab_ui: RequestTab,

    pub send_count_ui: String,

    pub req_cfg: HttpRequestConfig,

    #[serde(skip)]
    pub response_promise: Option<Promise<anyhow::Result<HttpResponse>>>,

    #[serde(skip)]
    pub response_promise_vec: Vec<Promise<anyhow::Result<HttpResponse>>>,

    #[serde(skip)]
    pub success_count: usize,

    #[serde(skip)]
    pub error_count: usize,

    #[serde(skip)]
    pub download_path: String,

    #[serde(skip)]
    pub response_tab_ui: ResponseTab,
}

impl HttpConfig {
    pub fn send_count(&self) -> usize {
        self.send_count_ui.parse().unwrap_or(0)
    }

    pub fn get_request_reper(&mut self) -> (usize, usize, usize) {
        let mut success: usize = 0;
        let mut error: usize = 0;
        let mut ready: usize = 0;

        self.response_promise_vec
            .iter()
            .for_each(|p| match p.ready() {
                Some(result) => match result {
                    Ok(res) => {
                        if res.status == reqwest::StatusCode::OK {
                            success += 1;
                        } else {
                            error += 1;
                            println!("status error: {}", res.status);
                        }
                    }
                    Err(err) => {
                        error += 1;
                        println!("error: {}", err);
                    }
                },
                None => {
                    ready += 1;
                }
            });

        (success, error, ready)
    }

    pub fn from_name(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    pub async fn request_builder_from_cfg(
        req_cfg: HttpRequestConfig,
    ) -> anyhow::Result<RequestBuilder> {
        let HttpRequestConfig {
            body_tab_ui,
            body_raw_type,
            body_raw,
            url,
            ..
        } = req_cfg;

        let method = req_cfg.method.as_reqwest_method();
        let request_query = util::part_vec(req_cfg.query);
        let request_header = util::part_vec(req_cfg.header);
        let request_body_form = util::part_vec(req_cfg.body_form);
        let request_body_form_data = util::part_vec(req_cfg.body_form_data);

        let client = reqwest::Client::new();
        let mut request_builder = client.request(method, &url);

        // add query
        request_builder = request_builder.query(&request_query);

        // add header
        let mut has_content_type = false;
        for (k, v) in request_header {
            if k.to_lowercase() == CONTENT_TYPE {
                has_content_type = true;
            }
            request_builder = request_builder.header(k, v);
        }

        // add body
        request_builder = match body_tab_ui {
            RequestBodyTab::Raw => {
                if !body_raw.is_empty() {
                    request_builder = match body_raw_type {
                        RequestBodyRawType::Text => {
                            if !has_content_type {
                                request_builder = request_builder.header(CONTENT_TYPE, TEXT_PLAIN);
                            }

                            request_builder.body(body_raw)
                        }

                        RequestBodyRawType::Json => {
                            if !has_content_type {
                                request_builder =
                                    request_builder.header(CONTENT_TYPE, APPLICATION_JSON);
                            }

                            request_builder.body(body_raw)
                        }

                        RequestBodyRawType::Form => {
                            if !has_content_type {
                                request_builder =
                                    request_builder.header(CONTENT_TYPE, APPLICATION_FORM);
                            }

                            request_builder.body(body_raw)
                        }

                        RequestBodyRawType::XML => {
                            if !has_content_type {
                                request_builder = request_builder.header(CONTENT_TYPE, TEXT_XML);
                            }

                            request_builder.body(body_raw)
                        }

                        RequestBodyRawType::BinaryFile => {
                            let dat = util::read_binary(&body_raw).await?;

                            if !has_content_type {
                                request_builder =
                                    request_builder.header(CONTENT_TYPE, APPLICATION_STREAM);
                            }

                            request_builder.body(dat)
                        }
                    };
                }
                request_builder
            }

            RequestBodyTab::Form => request_builder
                .header(CONTENT_TYPE, APPLICATION_FORM)
                .form(&request_body_form),

            RequestBodyTab::FormData => {
                request_builder.multipart(util::handle_multipart(request_body_form_data).await?)
            }
        };

        Ok(request_builder)
    }

    pub async fn http_send(request_builder: RequestBuilder) -> anyhow::Result<HttpResponse> {
        let response = request_builder.send().await?;

        let status = response.status();
        let version = response.version();
        let headers = response.headers().to_owned();

        let mut data: Option<String> = None;
        let mut img: Option<Result<RetainedImage, String>> = None;
        let data_vec = response.bytes().await.and_then(|bs| Ok(bs.to_vec())).ok();

        if let Some(ct) = headers.get(CONTENT_TYPE) {
            if let Ok(ct) = ct.to_str() {
                if ct.starts_with("image/") {
                    if let Some(img_vec) = &data_vec {
                        img = Some(RetainedImage::from_image_bytes("", img_vec.as_ref()));
                    }
                } else {
                    data = util::get_utf8_data(&data_vec).await;
                }
            } else {
                data = util::get_utf8_data(&data_vec).await;
            }
        } else {
            data = util::get_utf8_data(&data_vec).await;
        }

        Ok(HttpResponse {
            data_vec,
            headers,
            version,
            status,
            data,
            img,
        })
    }

    pub fn http_send_promise(
        rt: &Runtime,
        req_cfg: HttpRequestConfig,
    ) -> Promise<anyhow::Result<HttpResponse>> {
        let (sender, response_promise) = Promise::new();

        rt.spawn(async move {
            match HttpConfig::request_builder_from_cfg(req_cfg).await {
                Ok(request_builder) => match HttpConfig::http_send(request_builder).await {
                    Ok(data) => sender.send(Ok(data)),
                    Err(err) => sender.send(Err(err)),
                },
                Err(err) => {
                    sender.send(Err(err));
                }
            }
        });

        return response_promise;
    }
}

impl Clone for HttpConfig {
    fn clone(&self) -> Self {
        let mut name = self.name.clone();
        name.push_str(" Copy");

        Self {
            name,
            tab_ui: self.tab_ui.clone(),
            response_promise: None,
            response_tab_ui: self.response_tab_ui.clone(),
            req_cfg: self.req_cfg.clone(),
            download_path: Default::default(),
            response_promise_vec: Default::default(),
            success_count: 0,
            error_count: 0,
            send_count_ui: self.send_count_ui.to_owned(),
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            name: "ApiTest".to_owned(),
            response_promise: Default::default(),
            download_path: Default::default(),
            tab_ui: RequestTab::Params,
            response_tab_ui: ResponseTab::Data,
            req_cfg: HttpRequestConfig::default(),
            response_promise_vec: Default::default(),
            success_count: 0,
            error_count: 0,
            send_count_ui: String::from("1"),
        }
    }
}

pub struct HttpResponse {
    pub headers: HeaderMap,
    pub version: reqwest::Version,
    pub status: reqwest::StatusCode,
    pub data: Option<String>,
    pub img: Option<Result<egui_extras::RetainedImage, String>>,
    pub data_vec: Option<Vec<u8>>,
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
pub enum RequestTab {
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
pub enum RequestBodyTab {
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
pub enum RequestBodyRawType {
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
pub enum ResponseTab {
    Data,
    Header,
}

impl Default for ResponseTab {
    fn default() -> Self {
        ResponseTab::Data
    }
}

#[derive(strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
pub enum Method {
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

impl Method {
    pub fn as_reqwest_method(&self) -> reqwest::Method {
        reqwest::Method::from_bytes(self.as_ref().as_bytes()).unwrap()
    }
}

impl Default for Method {
    fn default() -> Self {
        Method::GET
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub childrent: Vec<HttpConfig>,

    #[serde(skip)]
    pub new_child_name: String,

    #[serde(skip)]
    pub del_child_name: String,
}

impl Group {
    pub fn from_name(name: String) -> Self {
        Group {
            name,
            childrent: Default::default(),
            new_child_name: Default::default(),
            del_child_name: Default::default(),
        }
    }

    pub fn create_child(&mut self) {
        if !self.new_child_name.is_empty() {
            self.childrent
                .push(HttpConfig::from_name(self.new_child_name.to_owned()));
            self.new_child_name.clear();
        }
    }

    pub fn del_child(&mut self) {
        if !self.del_child_name.is_empty() {
            if let Some(index) = self
                .childrent
                .iter()
                .position(|e| e.name == self.del_child_name)
            {
                self.childrent.remove(index);
                self.del_child_name.clear();
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub project_path: String,
}

impl AppConfig {
    pub fn load(dir: &str) -> anyhow::Result<Self> {
        let path = std::path::Path::new(dir).join(".config.json");

        if path.exists() {
            let data = std::fs::read(path)?;
            let dat: AppConfig = serde_json::from_slice(data.as_slice())?;
            Ok(dat)
        } else {
            bail!("config not exists")
        }
    }
}
