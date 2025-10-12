## api-test-rs

## build

```
cargo build --release
```

See also:

- https://github.com/emilk/egui
- https://www.egui.rs/#demo

todo:

- 优化 ws

## 脚本

```
request.url | 完整 URL | request.url = "https://new.com/api"
request.method | 请求方法 | request.method = "POST"
request.headers["key"] | 请求头 | request.headers["Authorization"] = "Bearer " + vars["token"]
request.params["key"] | 查询参数 ✅ | request.params["page"] = "1"
request.body | 请求体 | request.body = to_json(body_obj)
vars["key"] | 环境变量 | vars["timestamp"] = timestamp().to_string()
```

### 在 Post-Response Script 中，您可以访问原始请求信息：

| 属性                   | 类型   | 说明     | 示例                            |
| ---------------------- | ------ | -------- | ------------------------------- |
| request.url            | String | 请求 URL | "https://api.example.com/users" |
| request.method         | String | 请求方法 | "POST"                          |
| request.headers["key"] | Map    | 请求头   | request.headers["Content-Type"] |
| request.params["key"]  | Map    | 查询参数 | request.params["id"]            |
| request.body           | String | 请求体   | 可用 parse_json() 解析          |

### 响应信息：

| 属性                    | 类型    | 说明           | 示例                             |
| ----------------------- | ------- | -------------- | -------------------------------- |
| response.status         | Integer | HTTP 状态码    | 200, 404, 500                    |
| response.headers["key"] | Map     | 响应头         | response.headers["Content-Type"] |
| response.body           | String  | 响应体         | 可用 parse_json() 解析           |
| response.duration       | Integer | 响应时间(毫秒) | 1234                             |

## 脚本 API 参考

### 加密函数
- `md5(data)` - MD5 哈希
- `sha256(data)` - SHA256 哈希
- `sha512(data)` - SHA512 哈希
- `hmac_sha256(key, data)` - HMAC-SHA256

### 编码函数
- `base64_encode(data)` - Base64 编码
- `base64_decode(data)` - Base64 解码
- `url_encode(data)` - URL 编码
- `url_decode(data)` - URL 解码
- `hex_encode(data)` - Hex 编码
- `hex_decode(data)` - Hex 解码

### JSON 函数
- `parse_json(json_str)` - 解析 JSON 字符串为对象
- `to_json(obj)` - 对象转 JSON 字符串
- `json_stringify(obj)` - 对象转美化的 JSON 字符串
- `is_valid_json(json_str)` - 检查 JSON 是否有效

### 工具函数
- `random()` - 生成随机数 (0-999999)
- `random_string(length)` - 生成随机字符串
- `timestamp()` - 获取当前时间戳(秒)
- `timestamp_ms()` - 获取当前时间戳(毫秒)
- `uuid()` - 生成 UUID v4
- `console_log(msg)` - 输出日志（支持字符串、数字、布尔值、对象）

### 文件操作函数
- `read_file(path)` - 读取文本文件内容
- `write_file(path, content)` - 写入文本文件（覆盖），返回 bool
- `append_file(path, content)` - 追加到文件，返回 bool
- `file_exists(path)` - 检查文件是否存在，返回 bool
- `delete_file(path)` - 删除文件，返回 bool
- `read_file_bytes(path)` - 读取二进制文件（返回 Base64 字符串）
- `write_file_bytes(path, base64_content)` - 写入二进制文件（从 Base64），返回 bool
- `create_dir(path)` - 创建目录（递归），返回 bool
- `list_files(path)` - 列出目录中的文件，返回数组

### HTTP 网络请求函数
- `http_get(url)` - 发送 GET 请求，返回响应体字符串（文本）
- `http_get_bytes(url)` - 发送 GET 请求，返回 Base64 编码的二进制数据
- `http_post(url, body)` - 发送 POST 请求（JSON），返回响应体字符串
- `http_request(url, method)` - 发送 HTTP 请求，返回对象 `{status, body}`
- `http_request(url, method, body, headers)` - 完整 HTTP 请求，返回对象 `{status, headers, body}`

### 文件操作示例

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

// 检查文件
if (file_exists("./config.json")) {
    let config = parse_json(read_file("./config.json"));
    vars["api_key"] = config.api_key;
}

// 保存二进制响应（图片等）
if (response.headers["content-type"].contains("image")) {
    let base64_data = base64_encode(response.body);
    write_file_bytes("./downloads/image.png", base64_data);
}

// 列出文件
let files = list_files("./data");
for file in files {
    console_log(file);
}
```

### HTTP 网络请求示例

```javascript
// 简单 GET 请求
let data = http_get("https://api.example.com/users");
let users = parse_json(data);
console_log("用户数量: " + users.length);

// POST 请求
let payload = #{
    name: "John",
    email: "john@example.com"
};
let response = http_post("https://api.example.com/users", to_json(payload));
console_log("创建结果: " + response);

// 完整 HTTP 请求
let headers = #{
    "Authorization": "Bearer " + vars["token"],
    "Content-Type": "application/json"
};
let body = to_json(#{ query: "test" });
let resp = http_request("https://api.example.com/search", "POST", body, headers);

if (resp.status == 200) {
    let result = parse_json(resp.body);
    console_log("搜索结果: " + to_json(result));
} else {
    console_log("请求失败，状态码: " + resp.status);
}

// 链式调用 - 从第一个 API 获取 token，用于第二个 API
let auth_resp = http_post("https://api.example.com/auth/login",
    to_json(#{ username: "user", password: "pass" }));
let auth_data = parse_json(auth_resp);
vars["token"] = auth_data.token;

// 使用获取的 token
let data_resp = http_get("https://api.example.com/data?token=" + vars["token"]);
console_log("数据: " + data_resp);

// OAuth 示例 - 在 Pre-Request Script 中获取访问令牌
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

// 下载二进制文件（图片、PDF 等）
let image_data = http_get_bytes("https://example.com/logo.png");
write_file_bytes("./downloads/logo.png", image_data);
console_log("图片下载完成");

// 在 request.body 中使用远程数据
// 如果是文本/JSON
request.body = http_get("https://example.com/data.json");

// 如果是二进制数据（需要保持 Base64 格式或解码）
let binary_data = http_get_bytes("https://example.com/file.bin");
request.body = binary_data; // Base64 格式
// 或者先保存到文件再使用
write_file_bytes("./temp/data.bin", binary_data);
```
