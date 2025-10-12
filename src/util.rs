#![allow(warnings, unused)]

use std::{ffi::OsStr, path::Path};

use crate::{HttpRequestConfig, HttpResponse};
use anyhow::{bail, Result};
use eframe::egui;
use image::GenericImageView;

use lazy_static::lazy_static;
use regex::Regex;

use crate::{AppConfig, PairUi, Project};
use crate::script_engine::{ScriptEngine, PreRequestContext, PostResponseContext, ScriptContext};

pub fn load_app_icon() -> eframe::egui::IconData {
    let app_icon_bytes = include_bytes!("../data/icon.jpg");
    let app_icon = image::load_from_memory(app_icon_bytes).expect("load icon error");
    let (app_icon_width, app_icon_height) = app_icon.dimensions();

    eframe::egui::IconData {
        rgba: app_icon.into_rgba8().into_vec(),
        width: app_icon_width,
        height: app_icon_height,
    }
}

pub fn setup_custom_fonts(ctx: &egui::Context) {
    // 从默认字体开始（我们将添加而不是替换它们）
    let mut fonts = egui::FontDefinitions::default();

    // load system font
    let Ok(font) = std::fs::read(
        // r#"c:/Windows/Fonts/consola.ttf"#
        r#"c:/Windows/Fonts/msyhl.ttc"#,
    ) else {
        panic!("font not find");
    };

    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_owned(font).into(),
    );

    // 安装我的字体
    // fonts.font_data.insert(
    //     "my_font".to_owned(),
    //     egui::FontData::from_owned(include_bytes!(
    //         "../font/YeZiGongChangChuanQiuShaXingKai-2.ttf"
    //     )),
    // );

    // 对于比例文本，将我的字体放在第一位（最高优先级）
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("my_font".to_owned());

    // 告诉 egui 使用这些字体
    ctx.set_fonts(fonts);
}

