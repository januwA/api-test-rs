#![allow(warnings, unused)]

use std::sync::{Arc, Mutex};
use std::{ffi::OsStr, path::Path};

use crate::{HttpRequestConfig, HttpResponse};
use anyhow::{bail, Result};

use lazy_static::lazy_static;
use regex::Regex;

use crate::script_engine::{PostResponseContext, PreRequestContext, ScriptContext, ScriptEngine};
use crate::{AppConfig, PairUi, Project};

pub fn get_filename<S: AsRef<OsStr> + ?Sized>(path: &S) -> Result<String> {
    Ok(std::path::Path::new(path)
        .file_name() // 改用 file_name() 以保留扩展名
        .ok_or_else(|| "获取文件名失败")
        .map_err(anyhow::Error::msg)?
        .to_str()
        .ok_or_else(|| "转换文件名失败")
        .map_err(anyhow::Error::msg)?
        .to_owned())
}

/**
 * 从文件地址加载项目
 */
pub fn load_project(project_path: &str) -> Result<Project> {
    if project_path.is_empty() {
        bail!("加载路径不能为空")
    }

    let load_path = Path::new(project_path);
    if !load_path.exists() {
        bail!("文件不存在")
    }
    let data = std::fs::read(&load_path)?;
    let dat: Project = serde_json::from_slice(data.as_slice())?;

    Ok(dat)
}

/**
 * 将一块数据下载到本地
 */
pub fn download(request_url: &str, download_path: &str, data: &[u8]) -> Result<()> {
    if download_path.is_empty() {
        bail!("下载路径不能为空");
    }

    let path_obj = Path::new(download_path);
    let final_path = if path_obj.file_name().is_some() {
        // If download_path itself contains a filename, use it directly.
        path_obj.to_path_buf()
    } else {
        // If download_path is a directory, try to get a filename from request_url.
        let filename = Path::new(request_url)
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("无法从请求URL确定文件名"))?; // Use anyhow for consistent error type
        path_obj.join(filename)
    };

    // Ensure the parent directory exists
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&final_path, data)
        .map_err(|e| anyhow::anyhow!("写入文件失败: {} -> {}", final_path.display(), e))?;

    Ok(())
}

/**
 * 从网络或则本地读取数据
 */
pub async fn read_binary(path: &str) -> Result<Vec<u8>> {
    if path.is_empty() {
        bail!("路径不能为空")
    }

    Ok(if path.starts_with("http") {
        let res = reqwest::get(path).await?;
        let dat = res.bytes().await?;
        dat.to_vec()
    } else {
        let p = Path::new(path);
        if !p.exists() {
            bail!("file not exists")
        }
        tokio::fs::read(p).await?
    })
}

pub async fn handle_multipart(kv_vec: &Vec<(String, String)>) -> Result<reqwest::multipart::Form> {
    use reqwest::multipart::{Form, Part};

    let mut form = Form::new();
    // name : bar
    // file : @a.jpg
    // files: @a.jpg @b.jpg
    for (k, v) in kv_vec {
        if !v.is_empty() && v.contains('@') {
            let filepaths: Vec<_> = v
                .split('@')
                .filter(|e| !e.is_empty())
                .map(|e| e.trim())
                .collect();
            for filepath in filepaths {
                let file_body = read_binary(filepath).await?;

                form = form.part(
                    k.to_owned(),
                    Part::bytes(file_body).file_name(get_filename(filepath)?),
                );
            }
        } else {
            form = form.text(k.to_owned(), v.to_owned());
        }
    }
    Ok(form)
}

pub fn tuple_vec(vec: &Vec<PairUi>) -> Vec<(&str, &str)> {
    vec.into_iter().filter_map(|el| el.tuple()).collect()
}

// 使用变量填充字符串
pub fn real_tuple_vec(vec: &Vec<PairUi>, vars: &Vec<PairUi>) -> Vec<(String, String)> {
    tuple_vec(vec)
        .iter()
        .map(|x| real_tuple_fn(x, vars))
        .collect()
}

