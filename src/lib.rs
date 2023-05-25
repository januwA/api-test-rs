use anyhow::{anyhow, bail, Result};
use reqwest::{header::HeaderMap, RequestBuilder};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot::{self, error::TryRecvError};
mod util;

const CONTENT_TYPE: &str = "Content-Type";
const TEXT_PLAIN: &str = "text/plain";
const TEXT_XML: &str = "text/xml";
const APPLICATION_JSON: &str = "application/json";
const APPLICATION_FORM: &str = "application/x-www-form-urlencoded";
const APPLICATION_STREAM: &str = "application/octet-stream";

#[derive(Debug, Serialize, Deserialize)]
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

impl HttpRequestConfig {
    pub async fn request_build(self, vars: &Vec<PairUi>) -> Result<RequestBuilder> {
        let HttpRequestConfig {
            body_tab_ui,
            body_raw_type,
            url,
            ..
        } = self;

        let real_url = util::parse_var_str(&url, vars);
        let method = self.method.as_reqwest_method();
        let request_query = util::real_part_vec(self.query, vars);
        let request_header = util::real_part_vec(self.header, vars);
        let request_body_form = util::real_part_vec(self.body_form, vars);
        let request_body_form_data = util::real_part_vec(self.body_form_data, vars);

        let body_raw = self.body_raw;
        // let body_raw = util::parse_var_str(&self.body_raw, vars);

        let client = reqwest::Client::new();

        let mut request_builder = client.request(method, &real_url);

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
}

#[derive(Serialize, Deserialize)]
pub struct HttpConfig {
    pub name: String,
    pub tab_ui: RequestTab,

    pub send_count_ui: String,

    pub req_cfg: HttpRequestConfig,

    #[serde(skip)]
    pub send_count: usize,

    #[serde(skip)]
    pub response_promise: Option<Promise<Result<HttpResponseUi>>>,

    #[serde(skip)]
    pub response_promise_vec: Vec<Promise<Result<HttpResponseUi>>>,

    #[serde(skip)]
    pub s_e_r: (usize, usize, usize),

    #[serde(skip)]
    pub download_path: String,

    #[serde(skip)]
    pub response_tab_ui: ResponseTab,
}

impl HttpConfig {
    pub fn get_request_reper(&mut self) -> (usize, usize, usize) {
        if self.s_e_r.0 + self.s_e_r.1 >= self.send_count {
            if !self.response_promise_vec.is_empty() {
                self.response_promise.get_or_insert_with(|| {
                    let l = self.response_promise_vec.remove(0); // 显示第一个数据
                    self.response_promise_vec.clear(); // 清理掉结果
                    l
                });
            }
            return self.s_e_r;
        }

        self.s_e_r = (0, 0, 0);
        self.response_promise_vec
            .iter_mut()
            .for_each(|p| match p.read() {
                PromiseStatus::PADING => {
                    self.s_e_r.2 += 1;
                }
                PromiseStatus::Fulfilled(_) => {
                    self.s_e_r.0 += 1;
                }
                PromiseStatus::Rejected(_) => {
                    self.s_e_r.1 += 1;
                }
            });

        self.s_e_r
    }

    pub fn from_name(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }
}

impl Clone for HttpConfig {
    fn clone(&self) -> Self {
        Self {
            name: self.name.to_owned(),
            tab_ui: self.tab_ui.to_owned(),
            response_promise: None,
            response_tab_ui: self.response_tab_ui.to_owned(),
            req_cfg: self.req_cfg.to_owned(),
            download_path: Default::default(),
            response_promise_vec: Default::default(),
            send_count_ui: self.send_count_ui.to_owned(),
            send_count: 0,
            s_e_r: (0, 0, 0),
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
            send_count_ui: String::from("1"),
            s_e_r: (0, 0, 0),
            send_count: 0,
        }
    }
}

pub struct HttpResponseUi {
    pub headers: HeaderMap,
    pub version: reqwest::Version,
    pub status: reqwest::StatusCode,
    pub data: Option<String>,
    pub img: Option<Result<egui_extras::RetainedImage, String>>,
    pub data_vec: Option<Vec<u8>>,
}

impl HttpResponseUi {
    pub fn content_type(&self) -> Option<&str> {
        self.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok())
    }

    pub fn content_type_image(&self) -> bool {
        self.content_type()
            .and_then(|v| Some(v.starts_with("image/")))
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

    pub fn pair(self) -> Option<(String, String)> {
        if self.bad() {
            None
        } else {
            Some((self.key, self.value))
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

#[derive(Debug)]
pub enum PromiseStatus<T> {
    PADING,
    Fulfilled(T),
    Rejected(anyhow::Error),
}

#[derive(Debug)]
pub struct Promise<T> {
    pub data: Option<T>,
    rx: oneshot::Receiver<T>,
}

impl<T> Promise<T> {
    pub fn new() -> (oneshot::Sender<T>, Self) {
        let (tx, rx) = oneshot::channel::<T>();
        (tx, Self { data: None, rx })
    }

    pub fn read(&mut self) -> PromiseStatus<&T> {
        match self.rx.try_recv() {
            Ok(data) => {
                self.data = Some(data);

                return if let Some(data) = &self.data {
                    PromiseStatus::Fulfilled(data)
                } else {
                    PromiseStatus::PADING
                };
            }
            Err(err) => match err {
                // 尚未发送值
                TryRecvError::Empty => return PromiseStatus::PADING,

                // 被丢弃，或则已经发送
                TryRecvError::Closed => {
                    if let Some(data) = &self.data {
                        return PromiseStatus::Fulfilled(data);
                    }
                    return PromiseStatus::Rejected(anyhow!(err));
                }
            },
        }
    }

    pub fn read_mut(&mut self) -> PromiseStatus<&mut T> {
        match self.rx.try_recv() {
            Ok(data) => {
                self.data = Some(data);

                return if let Some(data) = &mut self.data {
                    PromiseStatus::Fulfilled(data)
                } else {
                    PromiseStatus::PADING
                };
            }
            Err(err) => match err {
                // 尚未发送值
                TryRecvError::Empty => return PromiseStatus::PADING,

                // 被丢弃，或则已经发送
                TryRecvError::Closed => {
                    if let Some(data) = &mut self.data {
                        return PromiseStatus::Fulfilled(data);
                    }
                    return PromiseStatus::Rejected(anyhow!(err));
                }
            },
        }
    }
}
