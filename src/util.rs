#![allow(warnings, unused)]

use std::ffi::OsStr;

use anyhow::{anyhow, bail, Result};
use eframe::egui;
use image::GenericImageView;
use serde_json::json;

use crate::{AppConfig, Group, PairUi};

pub fn load_app_icon() -> eframe::IconData {
    let app_icon_bytes = include_bytes!("../data/icon.png");
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

pub async fn get_utf8_data(data_vec: &Option<Vec<u8>>) -> Option<String> {
    match data_vec {
        Some(data_vec) => match std::str::from_utf8(data_vec.as_ref()) {
            Ok(utf8_text) => Some(utf8_text.to_string()),
            Err(_) => None,
        },
        _ => None,
    }
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
pub fn load_project(project_path: &str) -> Result<(String, Vec<Group>)> {
    if project_path.is_empty() {
        bail!("加载路径不能为空")
    }

    let load_path = std::path::Path::new(project_path);
    if !load_path.exists() {
        bail!("文件不存在")
    }
    let data = std::fs::read(&load_path)?;
    let dat: Vec<Group> = serde_json::from_slice(data.as_slice())?;
    let project_name = get_filename(&load_path)?;
    Ok((project_name, dat))
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

pub fn save_current_project(dir: &str, project_name: &str, groups: &Vec<Group>) -> Result<()> {
    if project_name.is_empty() {
        bail!("项目名称不能为空")
    };

    // 先保存group
    let group_data = serde_json::to_vec(groups)?;
    let save_path = std::path::Path::new(dir).join(format!("{}.json", &project_name));
    std::fs::write(&save_path, group_data)?;

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
