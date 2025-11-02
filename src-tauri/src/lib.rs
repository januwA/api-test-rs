use anyhow::{bail, Result};
use reqwest::{header::HeaderMap, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

// 公开导出模块
pub mod util;
pub mod script_engine;

// 重新导出常用类型
pub use util::*;
pub use script_engine::*;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(10000)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client")
    })
}

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

#[derive(Debug, Clone, Default)]
pub struct RequestStats {
    pub pending: usize,
    pub sending: usize,
    pub success: usize,
    pub failed: usize,
    pub response_times: Vec<u128>,
    pub total_start_time: Option<std::time::Instant>,
    pub total_end_time: Option<std::time::Instant>,
    pub total_upload_bytes: u64,
    pub total_download_bytes: u64,
    pub max_response_times: usize,
}

impl RequestStats {
    pub fn total_requests(&self) -> usize {
        self.success + self.failed
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 {
            0.0
        } else {
            (self.success as f64 / total as f64) * 100.0
        }
    }

    pub fn min_response_time(&self) -> Option<u128> {
        self.response_times.iter().min().copied()
    }

    pub fn max_response_time(&self) -> Option<u128> {
        self.response_times.iter().max().copied()
    }

    pub fn avg_response_time(&self) -> Option<f64> {
        if self.response_times.is_empty() {
            None
        } else {
            let sum: u128 = self.response_times.iter().sum();
            Some(sum as f64 / self.response_times.len() as f64)
        }
    }

    pub fn percentile(&self, p: f64) -> Option<u128> {
        if self.response_times.is_empty() {
            return None;
        }
        let mut sorted = self.response_times.clone();
        sorted.sort();
        let index = ((p / 100.0) * sorted.len() as f64).ceil() as usize - 1;
        sorted.get(index.min(sorted.len() - 1)).copied()
    }

    pub fn add_response_time(&mut self, time: u128) {
        if self.response_times.len() < self.max_response_times {
            self.response_times.push(time);
        } else if self.max_response_times > 0 {
            // 使用Reservoir Sampling算法进行更好的采样
            // 每当收到第n个样本时，以k/n的概率替换现有样本
            // 其中k是采样池的大小
            let n = self.total_requests() as f64;
            let k = self.max_response_times as f64;
            if rand::random::<f64>() < (k / n) {
                let idx = rand::random::<usize>() % self.max_response_times;
                self.response_times[idx] = time;
            }
        }
    }

    pub fn qps(&self) -> Option<f64> {
        if let (Some(start), Some(end)) = (self.total_start_time, self.total_end_time) {
            let duration = end.duration_since(start).as_secs_f64();
            if duration > 0.0 {
                Some(self.total_requests() as f64 / duration)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn total_duration(&self) -> Option<f64> {
        if let (Some(start), Some(end)) = (self.total_start_time, self.total_end_time) {
            Some(end.duration_since(start).as_secs_f64())
        } else {
            None
        }
    }

    pub fn current_duration(&self) -> Option<f64> {
        if let Some(start) = self.total_start_time {
            Some(std::time::Instant::now().duration_since(start).as_secs_f64())
        } else {
            None
        }
    }

    pub fn upload_throughput_mbps(&self) -> Option<f64> {
        if let Some(duration) = self.total_duration() {
            if duration > 0.0 {
                Some((self.total_upload_bytes as f64 / 1024.0 / 1024.0) / duration)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn download_throughput_mbps(&self) -> Option<f64> {
        if let Some(duration) = self.total_duration() {
            if duration > 0.0 {
                Some((self.total_download_bytes as f64 / 1024.0 / 1024.0) / duration)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn realtime_qps(&self) -> Option<f64> {
        if let Some(duration) = self.current_duration() {
            if duration > 0.0 {
                Some(self.total_requests() as f64 / duration)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn realtime_upload_throughput_mbps(&self) -> Option<f64> {
        if let Some(duration) = self.current_duration() {
            if duration > 0.0 {
                Some((self.total_upload_bytes as f64 / 1024.0 / 1024.0) / duration)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn realtime_download_throughput_mbps(&self) -> Option<f64> {
        if let Some(duration) = self.current_duration() {
            if duration > 0.0 {
                Some((self.total_download_bytes as f64 / 1024.0 / 1024.0) / duration)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    // 脚本支持
    /// 请求前脚本
    #[serde(default)]
    pub pre_request_script: String,
    /// 响应后脚本
    #[serde(default)]
    pub post_response_script: String,
    /// 是否启用脚本
    #[serde(default)]
    pub script_enabled: bool,
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
            pre_request_script: String::new(),
            post_response_script: String::new(),
            script_enabled: false,
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

        let client = get_http_client();

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub stats: RequestStats,

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
        let max_samples = 100000.min(self.send_count);
        self.stats = RequestStats {
            pending: self.send_count,
            sending: 0,
            success: 0,
            failed: 0,
            response_times: Vec::with_capacity(max_samples),
            total_start_time: Some(std::time::Instant::now()),
            total_end_time: None,
            total_upload_bytes: 0,
            total_download_bytes: 0,
            max_response_times: max_samples,
        };
    }
    pub fn from_name(name: String) -> Self {
        Self {
            name,
            ..Self::default()
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
            stats: Default::default(),
            send_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    #[serde(skip)]
    pub headers: HeaderMap,
    pub headers_str: String,
    pub request_headers_str: String,
    #[serde(with = "version_serde")]
    pub version: reqwest::Version,
    pub status: u16,
    pub img: Option<()>,
    pub text: Option<String>,
    pub data_vec: Option<Vec<u8>>,
    pub duration: u128,
    pub request_size: u64,
    pub response_size: u64,
    /// 脚本修改后的环境变量
    pub modified_vars: Option<Vec<PairUi>>,
}

mod version_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use reqwest::Version;

    pub fn serialize<S>(version: &Version, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:?}", version))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Version, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _s = String::deserialize(deserializer)?;
        Ok(Version::HTTP_11) // Default version
    }
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

    pub fn content_type_video(&self) -> bool {
        self.content_type()
            .and_then(|v| Some(v.starts_with("video/")))
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
    Scripts,
    Curl,
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
    Stats,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub children: Vec<HttpTest>,

    #[serde(skip)]
    pub new_child_name: String,
}

impl Group {
    pub fn from_name(name: String) -> Self {
        Group {
            name,
            children: Default::default(),
            new_child_name: Default::default(),
        }
    }

    pub fn create_child(&mut self) {
        if !self.new_child_name.is_empty() {
            self.children
                .push(HttpTest::from_name(self.new_child_name.to_owned()));
            self.new_child_name.clear();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub project_path: String,
    pub font_size: f32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
