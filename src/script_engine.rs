#![allow(warnings, unused)]

use anyhow::{bail, Result};
use rhai::{Dynamic, Engine, Map, Scope, AST};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 脚本执行上下文 - 请求前
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreRequestContext {
    /// 请求URL
    pub url: String,
    /// 请求方法
    pub method: String,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 查询参数
    pub params: HashMap<String, String>,
    /// 请求体
    pub body: String,
    /// 环境变量
    pub variables: HashMap<String, String>,
}

/// 脚本执行上下文 - 响应后
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponseContext {
    /// 请求上下文
    pub request: PreRequestContext,
    /// 响应状态码
    pub status: u16,
    /// 响应头
    pub headers: HashMap<String, String>,
    /// 响应体(文本)
    pub body: String,
    /// 响应时间(ms)
    pub duration: u128,
    /// 环境变量(可修改)
    pub variables: HashMap<String, String>,
}

/// 脚本执行结果
#[derive(Debug, Clone)]
pub struct ScriptResult {
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 修改后的上下文
    pub context: ScriptContext,
    /// 控制台输出
    pub console_output: Vec<String>,
}

/// 统一的脚本上下文
#[derive(Debug, Clone)]
pub enum ScriptContext {
    PreRequest(PreRequestContext),
    PostResponse(PostResponseContext),
}

/// 脚本引擎
pub struct ScriptEngine {
    engine: Engine,
}

impl ScriptEngine {
    /// 创建新的脚本引擎实例
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // 注册加密函数
        Self::register_crypto_functions(&mut engine);

        // 注册编码函数
        Self::register_encoding_functions(&mut engine);

        // 注册 JSON 函数
        Self::register_json_functions(&mut engine);

        // 注册工具函数
        Self::register_utility_functions(&mut engine);

        // 注册 console_log 函数
        Self::register_console_functions(&mut engine);

        // 注册文件操作函数
        Self::register_file_functions(&mut engine);

        // 注册网络请求函数
        Self::register_http_functions(&mut engine);

