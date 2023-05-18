use eframe::egui;
use image::GenericImageView;

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

#[allow(clippy::needless_pass_by_value)]
pub fn parse_ehttp_response(
    response: ehttp::Response,
) -> Result<egui_extras::RetainedImage, String> {
    let content_type = response.content_type().unwrap_or_default();
    if content_type.starts_with("image/") {
        egui_extras::RetainedImage::from_image_bytes(&response.url, &response.bytes)
    } else {
        Err(format!(
            "Expected image, found content-type {:?}",
            content_type
        ))
    }
}

pub async fn get_utf8_date(data_vec: &Option<Vec<u8>>) -> Option<String> {
    match data_vec {
        Some(data_vec) => match std::str::from_utf8(data_vec.as_ref()) {
            Ok(utf8_text) => Some(utf8_text.to_string()),
            Err(_) => None,
        },
        _ => None,
    }
}
