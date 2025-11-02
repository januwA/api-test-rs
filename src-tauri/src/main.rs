// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

// 引入库模块
use api_test_lib::Project;

mod commands;

// 应用状态
pub struct AppState {
    pub project: Arc<Mutex<Option<Project>>>,
    pub script_output: Arc<Mutex<Vec<String>>>,
}

fn main() {
    let app_state = AppState {
        project: Arc::new(Mutex::new(None)),
        script_output: Arc::new(Mutex::new(Vec::new())),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // 项目管理
            commands::load_project,
            commands::save_project,
            commands::create_project,
            commands::get_project,
            commands::update_project,
            // HTTP 请求
            commands::send_http_request,
            commands::send_http_batch,
            // WebSocket
            commands::connect_websocket,
            commands::send_websocket_message,
            commands::close_websocket,
            // 变量管理
            commands::get_variables,
            commands::set_variable,
            commands::delete_variable,
            // 脚本输出
            commands::get_script_output,
            commands::clear_script_output,
            // 工具函数
            commands::parse_curl_command,
            commands::generate_curl_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