        Self { engine }
    }

    /// 执行请求前脚本
    pub fn execute_pre_request(
        &mut self,
        script: &str,
        context: PreRequestContext,
    ) -> Result<ScriptResult> {
        let mut console_output = Vec::new();

        // 创建作用域
        let mut scope = Scope::new();

        // 将上下文转换为 Rhai Map
        scope.push("request", Self::pre_request_to_map(&context));
        scope.push("vars", Self::hashmap_to_map(&context.variables));

        // 执行脚本
        match self.engine.eval_with_scope::<Dynamic>(&mut scope, script) {
            Ok(_) => {
                // 从 scope 中提取修改后的值
                let modified_context = Self::extract_pre_request_context(&scope, context)?;

                Ok(ScriptResult {
                    success: true,
                    error: None,
                    context: ScriptContext::PreRequest(modified_context),
                    console_output,
                })
            }
            Err(e) => Ok(ScriptResult {
                success: false,
                error: Some(e.to_string()),
                context: ScriptContext::PreRequest(context),
                console_output,
            }),
        }
    }

    /// 执行响应后脚本
    pub fn execute_post_response(
        &mut self,
        script: &str,
        context: PostResponseContext,
    ) -> Result<ScriptResult> {
        let mut console_output = Vec::new();
        let mut scope = Scope::new();

        // 注册上下文
        scope.push("request", Self::pre_request_to_map(&context.request));
        scope.push("response", Self::post_response_to_map(&context));
        scope.push("vars", Self::hashmap_to_map(&context.variables));

        // 添加测试相关的变量
        scope.push("test_passed", true);
        scope.push("test_message", "".to_string());

        // 执行脚本
        match self.engine.eval_with_scope::<Dynamic>(&mut scope, script) {
            Ok(_) => {
                let modified_context = Self::extract_post_response_context(&scope, context)?;

                Ok(ScriptResult {
                    success: true,
                    error: None,
                    context: ScriptContext::PostResponse(modified_context),
                    console_output,
                })
            }
            Err(e) => Ok(ScriptResult {
                success: false,
                error: Some(e.to_string()),
                context: ScriptContext::PostResponse(context),
                console_output,
            }),
        }
    }

    // ===== 加密函数 =====
    fn register_crypto_functions(engine: &mut Engine) {
        use sha2::{Sha256, Sha512, Digest};

        // MD5
        engine.register_fn("md5", |data: &str| -> String {
            let result = md5::compute(data.as_bytes());
            format!("{:x}", result)
        });

        // SHA256
        engine.register_fn("sha256", |data: &str| -> String {
            let result = Sha256::digest(data.as_bytes());
            hex::encode(result)
        });

        // SHA512
        engine.register_fn("sha512", |data: &str| -> String {
            let result = Sha512::digest(data.as_bytes());
            hex::encode(result)
        });

        // HMAC-SHA256
        engine.register_fn("hmac_sha256", |key: &str, data: &str| -> String {
            use hmac::{Hmac, Mac};
            type HmacSha256 = Hmac<Sha256>;

            let mut mac = HmacSha256::new_from_slice(key.as_bytes())
                .expect("HMAC can take key of any size");
            mac.update(data.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        });
    }

    // ===== 编码函数 =====
    fn register_encoding_functions(engine: &mut Engine) {
        use base64::{engine::general_purpose, Engine as _};

        // Base64 编码
        engine.register_fn("base64_encode", |data: &str| -> String {
            general_purpose::STANDARD.encode(data.as_bytes())
        });

        // Base64 解码
        engine.register_fn("base64_decode", |data: &str| -> String {
            general_purpose::STANDARD
                .decode(data)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .unwrap_or_default()
        });

        // URL 编码
        engine.register_fn("url_encode", |data: &str| -> String {
            urlencoding::encode(data).to_string()
        });

        // URL 解码
        engine.register_fn("url_decode", |data: &str| -> String {
            urlencoding::decode(data).unwrap_or_default().to_string()
        });

        // Hex 编码
        engine.register_fn("hex_encode", |data: &str| -> String {
            hex::encode(data.as_bytes())
        });

        // Hex 解码
        engine.register_fn("hex_decode", |data: &str| -> String {
            hex::decode(data)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .unwrap_or_default()
        });
    }

    // ===== JSON 函数 =====
    fn register_json_functions(engine: &mut Engine) {
        use serde_json::Value;

        // 解析 JSON 字符串为 Rhai Map
        engine.register_fn("parse_json", |json_str: &str| -> Dynamic {
            match serde_json::from_str::<Value>(json_str) {
                Ok(value) => Self::json_value_to_dynamic(&value),
                Err(_) => Dynamic::UNIT, // 解析失败返回 ()
            }
        });

        // 将对象转为 JSON 字符串
        engine.register_fn("to_json", |obj: Map| -> String {
            let json_value = Self::map_to_json_value(&obj);
            serde_json::to_string(&json_value).unwrap_or_default()
        });

        // 美化 JSON 字符串
        engine.register_fn("json_stringify", |obj: Map| -> String {
            let json_value = Self::map_to_json_value(&obj);
            serde_json::to_string_pretty(&json_value).unwrap_or_default()
        });

        // 检查 JSON 是否有效
        engine.register_fn("is_valid_json", |json_str: &str| -> bool {
            serde_json::from_str::<Value>(json_str).is_ok()
        });
    }

    // ===== Console 函数 =====
    fn register_console_functions(engine: &mut Engine) {
        // console_log for String
        engine.register_fn("console_log", |msg: &str| {
            println!("[Script] {}", msg);
        });

        // console_log for integers
        engine.register_fn("console_log", |msg: i64| {
            println!("[Script] {}", msg);
        });

        // console_log for floats
        engine.register_fn("console_log", |msg: f64| {
            println!("[Script] {}", msg);
        });

        // console_log for booleans
        engine.register_fn("console_log", |msg: bool| {
            println!("[Script] {}", msg);
        });

        // console_log for Map (转为 JSON)
        engine.register_fn("console_log", |map: Map| {
            let json_value = Self::map_to_json_value(&map);
            println!("[Script] {}", serde_json::to_string_pretty(&json_value).unwrap_or_default());
        });

        // console_log for Dynamic (通用)
        engine.register_fn("console_log", |value: Dynamic| {
            if let Ok(s) = value.clone().into_string() {
                println!("[Script] {}", s);
            } else if let Some(map) = value.clone().try_cast::<Map>() {
                let json_value = Self::map_to_json_value(&map);
                println!("[Script] {}", serde_json::to_string_pretty(&json_value).unwrap_or_default());
            } else {
                println!("[Script] {:?}", value);
            }
        });
    }

    // ===== 工具函数 =====
    fn register_utility_functions(engine: &mut Engine) {
        // 生成随机数
        engine.register_fn("random", || -> i64 {
            use rand::Rng;
            rand::thread_rng().gen_range(0..1000000)
        });

        // 生成随机字符串
        engine.register_fn("random_string", |length: i64| -> String {
            use rand::Rng;
            const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            let mut rng = rand::thread_rng();
            (0..length)
                .map(|_| {
                    let idx = rng.gen_range(0..CHARSET.len());
                    CHARSET[idx] as char
                })
                .collect()
        });

        // 获取当前时间戳(秒)
        engine.register_fn("timestamp", || -> i64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        });

        // 获取当前时间戳(毫秒)
        engine.register_fn("timestamp_ms", || -> i64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
        });

        // UUID v4
        engine.register_fn("uuid", || -> String {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            format!(
                "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
                rng.gen::<u32>(),
                rng.gen::<u16>(),
                rng.gen::<u16>() & 0x0fff,
                (rng.gen::<u16>() & 0x3fff) | 0x8000,
                rng.gen::<u64>() & 0xffffffffffff
            )
        });
    }

    // ===== 文件操作函数 =====
    fn register_file_functions(engine: &mut Engine) {
        // 读取文件内容
        engine.register_fn("read_file", |path: &str| -> String {
            std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("[Script] 读取文件失败 {}: {}", path, e);
                String::new()
            })
        });

        // 写入文件（覆盖）
        engine.register_fn("write_file", |path: &str, content: &str| -> bool {
            // 确保父目录存在
            if let Some(parent) = std::path::Path::new(path).parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("[Script] 创建目录失败 {}: {}", parent.display(), e);
                    return false;
                }
            }

            match std::fs::write(path, content) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("[Script] 写入文件失败 {}: {}", path, e);
                    false
                }
            }
        });

        // 追加到文件
        engine.register_fn("append_file", |path: &str, content: &str| -> bool {
            use std::io::Write;

            // 确保父目录存在
            if let Some(parent) = std::path::Path::new(path).parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("[Script] 创建目录失败 {}: {}", parent.display(), e);
                    return false;
                }
            }

            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                Ok(mut file) => match file.write_all(content.as_bytes()) {
                    Ok(_) => true,
                    Err(e) => {
                        eprintln!("[Script] 追加文件失败 {}: {}", path, e);
                        false
                    }
                },
                Err(e) => {
                    eprintln!("[Script] 打开文件失败 {}: {}", path, e);
                    false
                }
            }
        });

        // 检查文件是否存在
        engine.register_fn("file_exists", |path: &str| -> bool {
            std::path::Path::new(path).exists()
        });

        // 删除文件
        engine.register_fn("delete_file", |path: &str| -> bool {
            match std::fs::remove_file(path) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("[Script] 删除文件失败 {}: {}", path, e);
                    false
                }
            }
        });

        // 读取文件为字节数组（返回 base64 编码的字符串）
        engine.register_fn("read_file_bytes", |path: &str| -> String {
            use base64::{engine::general_purpose, Engine as _};

            match std::fs::read(path) {
                Ok(bytes) => general_purpose::STANDARD.encode(&bytes),
                Err(e) => {
                    eprintln!("[Script] 读取文件失败 {}: {}", path, e);
                    String::new()
                }
            }
        });

        // 写入字节数组（从 base64 编码的字符串）
        engine.register_fn("write_file_bytes", |path: &str, base64_content: &str| -> bool {
            use base64::{engine::general_purpose, Engine as _};

            // 确保父目录存在
            if let Some(parent) = std::path::Path::new(path).parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("[Script] 创建目录失败 {}: {}", parent.display(), e);
                    return false;
                }
            }

            match general_purpose::STANDARD.decode(base64_content) {
                Ok(bytes) => match std::fs::write(path, bytes) {
                    Ok(_) => true,
                    Err(e) => {
                        eprintln!("[Script] 写入文件失败 {}: {}", path, e);
                        false
                    }
                },
                Err(e) => {
                    eprintln!("[Script] Base64解码失败: {}", e);
                    false
                }
            }
        });

        // 创建目录
        engine.register_fn("create_dir", |path: &str| -> bool {
            match std::fs::create_dir_all(path) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("[Script] 创建目录失败 {}: {}", path, e);
                    false
                }
            }
        });

        // 列出目录中的文件
        engine.register_fn("list_files", |path: &str| -> Vec<Dynamic> {
            match std::fs::read_dir(path) {
                Ok(entries) => {
                    entries
                        .filter_map(|entry| {
                            entry.ok().and_then(|e| {
                                e.path().to_str().map(|s| Dynamic::from(s.to_string()))
                            })
                        })
                        .collect()
                },
                Err(e) => {
                    eprintln!("[Script] 读取目录失败 {}: {}", path, e);
                    Vec::new()
                }
            }
        });
    }

    // ===== HTTP 网络请求函数 =====
    fn register_http_functions(engine: &mut Engine) {
        // HTTP GET 请求（文本）
        engine.register_fn("http_get", |url: &str| -> String {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                match reqwest::get(url).await {
                    Ok(response) => {
                        match response.text().await {
                            Ok(text) => text,
                            Err(e) => {
                                eprintln!("[Script] 读取响应失败: {}", e);
                                String::new()
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("[Script] HTTP GET 请求失败 {}: {}", url, e);
                        String::new()
                    }
                }
            })
        });

        // HTTP GET 请求（二进制，返回 Base64）
        engine.register_fn("http_get_bytes", |url: &str| -> String {
            use base64::{engine::general_purpose, Engine as _};

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                match reqwest::get(url).await {
                    Ok(response) => {
                        match response.bytes().await {
                            Ok(bytes) => general_purpose::STANDARD.encode(&bytes),
                            Err(e) => {
                                eprintln!("[Script] 读取响应失败: {}", e);
                                String::new()
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("[Script] HTTP GET 请求失败 {}: {}", url, e);
                        String::new()
                    }
                }
            })
        });

        // HTTP POST 请求（带 JSON body）
        engine.register_fn("http_post", |url: &str, body: &str| -> String {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = reqwest::Client::new();
                match client.post(url)
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                    .send()
                    .await
                {
                    Ok(response) => {
                        match response.text().await {
                            Ok(text) => text,
                            Err(e) => {
                                eprintln!("[Script] 读取响应失败: {}", e);
                                String::new()
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("[Script] HTTP POST 请求失败 {}: {}", url, e);
                        String::new()
                    }
                }
            })
        });

        // HTTP 请求（完整版，返回响应对象）
        engine.register_fn("http_request", |url: &str, method: &str, body: &str, headers: Map| -> Map {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = reqwest::Client::new();

                // 构建请求
                let mut request_builder = match method.to_uppercase().as_str() {
                    "GET" => client.get(url),
                    "POST" => client.post(url),
                    "PUT" => client.put(url),
                    "DELETE" => client.delete(url),
                    "PATCH" => client.patch(url),
                    _ => client.get(url),
                };

                // 添加请求头
                for (key, value) in headers.iter() {
                    if let Ok(value_str) = value.clone().into_string() {
                        request_builder = request_builder.header(key.to_string(), value_str);
                    }
                }

                // 添加请求体
                if !body.is_empty() && method.to_uppercase() != "GET" {
                    request_builder = request_builder.body(body.to_string());
                }

                // 发送请求
                match request_builder.send().await {
                    Ok(response) => {
                        let status = response.status().as_u16() as i64;
                        let headers_map = {
                            let mut h = Map::new();
                            for (name, value) in response.headers() {
                                if let Ok(v) = value.to_str() {
                                    h.insert(name.as_str().to_string().into(), Dynamic::from(v.to_string()));
                                }
                            }
                            h
                        };

                        let body_text = response.text().await.unwrap_or_default();

                        let mut result = Map::new();
                        result.insert("status".into(), Dynamic::from(status));
                        result.insert("headers".into(), Dynamic::from(headers_map));
                        result.insert("body".into(), Dynamic::from(body_text));
                        result
                    },
                    Err(e) => {
                        eprintln!("[Script] HTTP 请求失败 {}: {}", url, e);
                        let mut result = Map::new();
                        result.insert("status".into(), Dynamic::from(0_i64));
                        result.insert("headers".into(), Dynamic::from(Map::new()));
                        result.insert("body".into(), Dynamic::from(String::new()));
                        result.insert("error".into(), Dynamic::from(e.to_string()));
                        result
                    }
                }
            })
        });

        // 简化的 HTTP 请求（仅 URL 和 method）
        engine.register_fn("http_request", |url: &str, method: &str| -> Map {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = reqwest::Client::new();

                let request_builder = match method.to_uppercase().as_str() {
                    "GET" => client.get(url),
                    "POST" => client.post(url),
                    "PUT" => client.put(url),
                    "DELETE" => client.delete(url),
                    "PATCH" => client.patch(url),
                    _ => client.get(url),
                };

                match request_builder.send().await {
                    Ok(response) => {
                        let status = response.status().as_u16() as i64;
                        let body_text = response.text().await.unwrap_or_default();

                        let mut result = Map::new();
                        result.insert("status".into(), Dynamic::from(status));
                        result.insert("body".into(), Dynamic::from(body_text));
                        result
                    },
                    Err(e) => {
                        eprintln!("[Script] HTTP 请求失败 {}: {}", url, e);
                        let mut result = Map::new();
                        result.insert("status".into(), Dynamic::from(0_i64));
                        result.insert("body".into(), Dynamic::from(String::new()));
                        result.insert("error".into(), Dynamic::from(e.to_string()));
                        result
                    }
                }
            })
        });
    }

    // ===== 辅助转换函数 =====
    fn pre_request_to_map(context: &PreRequestContext) -> Map {
        let mut map = Map::new();
        map.insert("url".into(), Dynamic::from(context.url.clone()));
        map.insert("method".into(), Dynamic::from(context.method.clone()));
        map.insert("headers".into(), Self::hashmap_to_map(&context.headers));
        map.insert("params".into(), Self::hashmap_to_map(&context.params));
        map.insert("body".into(), Dynamic::from(context.body.clone()));
        map
    }

    fn post_response_to_map(context: &PostResponseContext) -> Map {
        let mut map = Map::new();
        map.insert("status".into(), Dynamic::from(context.status as i64));
        map.insert("headers".into(), Self::hashmap_to_map(&context.headers));
        map.insert("body".into(), Dynamic::from(context.body.clone()));
        map.insert("duration".into(), Dynamic::from(context.duration as i64));
        map
    }

    fn hashmap_to_map(hashmap: &HashMap<String, String>) -> Dynamic {
        let mut map = Map::new();
        for (k, v) in hashmap {
            map.insert(k.clone().into(), Dynamic::from(v.clone()));
        }
        Dynamic::from(map)
    }

    fn extract_pre_request_context(
        scope: &Scope,
        mut context: PreRequestContext,
    ) -> Result<PreRequestContext> {
        // 提取修改后的 request 对象
        if let Some(request) = scope.get_value::<Map>("request") {
            if let Some(url) = request.get("url") {
                context.url = url.clone().into_string().unwrap_or(context.url);
            }
            if let Some(method) = request.get("method") {
                context.method = method.clone().into_string().unwrap_or(context.method);
            }
            if let Some(body) = request.get("body") {
                context.body = body.clone().into_string().unwrap_or(context.body);
            }
            if let Some(headers) = request.get("headers").and_then(|h| h.clone().try_cast::<Map>()) {
                context.headers = Self::map_to_hashmap(&headers);
            }
            if let Some(params) = request.get("params").and_then(|p| p.clone().try_cast::<Map>()) {
                context.params = Self::map_to_hashmap(&params);
            }
        }

        // 提取修改后的变量
        if let Some(vars) = scope.get_value::<Map>("vars") {
            context.variables = Self::map_to_hashmap(&vars);
        }

        Ok(context)
    }

    fn extract_post_response_context(
        scope: &Scope,
        mut context: PostResponseContext,
    ) -> Result<PostResponseContext> {
        // 提取修改后的变量
        if let Some(vars) = scope.get_value::<Map>("vars") {
            context.variables = Self::map_to_hashmap(&vars);
        }

        Ok(context)
    }

    fn map_to_hashmap(map: &Map) -> HashMap<String, String> {
        map.iter()
            .filter_map(|(k, v)| {
                Some((
                    k.to_string(),
                    v.clone().into_string().ok()?,
                ))
            })
            .collect()
    }

    // JSON Value 转 Rhai Dynamic
    fn json_value_to_dynamic(value: &serde_json::Value) -> Dynamic {
        use serde_json::Value;

        match value {
            Value::Null => Dynamic::UNIT,
            Value::Bool(b) => Dynamic::from(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Dynamic::from(i)
                } else if let Some(f) = n.as_f64() {
                    Dynamic::from(f)
                } else {
                    Dynamic::UNIT
                }
            }
            Value::String(s) => Dynamic::from(s.clone()),
            Value::Array(arr) => {
                let rhai_arr: Vec<Dynamic> = arr.iter().map(Self::json_value_to_dynamic).collect();
                Dynamic::from(rhai_arr)
            }
            Value::Object(obj) => {
                let mut map = Map::new();
                for (k, v) in obj {
                    map.insert(k.clone().into(), Self::json_value_to_dynamic(v));
                }
                Dynamic::from(map)
            }
        }
    }

    // Rhai Map 转 JSON Value
    fn map_to_json_value(map: &Map) -> serde_json::Value {
        use serde_json::{json, Value};

        let mut obj = serde_json::Map::new();
        for (k, v) in map {
            let json_val = Self::dynamic_to_json_value(v);
            obj.insert(k.to_string(), json_val);
        }
        Value::Object(obj)
    }

    // Rhai Dynamic 转 JSON Value
    fn dynamic_to_json_value(dynamic: &Dynamic) -> serde_json::Value {
        use serde_json::{json, Value};

        if dynamic.is_unit() {
            Value::Null
        } else if let Some(b) = dynamic.as_bool().ok() {
            Value::Bool(b)
        } else if let Some(i) = dynamic.as_int().ok() {
            json!(i)
        } else if let Some(f) = dynamic.as_float().ok() {
            json!(f)
        } else if let Some(s) = dynamic.clone().into_string().ok() {
            Value::String(s)
        } else if let Some(arr) = dynamic.clone().try_cast::<Vec<Dynamic>>() {
            let json_arr: Vec<Value> = arr.iter().map(Self::dynamic_to_json_value).collect();
            Value::Array(json_arr)
        } else if let Some(map) = dynamic.clone().try_cast::<Map>() {
            Self::map_to_json_value(&map)
        } else {
            Value::Null
        }
    }
}

// 添加 urlencoding 需要的依赖（如果没有的话，我们手动实现）
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                    c.to_string()
                } else {
                    format!("%{:02X}", c as u8)
                }
            })
            .collect()
    }

    pub fn decode(s: &str) -> Result<std::borrow::Cow<'static, str>, ()> {
        let mut result = String::new();
        let mut chars = s.chars();

        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push(c);
                    result.push_str(&hex);
                }
            } else if c == '+' {
                result.push(' ');
            } else {
                result.push(c);
            }
        }

        Ok(std::borrow::Cow::Owned(result))
    }
}