pub fn save_project(dir: &str, project: &Project, font_size: f32) -> Result<()> {
    if project.name.is_empty() {
        bail!("项目名称不能为空")
    };

    let data = serde_json::to_vec(project)?;
    let save_path = Path::new(dir).join(format!("{}.json", &project.name));
    std::fs::write(&save_path, data)?;

    // 在保存 .config
    let config_content = serde_json::to_vec(&AppConfig {
        project_path: save_path.to_str().unwrap().to_string(),
        font_size,
    })?;

    std::fs::write(Path::new(dir).join("./.config.json"), config_content)?;

    Ok(())
}

/// 执行请求前的脚本处理
async fn execute_pre_request_script(
    req_cfg: &HttpRequestConfig,
    script_output: Arc<Mutex<Vec<String>>>,
    modified_req_cfg: &mut HttpRequestConfig,
    script_vars: &mut Vec<PairUi>,
) {
    if req_cfg.pre_request_script.trim().is_empty() {
        return;
    }

    let mut engine = ScriptEngine::new(script_output.clone());

    let context = PreRequestContext {
        url: modified_req_cfg.url.clone(),
        method: modified_req_cfg.method.as_ref().to_string(),
        headers: modified_req_cfg
            .header
            .iter()
            .filter(|kv| !kv.disable)
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect(),
        params: modified_req_cfg
            .query
            .iter()
            .filter(|kv| !kv.disable)
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect(),
        body: modified_req_cfg.body_raw.clone(),
        variables: script_vars
            .iter()
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect(),
    };

    match engine.execute_pre_request(&req_cfg.pre_request_script, context) {
        Ok(result) => {
            if result.success {
                if let ScriptContext::PreRequest(ctx) = result.context {
                    apply_pre_request_script_changes(modified_req_cfg, script_vars, ctx);
                }
            } else if let Some(err) = result.error {
                log_script_error(&script_output, &format!("Pre-request script error: {}", err));
            }
        }
        Err(e) => {
            log_script_error(&script_output, &format!("Pre-request script execution error: {}", e));
        }
    }
}

/// 应用预请求脚本的修改
fn apply_pre_request_script_changes(
    req_cfg: &mut HttpRequestConfig,
    script_vars: &mut Vec<PairUi>,
    ctx: PreRequestContext,
) {
    req_cfg.url = ctx.url;
    req_cfg.body_raw = ctx.body;

    // 更新 headers
    for (key, value) in ctx.headers {
        update_or_add_pair(&mut req_cfg.header, &key, &value);
    }

    // 更新 params (查询参数)
    for (key, value) in ctx.params {
        update_or_add_pair(&mut req_cfg.query, &key, &value);
    }

    // 更新变量
    for (key, value) in ctx.variables {
        update_or_add_pair(script_vars, &key, &value);
    }
}

/// 执行响应后的脚本处理
async fn execute_post_response_script(
    req_cfg: &HttpRequestConfig,
    script_output: Arc<Mutex<Vec<String>>>,
    modified_req_cfg: &HttpRequestConfig,
    script_vars: &mut Vec<PairUi>,
    status: u16,
    headers: &reqwest::header::HeaderMap,
    response_body: &str,
    duration: u128,
) {
    if req_cfg.post_response_script.trim().is_empty() {
        return;
    }

    let mut engine = ScriptEngine::new(script_output.clone());

    let request_context = PreRequestContext {
        url: modified_req_cfg.url.clone(),
        method: modified_req_cfg.method.as_ref().to_string(),
        headers: modified_req_cfg
            .header
            .iter()
            .filter(|kv| !kv.disable)
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect(),
        params: modified_req_cfg
            .query
            .iter()
            .filter(|kv| !kv.disable)
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect(),
        body: modified_req_cfg.body_raw.clone(),
        variables: script_vars
            .iter()
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect(),
    };

    let context = PostResponseContext {
        request: request_context,
        status,
        headers: headers
            .iter()
            .map(|(name, val)| (
                name.as_str().to_string(),
                val.to_str().unwrap_or("").to_string(),
            ))
            .collect(),
        body: response_body.to_string(),
        duration,
        variables: script_vars
            .iter()
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect(),
    };

    match engine.execute_post_response(&req_cfg.post_response_script, context) {
        Ok(result) => {
            if result.success {
                if let ScriptContext::PostResponse(ctx) = result.context {
                    // 应用变量修改
                    for (key, value) in ctx.variables {
                        update_or_add_pair(script_vars, &key, &value);
                    }
                }
            } else if let Some(err) = result.error {
                log_script_error(&script_output, &format!("Post-response script error: {}", err));
            }
        }
        Err(e) => {
            log_script_error(&script_output, &format!("Post-response script execution error: {}", e));
        }
    }
}

