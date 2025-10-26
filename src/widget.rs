use core::f32;

use crate::COLUMN_WIDTH_INITIAL;
use api_test_rs::PairUi;
use eframe::{
    egui::{self, Response, RichText, Ui},
    epaint::Color32,
};

pub fn error_button(ui: &mut Ui, text: impl Into<String>) -> Response {
    ui.add(egui::Button::new(RichText::new(text).color(Color32::BLACK)).fill(Color32::ORANGE))
}

pub fn error_label(ui: &mut Ui, text: impl Into<String>) -> Response {
    ui.label(RichText::new(text).size(20.0).color(Color32::RED))
}

pub fn pair_table(ui: &mut Ui, id: impl std::hash::Hash, pair_vec: &mut Vec<PairUi>) {
    ui.vertical(|ui| {
        if ui.button("Add").clicked() {
            pair_vec.push(PairUi::default());
        }
    });

    ui.separator();

    egui_extras::StripBuilder::new(ui)
        .size(egui_extras::Size::remainder()
        .at_least(50.0)
        .at_most(120.0)) // for the table
        // .size(egui_extras::Size::initial(200.0)) // for the table
        .vertical(|mut strip| {
            strip.cell(|ui| {
                egui::ScrollArea::vertical().id_salt(id).show(ui, |ui| {
                    // let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

                    let  table = egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(egui_extras::Column::auto())
                        .column(egui_extras::Column::initial(COLUMN_WIDTH_INITIAL).range(100.0..=400.0))
                        .column(egui_extras::Column::initial(COLUMN_WIDTH_INITIAL).range(100.0..=400.0))
                        .column(egui_extras::Column::initial(100.0).at_least(40.0).at_most(400.0))
                        // .column(egui_extras::Column::initial(100.0).range(40.0..=300.0))
                        // .column( egui_extras::Column::initial(100.0).at_least(40.0), )
                        // .column(egui_extras::Column::remainder())
                        // .max_scroll_height(200.0)
                        .min_scrolled_height(10.0)
                        // .scroll_to_row(1, Some(egui::Align::BOTTOM))
                        ;

                    table
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.strong("");
                            });
                            header.col(|ui| {
                                ui.strong("Key");
                            });
                            header.col(|ui| {
                                ui.strong("Value");
                            });
                        })
                        .body(|mut body| {
                            pair_vec.retain_mut(|el| {
                                let mut is_retain = true;

                                body.row(30.0, |mut row| {
                                    row.col(|ui| {
                                        ui.checkbox(&mut el.disable, "");
                                    });

                                    row.col(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut el.key)
                                                .desired_width(f32::INFINITY),
                                        );
                                    });

                                    row.col(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut el.value)
                                                .desired_width(f32::INFINITY),
                                        );
                                    });

                                    row.col(|ui| {
                                        if error_button(ui,"Del").clicked() {
                                            is_retain = false;
                                        }
                                    });
                                });
                                is_retain
                            });
                        })
                });
            });
        });
}

pub fn horizontal_tabs<T>(ui: &mut Ui, tabs: std::slice::Iter<T>, current_value: &mut T)
where
    T: Clone + PartialEq + AsRef<str> + ?Sized,
{
    ui.horizontal(|ui| {
        tabs.for_each(|label| {
            ui.selectable_value(current_value, label.to_owned(), label.as_ref());
        });
    });
}

pub fn pair_table_with_file_picker(ui: &mut Ui, id: impl std::hash::Hash, pair_vec: &mut Vec<PairUi>) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            if ui.button("Add").clicked() {
                pair_vec.push(PairUi::default());
            }
            ui.label(RichText::new("ℹ️ 提示：文件路径需要 @ 前缀，例如 @file.jpg；多个文件用空格分隔，例如 @a.jpg @b.jpg")
                .color(Color32::from_rgb(100, 149, 237))
                .italics());
        });
    });

    ui.separator();

    egui_extras::StripBuilder::new(ui)
        .size(egui_extras::Size::remainder()
        .at_least(50.0)
        .at_most(120.0))
        .vertical(|mut strip| {
            strip.cell(|ui| {
                egui::ScrollArea::vertical().id_salt(id).show(ui, |ui| {
                    let table = egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(egui_extras::Column::auto())
                        .column(egui_extras::Column::initial(COLUMN_WIDTH_INITIAL).range(100.0..=400.0))
                        .column(egui_extras::Column::initial(COLUMN_WIDTH_INITIAL).range(100.0..=400.0))
                        .column(egui_extras::Column::initial(100.0).at_least(40.0).at_most(400.0))
                        .min_scrolled_height(10.0);

                    table
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.strong("");
                            });
                            header.col(|ui| {
                                ui.strong("Key");
                            });
                            header.col(|ui| {
                                ui.strong("Value");
                            });
                            header.col(|ui| {
                                ui.strong("Actions");
                            });
                        })
                        .body(|mut body| {
                            pair_vec.retain_mut(|el| {
                                let mut is_retain = true;

                                body.row(30.0, |mut row| {
                                    row.col(|ui| {
                                        ui.checkbox(&mut el.disable, "");
                                    });

                                    row.col(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut el.key)
                                                .desired_width(f32::INFINITY),
                                        );
                                    });

                                    row.col(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::TextEdit::singleline(&mut el.value)
                                                    .desired_width(f32::INFINITY),
                                            );
                                        });
                                    });

                                    row.col(|ui| {
                                        ui.horizontal(|ui| {
                                            if ui.button("📁").on_hover_text("选择文件").clicked() {
                                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                                    let file_path = format!("@{}", path.display().to_string());
                                                    if el.value.is_empty() {
                                                        el.value = file_path;
                                                    } else {
                                                        el.value = format!("{} {}", el.value, file_path);
                                                    }
                                                }
                                            }
                                            if error_button(ui, "Del").clicked() {
                                                is_retain = false;
                                            }
                                        });
                                    });
                                });
                                is_retain
                            });
                        })
                });
            });
        });
}

pub fn code_view_ui(ui: &mut egui::Ui, mut code: &str) {
    code_view_with_syntax(ui, code, "json");
}

pub fn code_view_with_syntax(ui: &mut egui::Ui, mut code: &str, language: &str) {
    use egui_extras::syntax_highlighting;

    let theme = syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut layout_job = syntax_highlighting::highlight(
            ui.ctx(),
            ui.style(),
            &theme,
            buf.as_str(),
            language,
        );
        layout_job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(layout_job))
    };

    ui.add(
        egui::TextEdit::multiline(&mut code)
            .font(egui::TextStyle::Monospace)
            .code_editor()
            .desired_rows(1)
            .lock_focus(true)
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter),
    );
}

/// 可编辑的代码编辑器，带语法高亮
pub fn code_edit_with_syntax(ui: &mut egui::Ui, code: &mut String, language: &str, desired_rows: usize) {
    use egui_extras::syntax_highlighting;

    let theme = syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut layout_job = syntax_highlighting::highlight(
            ui.ctx(),
            ui.style(),
            &theme,
            buf.as_str(),
            language,
        );
        layout_job.wrap.max_width = wrap_width;
        ui.fonts(|f| f.layout_job(layout_job))
    };

    ui.add(
        egui::TextEdit::multiline(code)
            .font(egui::TextStyle::Monospace)
            .code_editor()
            .desired_rows(desired_rows)
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter),
    );
}