pub fn get_filename<S: AsRef<OsStr> + ?Sized>(path: &S) -> Result<String> {
    Ok(std::path::Path::new(path)
        .file_stem()
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

pub fn save_project(dir: &str, project: &Project) -> Result<()> {
    if project.name.is_empty() {
        bail!("项目名称不能为空")
    };

    let data = serde_json::to_vec(project)?;
    let save_path = Path::new(dir).join(format!("{}.json", &project.name));
    std::fs::write(&save_path, data)?;

    // 在保存 .config
    let config_content = serde_json::to_vec(&AppConfig {
        project_path: save_path.to_str().unwrap().to_string(),
    })?;

    std::fs::write(Path::new(dir).join("./.config.json"), config_content)?;

    Ok(())
}

pub async fn http_send(req_cfg: &HttpRequestConfig, vars: &Vec<PairUi>) -> Result<HttpResponse> {
    let mut request_size = 0u64;
    request_size += req_cfg.url.len() as u64;
    request_size += req_cfg.body_raw.len() as u64;
    for kv in &req_cfg.header {
        if !kv.disable {
            request_size += kv.key.len() as u64 + kv.value.len() as u64;
        }
    }

    // 创建可变的请求配置副本用于脚本修改
    let mut modified_req_cfg = req_cfg.clone();
    let mut script_vars = vars.clone();

    // 执行 Pre-Request Script
    if req_cfg.script_enabled && !req_cfg.pre_request_script.trim().is_empty() {
        let mut engine = ScriptEngine::new();

        let context = PreRequestContext {
            url: modified_req_cfg.url.clone(),
            method: modified_req_cfg.method.as_ref().to_string(),
            headers: modified_req_cfg.header.iter()
                .filter(|kv| !kv.disable)
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect(),
            params: modified_req_cfg.query.iter()
                .filter(|kv| !kv.disable)
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect(),
            body: modified_req_cfg.body_raw.clone(),
            variables: script_vars.iter()
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect(),
        };

        match engine.execute_pre_request(&req_cfg.pre_request_script, context) {
            Ok(result) => {
                if result.success {
                    // 应用脚本修改
                    if let ScriptContext::PreRequest(ctx) = result.context {
                        modified_req_cfg.url = ctx.url;
                        modified_req_cfg.body_raw = ctx.body;

                        // 更新 headers
                        for (key, value) in ctx.headers {
                            if let Some(existing) = modified_req_cfg.header.iter_mut().find(|h| h.key == key) {
                                existing.value = value;
                            } else {
                                modified_req_cfg.header.push(PairUi {
                                    key,
                                    value,
                                    disable: false,
                                });
                            }
                        }

                        // 更新 params (查询参数)
                        for (key, value) in ctx.params {
                            if let Some(existing) = modified_req_cfg.query.iter_mut().find(|p| p.key == key) {
                                existing.value = value;
                            } else {
                                modified_req_cfg.query.push(PairUi {
                                    key,
                                    value,
                                    disable: false,
                                });
                            }
                        }

                        // 更新变量
                        for (key, value) in ctx.variables {
                            if let Some(existing) = script_vars.iter_mut().find(|v| v.key == key) {
                                existing.value = value;
                            } else {
                                script_vars.push(PairUi {
                                    key,
                                    value,
                                    disable: false,
                                });
                            }
                        }
                    }
                } else if let Some(err) = result.error {
                    eprintln!("Pre-request script error: {}", err);
                }
            }
            Err(e) => {
                eprintln!("Pre-request script execution error: {}", e);
            }
        }
    }

    let request_builder = modified_req_cfg.request_build(&script_vars).await?;
    let start_time = std::time::Instant::now();
    let response = request_builder.send().await?;
    let duration = start_time.elapsed().as_millis();
    let status = response.status();
    let version = response.version();
    let headers = response.headers().to_owned();
    let data_vec = response.bytes().await.and_then(|bs| Ok(bs.to_vec())).ok();

    let response_size = data_vec.as_ref().map(|v| v.len() as u64).unwrap_or(0);

    let mut headers_str = String::new();
    headers.iter().for_each(|(name, val)| {
        let name = name.as_str();
        let value = val.to_str().unwrap_or("");
        headers_str.push_str(format!("{}: {}\n", name, value).as_str());
    });

    let response_body = data_vec.as_ref()
        .and_then(|d| String::from_utf8(d.clone()).ok())
        .unwrap_or_default();

    // 执行 Post-Response Script
    if req_cfg.script_enabled && !req_cfg.post_response_script.trim().is_empty() {
        let mut engine = ScriptEngine::new();

        let request_context = PreRequestContext {
            url: modified_req_cfg.url.clone(),
            method: modified_req_cfg.method.as_ref().to_string(),
            headers: modified_req_cfg.header.iter()
                .filter(|kv| !kv.disable)
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect(),
            params: modified_req_cfg.query.iter()
                .filter(|kv| !kv.disable)
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect(),
            body: modified_req_cfg.body_raw.clone(),
            variables: script_vars.iter()
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect(),
        };

        let context = PostResponseContext {
            request: request_context,
            status: status.as_u16(),
            headers: headers.iter()
                .map(|(name, val)| (name.as_str().to_string(), val.to_str().unwrap_or("").to_string()))
                .collect(),
            body: response_body.clone(),
            duration,
            variables: script_vars.iter()
                .map(|kv| (kv.key.clone(), kv.value.clone()))
                .collect(),
        };

        match engine.execute_post_response(&req_cfg.post_response_script, context) {
            Ok(result) => {
                if result.success {
                    // 应用变量修改（post-response 主要用于修改变量）
                    if let ScriptContext::PostResponse(ctx) = result.context {
                        for (key, value) in ctx.variables {
                            if let Some(existing) = script_vars.iter_mut().find(|v| v.key == key) {
                                existing.value = value;
                            } else {
                                script_vars.push(PairUi {
                                    key,
                                    value,
                                    disable: false,
                                });
                            }
                        }
                    }
                } else if let Some(err) = result.error {
                    eprintln!("Post-response script error: {}", err);
                }
            }
            Err(e) => {
                eprintln!("Post-response script execution error: {}", e);
            }
        }
    }

    // 检查变量是否被脚本修改过
    let modified_vars = if req_cfg.script_enabled &&
                          (!req_cfg.pre_request_script.trim().is_empty() ||
                           !req_cfg.post_response_script.trim().is_empty()) {
        Some(script_vars)
    } else {
        None
    };

    Ok(HttpResponse {
        data_vec,
        headers,
        version,
        status,
        img: None,
        text: None,
        headers_str,
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