/// 计算请求大小
fn calculate_request_size(req_cfg: &HttpRequestConfig) -> u64 {
    let mut size = req_cfg.url.len() as u64 + req_cfg.body_raw.len() as u64;
    for kv in &req_cfg.header {
        if !kv.disable {
            size += kv.key.len() as u64 + kv.value.len() as u64;
        }
    }
    size
}

/// 构建请求头字符串
fn build_request_headers_string(req_cfg: &HttpRequestConfig) -> String {
    let mut headers_str = String::new();
    req_cfg.header.iter()
        .filter(|h| !h.disable)
        .for_each(|h| {
            headers_str.push_str(&format!("{}: {}\n", h.key, h.value));
        });
    headers_str
}

/// 构建响应头字符串
fn build_response_headers_string(headers: &reqwest::header::HeaderMap) -> String {
    let mut headers_str = String::new();
    headers.iter().for_each(|(name, val)| {
        let name = name.as_str();
        let value = val.to_str().unwrap_or("");
        headers_str.push_str(&format!("{}: {}\n", name, value));
    });
    headers_str
}

/// 记录脚本错误
fn log_script_error(script_output: &Arc<Mutex<Vec<String>>>, error: &str) {
    if let Ok(mut output) = script_output.lock() {
        output.push(error.to_string());
    }
}

/// 更新或添加键值对
fn update_or_add_pair(pairs: &mut Vec<PairUi>, key: &str, value: &str) {
    if let Some(existing) = pairs.iter_mut().find(|p| p.key == key) {
        existing.value = value.to_string();
    } else {
        pairs.push(PairUi {
            key: key.to_string(),
            value: value.to_string(),
            disable: false,
        });
    }
}

pub async fn http_send(
    req_cfg: &HttpRequestConfig,
    vars: &Vec<PairUi>,
    script_output: Arc<Mutex<Vec<String>>>,
) -> Result<HttpResponse> {
    let request_size = calculate_request_size(req_cfg);

    // 创建可变的请求配置副本用于脚本修改
    let mut modified_req_cfg = req_cfg.clone();
    let mut script_vars = vars.clone();

    // 执行 Pre-Request Script
    execute_pre_request_script(
        req_cfg,
        script_output.clone(),
        &mut modified_req_cfg,
        &mut script_vars,
    ).await;

    let request_headers_str = build_request_headers_string(&modified_req_cfg);
    let request_builder = modified_req_cfg.request_build(&script_vars).await?;

    let start_time = std::time::Instant::now();
    let response = request_builder.send().await?;
    let duration = start_time.elapsed().as_millis();
    let status = response.status();
    let version = response.version();
    let headers = response.headers().to_owned();
    let data_vec = response.bytes().await.and_then(|bs| Ok(bs.to_vec())).ok();

    let response_size = data_vec.as_ref().map(|v| v.len() as u64).unwrap_or(0);
    let headers_str = build_response_headers_string(&headers);

    let response_body = data_vec
        .as_ref()
        .and_then(|d| String::from_utf8(d.clone()).ok())
        .unwrap_or_default();

    // 执行 Post-Response Script
    execute_post_response_script(
        req_cfg,
        script_output,
        &modified_req_cfg,
        &mut script_vars,
        status.as_u16(),
        &headers,
        &response_body,
        duration,
    ).await;

    // 检查变量是否被脚本修改过
    let modified_vars = if !req_cfg.pre_request_script.trim().is_empty()
        || !req_cfg.post_response_script.trim().is_empty()
    {
        Some(script_vars)
    } else {
        None
    };

    Ok(HttpResponse {
        data_vec,
        headers,
        version,
        status: status.as_u16(),
        img: None,
        text: None,
        headers_str,
        request_headers_str,
        duration,
        request_size,
        response_size,
        modified_vars,
    })
}

