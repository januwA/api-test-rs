use tauri::State;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tauri_plugin_store::StoreExt;

use api_test_lib::{
    util, HttpRequestConfig, HttpResponse, PairUi, Project,
};

use crate::AppState;

// ========== 项目管理 ==========

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectWithPath {
    pub project: Project,
    pub path: String,
}

#[tauri::command]
pub async fn load_project(
    app: tauri::AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<ProjectWithPath, String> {
    let project = util::load_project(&path).map_err(|e| e.to_string())?;

    // 更新状态
    if let Ok(mut proj) = state.project.lock() {
        *proj = Some(project.clone());
    }

    // 保存到配置中作为最后打开的项目
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("lastProjectPath", path.clone());
    store.save().map_err(|e| e.to_string())?;

    Ok(ProjectWithPath {
        project,
        path,
    })
}

#[tauri::command]
pub async fn save_project(
    app: tauri::AppHandle,
    dir: String,
    project: Project,
    font_size: f32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    util::save_project(&dir, &project, font_size).map_err(|e| e.to_string())?;

    // 更新状态
    if let Ok(mut proj) = state.project.lock() {
        *proj = Some(project.clone());
    }

    // 保存项目路径到配置中
    let project_file_path = format!("{}/{}.json", dir, project.name.replace(" ", "_"));
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("lastProjectPath", project_file_path);
    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn create_project(
    name: String,
    state: State<'_, AppState>,
) -> Result<Project, String> {
    let project = Project::from_name(&name);

    // 更新状态
    if let Ok(mut proj) = state.project.lock() {
        *proj = Some(project.clone());
    }

    Ok(project)
}

#[tauri::command]
pub async fn get_project(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<ProjectWithPath>, String> {
    // 首先检查内存中的项目
    if let Ok(proj) = state.project.lock() {
        if let Some(ref project) = *proj {
            // 尝试从配置中获取路径
            let store = app.store("settings.json").map_err(|e| e.to_string())?;
            if let Some(last_project_path) = store.get("lastProjectPath") {
                if let Some(path_str) = last_project_path.as_str() {
                    return Ok(Some(ProjectWithPath {
                        project: project.clone(),
                        path: path_str.to_string(),
                    }));
                }
            }
        }
    }

    // 如果内存中没有项目，尝试从配置中加载最后打开的项目
    let store = app.store("settings.json").map_err(|e| e.to_string())?;

    if let Some(last_project_path) = store.get("lastProjectPath") {
        if let Some(path_str) = last_project_path.as_str() {
            match util::load_project(path_str) {
                Ok(project) => {
                    // 更新内存状态
                    if let Ok(mut proj) = state.project.lock() {
                        *proj = Some(project.clone());
                    }
                    return Ok(Some(ProjectWithPath {
                        project,
                        path: path_str.to_string(),
                    }));
                }
                Err(e) => {
                    eprintln!("Failed to load last project from {}: {}", path_str, e);
                    // 继续创建默认项目
                }
            }
        }
    }

    // 如果没有配置或加载失败，返回None让前端创建默认项目
    Ok(None)
}

#[tauri::command]
pub async fn update_project(
    project: Project,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Ok(mut proj) = state.project.lock() {
        *proj = Some(project);
        Ok(())
    } else {
        Err("Failed to update project state".to_string())
    }
}

// ========== HTTP 请求 ==========

#[tauri::command]
pub async fn send_http_request(
    config: HttpRequestConfig,
    variables: Vec<PairUi>,
    state: State<'_, AppState>,
) -> Result<HttpResponse, String> {
    let script_output = state.script_output.clone();
    util::http_send(&config, &variables, script_output)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRequestProgress {
    pub completed: usize,
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}

#[tauri::command]
pub async fn send_http_batch(
    config: HttpRequestConfig,
    variables: Vec<PairUi>,
    count: usize,
    state: State<'_, AppState>,
) -> Result<Vec<HttpResponse>, String> {
    use futures::stream::{FuturesUnordered, StreamExt};
    use tokio::sync::Semaphore;
    use std::sync::Arc;

    let script_output = state.script_output.clone();
    let max_concurrent = 100.min(count); // 限制最大并发数为100
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut futures = FuturesUnordered::new();
    let mut responses = Vec::new();

    for _i in 0..count {
        let req_cfg = config.clone();
        let vars = variables.clone();
        let output = script_output.clone();
        let permit = semaphore.clone().acquire_owned().await
            .map_err(|e| format!("Failed to acquire semaphore: {}", e))?;

        futures.push(async move {
            let result = util::http_send(&req_cfg, &vars, output).await;
            drop(permit); // 释放信号量
            result
        });
    }

    // 收集所有结果
    while let Some(result) = futures.next().await {
        match result {
            Ok(response) => responses.push(response),
            Err(e) => {
                eprintln!("Request failed: {}", e);
                // 继续处理其他请求，不中断整个批处理
            }
        }
    }

    Ok(responses)
}

// ========== WebSocket ==========

#[tauri::command]
pub async fn connect_websocket(
    _url: String,
) -> Result<String, String> {
    // WebSocket 连接逻辑将在后续实现
    // 这里返回一个连接 ID
    Ok("ws_connection_id".to_string())
}

#[tauri::command]
pub async fn send_websocket_message(
    _connection_id: String,
    _message: String,
) -> Result<(), String> {
    // WebSocket 发送消息逻辑
    Ok(())
}

#[tauri::command]
pub async fn close_websocket(
    _connection_id: String,
) -> Result<(), String> {
    // WebSocket 关闭连接逻辑
    Ok(())
}

// ========== 变量管理 ==========

#[tauri::command]
pub async fn get_variables(
    state: State<'_, AppState>,
) -> Result<Vec<PairUi>, String> {
    if let Ok(proj) = state.project.lock() {
        if let Some(project) = proj.as_ref() {
            Ok(project.variables.clone())
        } else {
            Ok(Vec::new())
        }
    } else {
        Err("Failed to access project state".to_string())
    }
}

#[tauri::command]
pub async fn set_variable(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Ok(mut proj) = state.project.lock() {
        if let Some(project) = proj.as_mut() {
            // 查找并更新现有变量,或添加新变量
            if let Some(var) = project.variables.iter_mut().find(|v| v.key == key) {
                var.value = value;
            } else {
                project.variables.push(PairUi {
                    key,
                    value,
                    disable: false,
                });
            }
            Ok(())
        } else {
            Err("No project loaded".to_string())
        }
    } else {
        Err("Failed to access project state".to_string())
    }
}

#[tauri::command]
pub async fn delete_variable(
    key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Ok(mut proj) = state.project.lock() {
        if let Some(project) = proj.as_mut() {
            project.variables.retain(|v| v.key != key);
            Ok(())
        } else {
            Err("No project loaded".to_string())
        }
    } else {
        Err("Failed to access project state".to_string())
    }
}

// ========== 脚本输出 ==========

#[tauri::command]
pub async fn get_script_output(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    if let Ok(output) = state.script_output.lock() {
        Ok(output.clone())
    } else {
        Err("Failed to access script output".to_string())
    }
}

#[tauri::command]
pub async fn clear_script_output(
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Ok(mut output) = state.script_output.lock() {
        output.clear();
        Ok(())
    } else {
        Err("Failed to clear script output".to_string())
    }
}

// ========== 工具函数 ==========

#[tauri::command]
pub async fn parse_curl_command(
    _curl: String,
) -> Result<HttpRequestConfig, String> {
    // 解析 cURL 命令的逻辑
    // 这里先返回一个默认配置
    Ok(HttpRequestConfig::default())
}

#[tauri::command]
pub async fn generate_curl_command(
    config: HttpRequestConfig,
    variables: Vec<PairUi>,
) -> Result<String, String> {
    Ok(util::to_curl_command(&config, &variables))
}
