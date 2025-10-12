# API Test RS

一个基于 Rust + egui 的跨平台 API 测试工具,支持 HTTP/WebSocket 请求测试、脚本自动化、性能测试等功能。

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)

## ✨ 特性

- 🚀 **跨平台支持** - Windows、macOS、Linux
- 📡 **多协议支持** - HTTP/HTTPS、WebSocket
- 🔧 **脚本引擎** - 基于 Rhai 的强大脚本支持
- 📊 **性能测试** - 支持并发请求、QPS统计、响应时间分析
- 💾 **项目管理** - 支持保存/加载测试项目
- 🎨 **现代UI** - 基于 egui 的原生UI,支持明暗主题
- 🔐 **加密工具** - 内置 MD5、SHA256、Base64 等加密函数

## 📦 安装

### 从源码编译

```bash
# 克隆项目
git clone https://github.com/yourusername/api-test-rs.git
cd api-test-rs

# 编译
cargo build --release

# 运行
./target/release/api-test-rs
```

### 使用预编译包

从 [Releases](https://github.com/yourusername/api-test-rs/releases) 页面下载对应平台的可执行文件。

## 🚀 快速开始

1. **创建测试组** - 点击左侧输入框输入组名并回车
2. **添加测试** - 在组编辑对话框中添加测试项
3. **配置请求** - 设置 URL、Method、Headers、Body 等
4. **发送请求** - 点击 Send 按钮执行测试
5. **查看结果** - 在右侧查看响应数据、Headers、统计信息

## 📖 脚本系统

### 脚本类型

- **Pre-Request Script** - 请求前执行,可修改请求参数
- **Post-Response Script** - 响应后执行,可提取数据到环境变量

### 请求对象 (可在脚本中访问/修改)

| 属性 | 类型 | 说明 | 示例 |
|------|------|------|------|
| `request.url` | String | 完整 URL | `request.url = "https://api.example.com"` |
| `request.method` | String | 请求方法 | `request.method = "POST"` |
| `request.headers["key"]` | Map | 请求头 | `request.headers["Authorization"] = "Bearer token"` |
| `request.params["key"]` | Map | 查询参数 | `request.params["page"] = "1"` |
| `request.body` | String | 请求体 | `request.body = to_json(data)` |

### 响应对象 (仅在 Post-Response Script 中可用)

| 属性 | 类型 | 说明 | 示例 |
|------|------|------|------|
| `response.status` | Integer | HTTP 状态码 | `200`, `404`, `500` |
| `response.headers["key"]` | Map | 响应头 | `response.headers["Content-Type"]` |
| `response.body` | String | 响应体 | 可用 `parse_json()` 解析 |
| `response.duration` | Integer | 响应时间(毫秒) | `1234` |

### 环境变量

| 属性 | 说明 | 示例 |
|------|------|------|
| `vars["key"]` | 读写环境变量 | `vars["token"] = "abc123"` |

## 🔧 脚本 API 参考

### 加密函数

```javascript
md5(data)                    // MD5 哈希
sha256(data)                 // SHA256 哈希
sha512(data)                 // SHA512 哈希
hmac_sha256(key, data)       // HMAC-SHA256
```

### 编码函数

```javascript
base64_encode(data)          // Base64 编码
base64_decode(data)          // Base64 解码
url_encode(data)             // URL 编码
url_decode(data)             // URL 解码
hex_encode(data)             // Hex 编码
hex_decode(data)             // Hex 解码
```

### JSON 函数

```javascript
parse_json(json_str)         // 解析 JSON 字符串
to_json(obj)                 // 对象转 JSON
json_stringify(obj)          // 对象转美化 JSON
is_valid_json(json_str)      // 检查 JSON 是否有效
```

### 工具函数

```javascript
random()                     // 随机数 (0-999999)
random_string(length)        // 生成随机字符串
timestamp()                  // 当前时间戳(秒)
timestamp_ms()               // 当前时间戳(毫秒)
uuid()                       // 生成 UUID v4
console_log(msg)             // 输出日志
```

### 文件操作

```javascript
read_file(path)              // 读取文本文件
write_file(path, content)    // 写入文本文件
append_file(path, content)   // 追加到文件
file_exists(path)            // 检查文件是否存在
delete_file(path)            // 删除文件
read_file_bytes(path)        // 读取二进制文件(Base64)
write_file_bytes(path, b64)  // 写入二进制文件(Base64)
create_dir(path)             // 创建目录
list_files(path)             // 列出目录文件
```

### HTTP 网络请求

```javascript
http_get(url)                                    // GET 请求
http_get_bytes(url)                              // GET 二进制数据
http_post(url, body)                             // POST 请求
http_request(url, method)                        // 自定义请求
http_request(url, method, body, headers)         // 完整请求
```

## 📝 脚本示例

### 基础示例

```javascript
// Pre-Request Script - 添加时间戳和签名
vars["timestamp"] = timestamp().to_string();
let sign = md5(vars["app_key"] + vars["timestamp"]);
request.headers["X-Timestamp"] = vars["timestamp"];
request.headers["X-Sign"] = sign;

// Post-Response Script - 提取 token
let result = parse_json(response.body);
if (result.code == 0) {
    vars["token"] = result.data.token;
    console_log("Token 已保存: " + vars["token"]);
}
```

### OAuth 认证

```javascript
// Pre-Request Script - 自动获取访问令牌
if (!vars.contains("access_token") || vars["token_expired"] == "true") {
    let token_resp = http_post("https://oauth.example.com/token",
        to_json(#{
            grant_type: "client_credentials",
            client_id: vars["client_id"],
            client_secret: vars["client_secret"]
        }));

    let token_data = parse_json(token_resp);
    vars["access_token"] = token_data.access_token;
    vars["token_expired"] = "false";
}

request.headers["Authorization"] = "Bearer " + vars["access_token"];
```

### 文件操作

```javascript
// 保存响应到文件
if (response.status == 200) {
    write_file("./output/response.json", response.body);
    console_log("响应已保存");
}

// 读取测试数据
let test_data = read_file("./test_data.json");
let data = parse_json(test_data);
request.body = to_json(data);

// 追加日志
let log = timestamp() + " - " + request.url + " - " + response.status + "\n";
append_file("./logs/api.log", log);
```

### 链式 API 调用

```javascript
// 从第一个 API 获取数据,用于第二个 API
let user_resp = http_get("https://api.example.com/user/profile");
let user = parse_json(user_resp);

let orders_resp = http_get("https://api.example.com/orders?userId=" + user.id);
console_log("订单数据: " + orders_resp);
```

### 下载文件

```javascript
// 下载二进制文件
let image_data = http_get_bytes("https://example.com/logo.png");
write_file_bytes("./downloads/logo.png", image_data);
console_log("图片下载完成");
```

## 🏗️ 技术栈

- **GUI框架**: [egui](https://github.com/emilk/egui) / [eframe](https://github.com/emilk/egui/tree/master/crates/eframe)
- **HTTP客户端**: [reqwest](https://github.com/seanmonstar/reqwest)
- **WebSocket**: [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)
- **脚本引擎**: [rhai](https://github.com/rhaiscript/rhai)
- **异步运行时**: [tokio](https://github.com/tokio-rs/tokio)

## 🔨 构建

### 本地构建

```bash
# 调试版本
cargo build

# 发布版本 (优化)
cargo build --release
```

### 交叉平台构建

使用 GitHub Actions 自动构建所有平台:

```bash
# 创建 tag 触发构建
git tag v1.0.0
git push origin v1.0.0
```

或手动触发工作流: Actions → Build Cross-Platform → Run workflow

## 📋 待办事项

- [ ] WebSocket 功能优化
- [ ] 支持更多认证方式 (OAuth2, API Key 等)
- [ ] 请求历史记录
- [ ] Mock Server 功能
- [ ] 导入/导出 Postman Collection
- [ ] 性能测试报告导出

## 🤝 贡献

欢迎提交 Issue 和 Pull Request!

## 📄 许可证

MIT License

## 🔗 参考链接

- [egui 官网](https://www.egui.rs/)
- [egui Demo](https://www.egui.rs/#demo)
- [Rhai 文档](https://rhai.rs/)