pub fn parse_var_str(oragin_str: &str, vars: &Vec<PairUi>) -> String {
    lazy_static! {
        // {var}}       to var
        // {{ var }}    to var
        // {{{var}}}    to {var}
        static ref EXP1: Regex = Regex::new(r"\{\{([^\{\}]*)\}\}").unwrap();
    }

    let r2 = EXP1
        .replace_all(oragin_str, |cap: &regex::Captures| {
            let from = &cap[0];
            let var_name = &cap[1].trim();

            match vars.iter().find(|e| e.key.eq(var_name)) {
                Some(res) => cap[0].replace(from, &res.value),
                None => from.to_owned(),
            }
        })
        .to_string();
    r2
}

pub fn real_tuple_fn((k, v): &(&str, &str), vars: &Vec<PairUi>) -> (String, String) {
    (parse_var_str(k, vars), parse_var_str(v, vars))
}

/// URL 编码辅助函数
fn url_encode(s: &str) -> String {
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

/// 将请求配置转换为 curl 命令
pub fn to_curl_command(req_cfg: &HttpRequestConfig, vars: &Vec<PairUi>) -> String {
    let mut curl_parts = vec!["curl".to_string()];

    // 处理变量替换
    let real_url = parse_var_str(&req_cfg.url, vars);
    let request_query = real_tuple_vec(&req_cfg.query, vars);
    let request_header = real_tuple_vec(&req_cfg.header, vars);

    // 构建完整的 URL（包括查询参数）
    let mut full_url = real_url.clone();
    if !request_query.is_empty() {
        let query_string: Vec<String> = request_query
            .iter()
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect();
        full_url = format!("{}?{}", full_url, query_string.join("&"));
    }

    // 添加 URL
    curl_parts.push(format!("'{}'", full_url));

    // 添加方法
    if req_cfg.method.as_ref() != "GET" {
        curl_parts.push("-X".to_string());
        curl_parts.push(req_cfg.method.as_ref().to_string());
    }

    // 添加 Headers
    for (key, value) in request_header {
        curl_parts.push("-H".to_string());
        curl_parts.push(format!("'{}: {}'", key, value));
    }

    // 添加 Body
    match req_cfg.body_tab_ui {
        crate::RequestBodyTab::Raw => {
            if !req_cfg.body_raw.is_empty() {
                let real_body = parse_var_str(&req_cfg.body_raw, vars);
                curl_parts.push("-d".to_string());
                // 转义单引号并包装在单引号中
                let escaped_body = real_body.replace("'", "'\\''");
                curl_parts.push(format!("'{}'", escaped_body));
            }
        }
        crate::RequestBodyTab::Form => {
            let request_body_form = real_tuple_vec(&req_cfg.body_form, vars);
            for (key, value) in request_body_form {
                curl_parts.push("--data-urlencode".to_string());
                curl_parts.push(format!("'{}={}'", key, value));
            }
        }
        crate::RequestBodyTab::FormData => {
            let request_body_form_data = real_tuple_vec(&req_cfg.body_form_data, vars);
            for (key, value) in request_body_form_data {
                if !value.is_empty() && value.contains('@') {
                    // 文件字段
                    let filepaths: Vec<_> = value
                        .split('@')
                        .filter(|e| !e.is_empty())
                        .map(|e| e.trim())
                        .collect();
                    for filepath in filepaths {
                        curl_parts.push("-F".to_string());
                        curl_parts.push(format!("'{}=@{}'", key, filepath));
                    }
                } else {
                    // 普通字段
                    curl_parts.push("-F".to_string());
                    curl_parts.push(format!("'{}={}'", key, value));
                }
            }
        }
    }

    curl_parts.join(" \\\n  ")
}
