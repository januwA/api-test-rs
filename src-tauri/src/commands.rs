use tauri::{Emitter, State};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequestProgress {
    pub completed: usize,
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub current_batch: usize,
    pub total_batches: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRequestConfig {
    pub max_concurrent: usize,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequestResult {
    pub first_response: Option<HttpResponse>,
    pub completed: usize,
    pub success: usize,
    pub failed: usize,
    pub total_duration: u128,
    pub min_duration: u128,
    pub max_duration: u128,
    pub cancelled: bool,
}

#[tauri::command]
pub async fn send_http_batch(
    app: tauri::AppHandle,
    config: HttpRequestConfig,
    variables: Vec<PairUi>,
    count: usize,
    batch_config: Option<BatchRequestConfig>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    println!("🚀 Starting batch request: {} requests", count);

    // 克隆需要的状态数据
    let script_output = state.script_output.clone();
    let cancel_tokens = state.cancel_tokens.clone();

    // 在后台任务中执行批量请求，立即返回
    tokio::spawn(async move {
        let result = execute_batch_request(
            app.clone(),
            config,
            variables,
            count,
            batch_config,
            script_output,
            cancel_tokens,
        ).await;

        // 发送最终结果事件
        match result {
            Ok(batch_result) => {
                println!("✅ Batch request completed: {}/{} requests", batch_result.completed, count);
                let _ = app.emit("batch-completed", batch_result);
            }
            Err(e) => {
                eprintln!("❌ Batch request failed: {}", e);
                let _ = app.emit("batch-error", e);
            }
        }
    });

    Ok(())
}

async fn execute_batch_request(
    app: tauri::AppHandle,
    config: HttpRequestConfig,
    variables: Vec<PairUi>,
    count: usize,
    batch_config: Option<BatchRequestConfig>,
    script_output: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    cancel_tokens: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>>,
) -> Result<BatchRequestResult, String> {
    use futures::stream::{FuturesUnordered, StreamExt};
    use tokio::sync::Semaphore;
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};
    use tokio_util::sync::CancellationToken;

    // 默认配置：并发数1000，批次大小10000（压力测试模式）
    let batch_config = batch_config.unwrap_or(BatchRequestConfig {
        max_concurrent: 1000,
        batch_size: 10000,
    });

    // 移除并发数限制，让用户自己控制
    let max_concurrent = batch_config.max_concurrent.max(1); // 只保证最小为1
    let batch_size = batch_config.batch_size.max(100); // 批次最小100

    let total_batches = (count + batch_size - 1) / batch_size; // 计算总批次数
    let cancellation_token = CancellationToken::new();

    // 存储取消令牌到状态中，这样可以通过其他命令取消
    if let Ok(mut tokens) = cancel_tokens.lock() {
        tokens.insert("batch_request".to_string(), cancellation_token.clone());
    }

    // 统计数据
    let mut completed_count = 0;
    let mut success_count = 0;
    let mut failed_count = 0;
    let mut total_duration: u128 = 0;
    let mut min_duration: u128 = u128::MAX;
    let mut max_duration: u128 = 0;
    let mut first_response: Option<HttpResponse> = None;

    println!("⚙️ Configuration: {} concurrent, {} batch size, {} total batches",
             max_concurrent, batch_size, total_batches);

    let start_time = std::time::Instant::now();

    for batch_index in 0..total_batches {
        let batch_start = std::time::Instant::now();
        println!("\n📦 ========== Batch {}/{} START ==========", batch_index + 1, total_batches);
        println!("📊 Current stats: completed={}, success={}, failed={}",
                 completed_count, success_count, failed_count);

        // 检查是否被取消
        if cancellation_token.is_cancelled() {
            println!("❌ Batch request cancelled at batch {}", batch_index + 1);
            return Ok(BatchRequestResult {
                first_response,
                completed: completed_count,
                success: success_count,
                failed: failed_count,
                total_duration,
                min_duration: if min_duration == u128::MAX { 0 } else { min_duration },
                max_duration,
                cancelled: true,
            });
        }

        let start_idx = batch_index * batch_size;
        let end_idx = (start_idx + batch_size).min(count);

        // 发送批次开始进度事件
        let _ = app.emit("batch-progress", BatchRequestProgress {
            completed: completed_count,
            total: count,
            success: success_count,
            failed: failed_count,
            current_batch: batch_index + 1,
            total_batches,
        });

        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut futures = FuturesUnordered::new();

        println!("🚀 Launching {} requests (index {}-{})", end_idx - start_idx, start_idx, end_idx - 1);
        println!("🔒 Semaphore permits available: {}", max_concurrent);

        // 启动当前批次的请求 - 修复死锁问题
        for i in start_idx..end_idx {
            let req_cfg = config.clone();
            let vars = variables.clone();
            let output = script_output.clone();
            let semaphore_clone = semaphore.clone();  // 只clone信号量，不获取许可

            if (i - start_idx) % 1000 == 0 && i > start_idx {
                println!("   ⏳ Created {}/{} futures in this batch", i - start_idx, end_idx - start_idx);
            }

            // 在future内部获取许可，避免死锁
            let cancel_token = cancellation_token.clone();

            futures.push(async move {
                // 提前检查取消状态，避免不必要的工作
                if cancel_token.is_cancelled() {
                    return Err(format!("Request {} cancelled before execution", i + 1));
                }

                if i % 100 == 0 {
                    println!("   🔄 Future {} is being polled, acquiring permit...", i + 1);
                }

                // 在这里获取许可，futures可以并发竞争
                let permit = match semaphore_clone.acquire_owned().await {
                    Ok(p) => {
                        // 获取许可后再次检查取消状态
                        if cancel_token.is_cancelled() {
                            return Err(format!("Request {} cancelled after acquiring permit", i + 1));
                        }

                        if i % 100 == 0 {
                            println!("   🎫 Future {} got permit, sending HTTP request...", i + 1);
                        }
                        p
                    },
                    Err(e) => {
                        eprintln!("   ❌ Failed to acquire semaphore: {}", e);
                        return Err(format!("Semaphore error: {}", e));
                    }
                };

                if i % 100 == 0 {
                    println!("   📡 Sending HTTP request {} to {}", i + 1, req_cfg.url);
                }

                // 设置单个请求的超时时间为30秒
                let request_start = std::time::Instant::now();
                let result = timeout(
                    Duration::from_secs(30),
                    util::http_send(&req_cfg, &vars, output)
                ).await;

                if i % 100 == 0 {
                    println!("   📥 Request {} completed, result: {:?}", i + 1,
                             if result.is_ok() { "OK" } else { "ERROR" });
                }

                drop(permit); // 释放信号量
                let request_duration = request_start.elapsed();

                match result {
                    Ok(Ok(response)) => {
                        if i % 5000 == 0 {
                            println!("   ✅ Request {} completed in {:?}", i + 1, request_duration);
                        }
                        Ok(response)
                    },
                    Ok(Err(e)) => {
                        eprintln!("   ❌ Request {} FAILED after {:?}: {}", i + 1, request_duration, e);
                        Err(format!("Request {} failed: {}", i + 1, e))
                    },
                    Err(_) => {
                        eprintln!("   ⏱️ Request {} TIMEOUT after 30s", i + 1);
                        Err(format!("Request {} timed out", i + 1))
                    },
                }
            });
        }

        println!("✅ All {} futures created, waiting for completion...", end_idx - start_idx);

        // 收集当前批次的结果（只保存第一个响应）
        let mut batch_completed = 0;
        while let Some(result) = futures.next().await {
            batch_completed += 1;

            if batch_completed % 100 == 0 {
                // 每100个请求检查一次取消状态（更频繁）
                if cancellation_token.is_cancelled() {
                    println!("⚠️ Cancellation detected at {} requests", batch_completed);
                    println!("🛑 Aborting batch {}/{}...", batch_index + 1, total_batches);
                    break;
                }
            }

            if batch_completed % 5000 == 0 {
                println!("   📊 Collected {}/{} results in this batch", batch_completed, end_idx - start_idx);
            }

            match result {
                Ok(response) => {
                    // 统计成功/失败
                    if response.status >= 200 && response.status < 300 {
                        success_count += 1;
                    } else {
                        failed_count += 1;
                        if failed_count <= 5 {
                            println!("   ⚠️ Failed response: status={}, url={}", response.status, config.url);
                        }
                    }

                    // 统计时长
                    let duration = response.duration as u128;
                    total_duration += duration;
                    min_duration = min_duration.min(duration);
                    max_duration = max_duration.max(duration);

                    // 只保存第一个响应用于显示
                    if first_response.is_none() {
                        println!("💾 Saved first response: status={}, duration={}ms", response.status, response.duration);
                        first_response = Some(response);
                    }
                    // 其他响应直接丢弃，由Rust自动释放内存

                    completed_count += 1;
                }
                Err(e) => {
                    if failed_count < 5 {
                        eprintln!("❌ ERROR in batch request: {}", e);
                    }
                    failed_count += 1;
                    completed_count += 1;

                    // 如果第一个请求失败，也保存一个失败响应
                    if first_response.is_none() {
                        println!("💾 Saved first failed response");
                        first_response = Some(HttpResponse {
                            headers: reqwest::header::HeaderMap::new(),
                            headers_str: String::new(),
                            request_headers_str: String::new(),
                            version: reqwest::Version::HTTP_11,
                            status: 0,
                            img: None,
                            text: Some(format!("Request failed: {}", e)),
                            data_vec: None,
                            duration: 0,
                            request_size: 0,
                            response_size: 0,
                            modified_vars: None,
                        });
                    }
                }
            }

            // 每完成1000个请求发送一次进度更新
            if completed_count % 1000 == 0 {
                let progress = BatchRequestProgress {
                    completed: completed_count,
                    total: count,
                    success: success_count,
                    failed: failed_count,
                    current_batch: batch_index + 1,
                    total_batches,
                };
                match app.emit("batch-progress", progress.clone()) {
                    Ok(_) => {},
                    Err(e) => eprintln!("⚠️ Failed to emit progress event: {}", e),
                }
            }
        }

        let batch_duration = batch_start.elapsed();
        println!("✅ Batch {} COMPLETE: {} requests in {:?} ({:.2} req/s)",
                 batch_index + 1, batch_completed, batch_duration,
                 batch_completed as f64 / batch_duration.as_secs_f64());

        // 如果被取消，立即停止处理
        if cancellation_token.is_cancelled() {
            println!("🛑 Batch request CANCELLED by user");
            println!("📊 Final stats: completed={}, success={}, failed={}",
                     completed_count, success_count, failed_count);

            return Ok(BatchRequestResult {
                first_response,
                completed: completed_count,
                success: success_count,
                failed: failed_count,
                total_duration,
                min_duration: if min_duration == u128::MAX { 0 } else { min_duration },
                max_duration,
                cancelled: true,
            });
        }

        // 压力测试：批次间无延迟，持续施压
        // 这是故意的设计，用于测试服务器在持续高压力下的表现
    }

    // 清理取消令牌
    if let Ok(mut tokens) = cancel_tokens.lock() {
        tokens.remove("batch_request");
    }

    let total_elapsed = start_time.elapsed();
    let avg_duration = if completed_count > 0 { total_duration / completed_count as u128 } else { 0 };

    println!("\n🎉 ========== BATCH REQUEST COMPLETE ==========");
    println!("📊 Total: {} requests", count);
    println!("✅ Completed: {} requests", completed_count);
    println!("✔️  Success: {} requests ({:.1}%)", success_count,
             (success_count as f64 / completed_count as f64) * 100.0);
    println!("❌ Failed: {} requests ({:.1}%)", failed_count,
             (failed_count as f64 / completed_count as f64) * 100.0);
    println!("⏱️  Total time: {:?}", total_elapsed);
    println!("⚡ Throughput: {:.2} req/s", completed_count as f64 / total_elapsed.as_secs_f64());
    println!("📈 Avg response: {}ms", avg_duration);
    println!("📉 Min response: {}ms", if min_duration == u128::MAX { 0 } else { min_duration });
    println!("📈 Max response: {}ms", max_duration);
    println!("================================================\n");

    Ok(BatchRequestResult {
        first_response,
        completed: completed_count,
        success: success_count,
        failed: failed_count,
        total_duration,
        min_duration: if min_duration == u128::MAX { 0 } else { min_duration },
        max_duration,
        cancelled: false,
    })
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

// ========== 请求取消 ==========

#[tauri::command]
pub async fn cancel_batch_request(
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Ok(cancel_tokens) = state.cancel_tokens.lock() {
        if let Some(token) = cancel_tokens.get("batch_request") {
            token.cancel();
            Ok(())
        } else {
            Err("No active batch request to cancel".to_string())
        }
    } else {
        Err("Failed to access cancel tokens".to_string())
    }
}
