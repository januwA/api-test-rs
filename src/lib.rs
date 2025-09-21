use anyhow::{bail, Result};
use reqwest::{header::HeaderMap, RequestBuilder};
use serde::{Deserialize, Serialize};
mod util;

const CONTENT_TYPE: &str = "Content-Type";
const TEXT_PLAIN: &str = "text/plain";
const TEXT_XML: &str = "text/xml";
const APPLICATION_JSON: &str = "application/json";
const APPLICATION_FORM: &str = "application/x-www-form-urlencoded";
const APPLICATION_STREAM: &str = "application/octet-stream";

#[derive(Debug, Clone)]
pub enum WsMessage {
    Init(HttpRequestConfig, Vec<PairUi>),
    Send(HttpRequestConfig, Vec<PairUi>),
    Close,
    ReadMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequestConfig {
    pub method: Method,
    pub url: String,
    pub body_tab_ui: RequestBodyTab,
    pub query: Vec<PairUi>,
    pub header: Vec<PairUi>,
    pub body_form: Vec<PairUi>,
    pub body_form_data: Vec<PairUi>,
    // 原始字符串
    pub body_raw: String,
    // 原始字符串类型
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

impl HttpRequestConfig {
    pub async fn request_build(&self, vars: &Vec<PairUi>) -> Result<RequestBuilder> {
        let HttpRequestConfig {
            body_tab_ui,
            body_raw_type,
            url,
            ..
        } = self;

        let method = self.method.as_reqwest_method();

        // 处理变量
        let real_url = util::parse_var_str(&url, vars);
        let request_query = util::real_tuple_vec(&self.query, vars);
        let request_header = util::real_tuple_vec(&self.header, vars);
        let request_body_form = util::real_tuple_vec(&self.body_form, vars);
        let request_body_form_data = util::real_tuple_vec(&self.body_form_data, vars);

        let body_raw = self.body_raw.to_owned();
        // let body_raw = util::parse_var_str(&self.body_raw, vars);

        let client = reqwest::Client::new();

        let mut request_builder = client.request(method, &real_url);

        // add query
        request_builder = request_builder.query(&request_query);

        // add header
        let mut has_content_type = false;
        for (k, v) in &request_header {
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
                request_builder.multipart(util::handle_multipart(&request_body_form_data).await?)
            }
        };

        Ok(request_builder)
    }
}

#[derive(Serialize, Deserialize)]
pub struct HttpTest {
    pub name: String,
    pub tab_ui: RequestTab,
    pub send_count_ui: String,

    pub request: HttpRequestConfig,

    #[serde(skip)]
    pub send_count: usize,

    #[serde(skip)]
    pub response: Option<HttpResponse>,

    #[serde(skip)]
    pub response_vec: Vec<HttpResponse>,

    #[serde(skip)]
    pub s_e: (usize, usize),

    #[serde(skip)]
    pub download_path: String,

    #[serde(skip)]
    pub response_tab_ui: ResponseTab,
}

impl HttpTest {
    pub fn send_before_init(&mut self) {
        self.send_count = self.send_count_ui.parse().unwrap_or(0);
        self.response = None;
        self.response_vec.clear();
        // init result vec size
        self.response_vec = Vec::with_capacity(self.send_count);
        self.s_e = (0, 0);
    }
    pub fn from_name(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }
}

impl Clone for HttpTest {
    fn clone(&self) -> Self {
        Self {
            name: self.name.to_owned(),
            tab_ui: self.tab_ui.to_owned(),
            response: None,
            response_tab_ui: self.response_tab_ui.to_owned(),
            request: self.request.to_owned(),
            download_path: Default::default(),
            response_vec: Default::default(),
            send_count_ui: self.send_count_ui.to_owned(),
            send_count: 0,
            s_e: (0, 0),
        }
    }
}

impl Default for HttpTest {
    fn default() -> Self {
        Self {
            name: "ApiTest".to_owned(),
            response: Default::default(),
            download_path: Default::default(),
            tab_ui: RequestTab::Params,
            response_tab_ui: ResponseTab::Data,
            request: HttpRequestConfig::default(),
            response_vec: Default::default(),
            send_count_ui: String::from("1"),
            s_e: (0, 0),
            send_count: 0,
        }
    }
}

pub struct HttpResponse {
    pub headers: HeaderMap,
    pub headers_str: String,
    pub version: reqwest::Version,
    pub status: reqwest::StatusCode,
    pub img: Option<()>,
    pub text: Option<String>,
    pub data_vec: Option<Vec<u8>>,
    pub duration: u128,
}

impl HttpResponse {
    pub fn content_type(&self) -> Option<&str> {
        self.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok())
    }

    pub fn content_type_image(&self) -> bool {
        self.content_type()
            .and_then(|v| Some(v.starts_with("image/")))
            .unwrap_or(false)
    }

    pub fn content_type_json(&self) -> bool {
        self.content_type()
            .and_then(|v| Some(v.contains(APPLICATION_JSON)))
            .unwrap_or(false)
    }
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

    pub fn tuple(&self) -> Option<(&str, &str)> {
        if self.bad() {
            None
        } else {
            Some((&self.key, &self.value))
        }
    }

    pub fn from_kv(k: &str, v: &str) -> Self {
        Self {
            key: k.to_owned(),
            value: v.to_owned(),
            disable: false,
        }
    }
}

#[derive(Debug, strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResponseTab {
    Data,
    Header,
}

impl Default for ResponseTab {
    fn default() -> Self {
        ResponseTab::Data
    }
}

#[derive(Debug, strum::AsRefStr, Clone, PartialEq, Serialize, Deserialize)]
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
    WS,
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
    pub childrent: Vec<HttpTest>,

    #[serde(skip)]
    pub new_child_name: String,
}

impl Group {
    pub fn from_name(name: String) -> Self {
        Group {
            name,
            childrent: Default::default(),
            new_child_name: Default::default(),
        }
    }

    pub fn create_child(&mut self) {
        if !self.new_child_name.is_empty() {
            self.childrent
                .push(HttpTest::from_name(self.new_child_name.to_owned()));
            self.new_child_name.clear();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub project_path: String,
}

impl AppConfig {
    pub fn load(dir: &str) -> Result<Self> {
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

#[derive(Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub groups: Vec<Group>,
    pub variables: Vec<PairUi>,
}

impl Project {
    pub fn from_name(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            groups: Default::default(),
            variables: Default::default(),
        }
    }
}
