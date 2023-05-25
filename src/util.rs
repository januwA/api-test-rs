#![allow(warnings, unused)]

use std::ffi::OsStr;

use crate::{HttpConfig, HttpRequestConfig, HttpResponseUi, Promise};
use anyhow::{anyhow, bail, Result};
use eframe::egui;
use image::GenericImageView;
use reqwest::RequestBuilder;
use serde_json::json;
use tokio::runtime::Runtime;

use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

use crate::{AppConfig, Group, PairUi, Project};

pub fn load_app_icon() -> eframe::IconData {
    let app_icon_bytes = include_bytes!("../data/icon.jpg");
    let app_icon = image::load_from_memory(app_icon_bytes).expect("load icon error");
    let (app_icon_width, app_icon_height) = app_icon.dimensions();

    eframe::IconData {
        rgba: app_icon.into_rgba8().into_vec(),
        width: app_icon_width,
        height: app_icon_height,
    }
}

pub fn setup_custom_fonts(ctx: &egui::Context) {
    // 从默认字体开始（我们将添加而不是替换它们）
    let mut fonts = egui::FontDefinitions::default();

    // load system font
    let Ok(font) = std::fs::read("c:/Windows/Fonts/msyh.ttc") else {
      panic!("font not find");
  };

    fonts
        .font_data
        .insert("my_font".to_owned(), egui::FontData::from_owned(font));

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

    let load_path = std::path::Path::new(project_path);
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
pub fn download(download_path: &str, data: &Vec<u8>) -> Result<()> {
    if download_path.is_empty() {
        bail!("加载路径不能为空")
    }

    let p: &std::path::Path = std::path::Path::new(&download_path);
    let p_dir = p.parent();

    let Some(p_dir) = p_dir else {
        bail!("下载目录错误")
    };

    if !p_dir.is_dir() || !p_dir.exists() {
        bail!("下载目录不存在")
    }

    if let Err(err) = std::fs::write(p, data.as_slice()) {
        bail!(err)
    }

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
        let p = std::path::Path::new(path);
        if !p.exists() {
            bail!("file not exists")
        }
        tokio::fs::read(p).await?
    })
}

pub async fn handle_multipart(kv_vec: Vec<(String, String)>) -> Result<reqwest::multipart::Form> {
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

pub fn part_vec(vec: Vec<PairUi>) -> Vec<(String, String)> {
    vec.into_iter().filter_map(|el| el.pair()).collect()
}

pub fn real_part_vec(vec: Vec<PairUi>, vars: &Vec<PairUi>) -> Vec<(String, String)> {
    part_vec(vec)
        .iter()
        .map(|x| real_pair_fn(x, vars))
        .collect()
}

pub fn save_project(dir: &str, project: &Project) -> Result<()> {
    if project.name.is_empty() {
        bail!("项目名称不能为空")
    };

    let data = serde_json::to_vec(project)?;
    let save_path = std::path::Path::new(dir).join(format!("{}.json", &project.name));
    std::fs::write(&save_path, data)?;

    // 在保存 .config
    let config_content = serde_json::to_vec(&AppConfig {
        project_path: save_path.to_str().unwrap().to_string(),
    })?;

    std::fs::write(
        std::path::Path::new(dir).join("./.config.json"),
        config_content,
    )?;

    Ok(())
}

pub async fn http_send(req_cfg: HttpRequestConfig, vars: &Vec<PairUi>) -> Result<HttpResponseUi> {
    let request_builder = req_cfg.request_build(vars).await?;

    let response: reqwest::Response = request_builder.send().await?;

    let status = response.status();
    let version = response.version();
    let headers = response.headers().to_owned();
    let data_vec = response.bytes().await.and_then(|bs| Ok(bs.to_vec())).ok();

    Ok(HttpResponseUi {
        data_vec,
        headers,
        version,
        status,
        img: None,
        data: None,
    })
}

pub fn http_send_promise(
    rt: &Runtime,
    req_cfg: HttpRequestConfig,
    vars: Vec<PairUi>,
) -> Promise<Result<HttpResponseUi>> {
    let (tx, p) = Promise::new();

    rt.spawn(async move {
        match http_send(req_cfg, &vars).await {
            Ok(data) => {
                if let Err(_) = tx.send(Ok(data)) {
                    println!("send err");
                }
            }
            Err(err) => {
                if let Err(_) = tx.send(Err(err)) {
                    println!("send err");
                }
            }
        }
    });

    p
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

pub fn real_pair_fn((k, v): &(String, String), vars: &Vec<PairUi>) -> (String, String) {
    (parse_var_str(k, vars), parse_var_str(v, vars))
}
