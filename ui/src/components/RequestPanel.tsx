import { useState, useEffect } from "react";
import {
  Select,
  Input,
  InputNumber,
  Button,
  Checkbox,
  Space,
  Radio,
  Typography,
  message,
  Card,
  Tooltip,
  Empty,
  Segmented,
  Modal,
  Progress,
} from "antd";
import {
  SendOutlined,
  CopyOutlined,
  LinkOutlined,
  ApiOutlined,
  FolderOpenOutlined,
  PlusOutlined,
  ClearOutlined,
  DeleteOutlined,
  SettingOutlined,
} from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../stores/projectStore";
import { useResponseStore } from "../stores/responseStore";
import { Method, RequestTab } from "../types";
import { Tabs as AntdTabs } from "antd";
import PairTable from "./common/PairTable";
import CodeEditor from "./common/CodeEditor";


const { Text } = Typography;

interface BatchProgress {
  completed: number;
  total: number;
  success: number;
  failed: number;
  current_batch: number;
  total_batches: number;
}

function RequestPanel() {
  const { project, selectedTest, updateTest } = useProjectStore();
  const { setResponse } = useResponseStore();
  const [activeTab, setActiveTab] = useState<RequestTab>(RequestTab.Params);
  const [scriptTab, setScriptTab] = useState<string>("pre-request");
  const [isLoading, setIsLoading] = useState(false);
  const [helpModalVisible, setHelpModalVisible] = useState(false);
  const [batchConfigModalVisible, setBatchConfigModalVisible] = useState(false);
  const [batchProgress, setBatchProgress] = useState<BatchProgress | null>(null);
  const [batchConcurrent, setBatchConcurrent] = useState<number>(1000);
  const [batchSize, setBatchSize] = useState<number>(10000);

  // 监听批量请求进度和完成事件
  useEffect(() => {
    console.log("🎧 [Frontend] Setting up event listeners...");

    const unlistenProgress = listen<BatchProgress>("batch-progress", (event) => {
      console.log("📊 [Frontend] Progress update:", event.payload);
      setBatchProgress(event.payload);
    }).catch(err => {
      console.error("❌ [Frontend] Failed to listen to batch-progress:", err);
    });

    const unlistenCompleted = listen<any>("batch-completed", (event) => {
      console.log("🎉 [Frontend] Batch completed event received:", event.payload);
      const batchResult = event.payload;

      // 计算平均时长
      const avgDuration = batchResult.completed > 0
        ? batchResult.total_duration / batchResult.completed
        : 0;

      // 使用第一个响应作为显示内容
      const firstResponse = batchResult.first_response || {
        status: 0,
        headers_str: "",
        request_headers_str: "",
        version: "HTTP/1.1",
        text: "No response",
        duration: 0,
        request_size: 0,
        response_size: 0,
      };

      setResponse({
        ...firstResponse,
        isBatch: true,
        total: batchResult.completed,
        success: batchResult.success,
        failed: batchResult.failed,
        completed: batchResult.completed,
        cancelled: batchResult.cancelled,
        avgDuration: Math.round(avgDuration),
        minDuration: batchResult.min_duration,
        maxDuration: batchResult.max_duration,
      });

      setIsLoading(false);
      setBatchProgress(null);

      if (batchResult.cancelled) {
        message.warning(`请求已取消，已完成 ${batchResult.completed} 个请求`);
      } else {
        message.success(`批量请求完成！成功: ${batchResult.success}, 失败: ${batchResult.failed}`);
      }
    });

    const unlistenError = listen<string>("batch-error", (event) => {
      console.error("❌ [Frontend] Batch error event received:", event.payload);
      message.error("批量请求失败: " + event.payload);
      setIsLoading(false);
      setBatchProgress(null);
    }).catch(err => {
      console.error("❌ [Frontend] Failed to listen to batch-error:", err);
    });

    return () => {
      console.log("🔌 [Frontend] Cleaning up event listeners...");
      unlistenProgress.then((fn) => fn()).catch(err => console.error("Cleanup error:", err));
      unlistenCompleted.then((fn) => fn()).catch(err => console.error("Cleanup error:", err));
      unlistenError.then((fn) => fn()).catch(err => console.error("Cleanup error:", err));
    };
  }, [setResponse]);

  if (!selectedTest || !project) {
    return (
      <Empty
        image={<ApiOutlined style={{ fontSize: 64 }} />}
        description={
          <Space direction="vertical" size="small">
            <Text type="secondary">还没有选择测试</Text>
            <Text type="secondary" style={{ fontSize: 12 }}>
              从左侧选择一个测试，或创建新的测试开始
            </Text>
          </Space>
        }
        style={{
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          height: "100%",
        }}
      />
    );
  }

  const currentTest =
    project.groups[selectedTest.groupIndex]?.children[selectedTest.testIndex];

  if (!currentTest) {
    return null;
  }

  const handleSendRequest = async () => {
    console.log("=".repeat(60));
    console.log("🎯 [Frontend] handleSendRequest called");
    console.log("=".repeat(60));

    setIsLoading(true);
    setBatchProgress(null); // 重置进度
    (window as any).batchStartTime = Date.now(); // 记录开始时间用于计算QPS

    try {
      const sendCount = parseInt(currentTest.send_count_ui) || 1;
      console.log(`📝 [Frontend] Send count: ${sendCount}`);
      console.log(`🔗 [Frontend] URL: ${currentTest.request.url}`);
      console.log(`📨 [Frontend] Method: ${currentTest.request.method}`);

      if (sendCount === 1) {
        // 单个请求
        console.log("➡️ [Frontend] Sending single request");
        const result = await invoke("send_http_request", {
          config: currentTest.request,
          variables: project.variables,
        });
        console.log("✅ [Frontend] Single request completed:", result);
        setResponse(result as any);
      } else {
        // 批量请求 - 高并发压力测试模式
        console.log(`🚀 [Frontend] Starting BATCH request: ${sendCount} requests`);
        console.log(`⚙️ [Frontend] Concurrent: ${batchConcurrent}, Batch size: ${batchSize}`);

        const batchConfig = {
          max_concurrent: batchConcurrent,
          batch_size: batchSize,
        };

        console.log("📦 [Frontend] Batch config:", JSON.stringify(batchConfig, null, 2));
        console.log("🔧 [Frontend] Request config:", {
          url: currentTest.request.url,
          method: currentTest.request.method,
          headers: currentTest.request.header?.length || 0,
          body: currentTest.request.body_raw?.substring(0, 50) || "none"
        });

        console.log("📡 [Frontend] Calling invoke('send_http_batch')...");

        try {
          await invoke("send_http_batch", {
            config: currentTest.request,
            variables: project.variables,
            count: sendCount,
            batch_config: batchConfig,
          });
          console.log("✅ [Frontend] invoke() returned successfully - batch started in background");
        } catch (invokeError) {
          console.error("❌ [Frontend] invoke() FAILED:", invokeError);
          console.error("❌ [Frontend] Error type:", typeof invokeError);
          console.error("❌ [Frontend] Error details:", JSON.stringify(invokeError, null, 2));
          throw invokeError;
        }

        console.log("⏳ [Frontend] Waiting for events (batch-progress, batch-completed)...");
        // 注意：不要在这里 setIsLoading(false)，等待 batch-completed 事件
        return; // 提前返回，避免执行 finally 块
      }
    } catch (error: any) {
      console.error("=".repeat(60));
      console.error("❌❌❌ [Frontend] CAUGHT ERROR ❌❌❌");
      console.error("=".repeat(60));
      console.error("Error:", error);
      console.error("Error message:", error?.message);
      console.error("Error stack:", error?.stack);
      console.error("Error type:", typeof error);
      console.error("Error string:", String(error));
      console.error("=".repeat(60));
      message.error("发送请求失败: " + String(error));
    } finally {
      console.log("🏁 [Frontend] handleSendRequest finally block");
      setIsLoading(false);
      setBatchProgress(null); // 清除进度
    }
  };

  const handleCancelRequest = async () => {
    try {
      await invoke("cancel_batch_request");
      message.info("正在取消请求...");
    } catch (error) {
      console.error("取消请求失败:", error);
      message.error("取消请求失败: " + error);
    }
  };

  const updateRequest = (field: string, value: any) => {
    const updatedTest = {
      ...currentTest,
      request: {
        ...currentTest.request,
        [field]: value,
      },
    };
    updateTest(selectedTest.groupIndex, selectedTest.testIndex, updatedTest);
  };

  const updateTestField = (field: string, value: any) => {
    const updatedTest = {
      ...currentTest,
      [field]: value,
    };
    updateTest(selectedTest.groupIndex, selectedTest.testIndex, updatedTest);
  };

  const tabs = [
    { id: RequestTab.Params, label: "Params" },
    { id: RequestTab.Headers, label: "Headers" },
    { id: RequestTab.Body, label: "Body" },
    { id: RequestTab.Scripts, label: "Scripts" },
    { id: RequestTab.Curl, label: "cURL" },
  ];

  const generateCurlCommand = (request: any): string => {
    let cmd = `curl -X ${request.method} '${request.url}'`;

    if (request.header?.length > 0) {
      request.header
        .filter((h: any) => !h.disable)
        .forEach((h: any) => {
          cmd += ` \\\n  -H '${h.key}: ${h.value}'`;
        });
    }

    if (request.body_raw && request.method !== "GET") {
      const escapedBody = request.body_raw.replace(/"/g, '\\"');
      cmd += ` \\\n  -d "${escapedBody}"`;
    }

    return cmd;
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
      }}
    >
      <Space.Compact>
        <Select
          value={currentTest.request.method}
          onChange={(value) => updateRequest("method", value)}
          style={{ width: 120 }}
          options={Object.values(Method).map((method) => ({
            label: method,
            value: method,
          }))}
        />

        <Input
          value={currentTest.request.url}
          onChange={(e) => updateRequest("url", e.target.value)}
          placeholder="输入 URL (支持 {{变量}} 语法)..."
          style={{ flex: 1 }}
          prefix={<LinkOutlined />}
        />

        {currentTest.request.method !== Method.WS && (
          <Space.Compact>
            <Tooltip title="批量请求次数 (1-10,000,000)">
              <InputNumber
                value={parseInt(currentTest.send_count_ui) || 1}
                onChange={(value) =>
                  updateTestField("send_count_ui", (value || 1).toString())
                }
                min={1}
                max={10000000}
                placeholder="次数"
                style={{ width: 120 }}
              />
            </Tooltip>
            {parseInt(currentTest.send_count_ui) > 1 && (
              <Tooltip title="批量请求配置">
                <Button
                  icon={<SettingOutlined />}
                  onClick={() => setBatchConfigModalVisible(true)}
                  size="small"
                  style={{ width: 32 }}
                />
              </Tooltip>
            )}
          </Space.Compact>
        )}

        <Tooltip title={isLoading ? "取消请求 (Ctrl+Enter)" : "发送请求 (Ctrl+Enter)"}>
          <Button
            type={isLoading ? "default" : "primary"}
            danger={isLoading}
            icon={isLoading ? <ClearOutlined /> : <SendOutlined />}
            onClick={isLoading ? handleCancelRequest : handleSendRequest}
            loading={false}
            disabled={!currentTest.request.url}
            style={{ minWidth: 100 }}
          >
            {isLoading ? "取消" : "发送"}
          </Button>
        </Tooltip>
      </Space.Compact>

      {/* 批量请求进度条 */}
      {batchProgress && (
        <Card size="small" style={{ marginTop: 8 }}>
          <Space direction="vertical" style={{ width: "100%" }} size="small">
            <div style={{ display: "flex", justifyContent: "space-between" }}>
              <Text strong>批量请求进度</Text>
              <Text type="secondary">
                批次 {batchProgress.current_batch}/{batchProgress.total_batches}
              </Text>
            </div>
            <Progress
              percent={Math.round((batchProgress.completed / batchProgress.total) * 100)}
              status="active"
              format={(percent) => `${percent}% (${batchProgress.completed}/${batchProgress.total})`}
            />
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12 }}>
              <Text type="success">成功: {batchProgress.success}</Text>
              <Text type="danger">失败: {batchProgress.failed}</Text>
              <Text>QPS: {Math.round(batchProgress.completed / ((Date.now() - (window as any).batchStartTime) / 1000) || 0)}</Text>
            </div>
          </Space>
        </Card>
      )}

      <AntdTabs
        activeKey={activeTab}
        onChange={(key) => setActiveTab(key as RequestTab)}
        items={tabs.map((tab) => ({
          key: tab.id,
          label: tab.label,
        }))}
      />

      {/* Params Tab */}
      {activeTab === RequestTab.Params && (
        <PairTable
          data={currentTest.request.query}
          onChange={(data) => updateRequest("query", data)}
          placeholder={{ key: "参数名", value: "参数值" }}
        />
      )}

      {/* Headers Tab */}
      {activeTab === RequestTab.Headers && (
        <PairTable
          data={currentTest.request.header}
          onChange={(data) => updateRequest("header", data)}
          placeholder={{ key: "Header 名", value: "Header 值" }}
        />
      )}

      {/* Body Tab */}
      {activeTab === RequestTab.Body && (
        <div>
          {currentTest.request.method === Method.WS ? (
            <Text type="secondary">WebSocket 消息将在连接后发送</Text>
          ) : (
            <>
              {/* Body Type Selection */}
              <Radio.Group
                value={currentTest.request.body_tab_ui}
                onChange={(e) => updateRequest("body_tab_ui", e.target.value)}
                style={{ marginBottom: 16 }}
              >
                <Radio.Button value="Raw">Raw</Radio.Button>
                <Radio.Button value="Form">Form</Radio.Button>
                <Radio.Button value="FormData">FormData</Radio.Button>
              </Radio.Group>

              {/* Raw */}
              {currentTest.request.body_tab_ui === "Raw" && (
                <div>
                  <Radio.Group
                    value={currentTest.request.body_raw_type}
                    onChange={(e) =>
                      updateRequest("body_raw_type", e.target.value)
                    }
                    style={{ marginBottom: 16 }}
                    size="small"
                  >
                    <Radio.Button value="Json">Json</Radio.Button>
                    <Radio.Button value="Text">Text</Radio.Button>
                    <Radio.Button value="XML">XML</Radio.Button>
                    <Radio.Button value="BinaryFile">BinaryFile</Radio.Button>
                  </Radio.Group>

                  <CodeEditor
                    value={currentTest.request.body_raw}
                    onChange={(value) => updateRequest("body_raw", value)}
                    language={currentTest.request.body_raw_type.toLowerCase()}
                    placeholder={`输入 ${currentTest.request.body_raw_type} 数据...`}
                    minHeight="300px"
                    showFormatButton={true}
                  />
                </div>
              )}

              {/* Form */}
              {currentTest.request.body_tab_ui === "Form" && (
                <PairTable
                  data={currentTest.request.body_form}
                  onChange={(data) => updateRequest("body_form", data)}
                  placeholder={{ key: "字段名", value: "字段值" }}
                />
              )}

              {/* FormData */}
              {currentTest.request.body_tab_ui === "FormData" && (
                <div>
                  <div style={{ marginBottom: 16 }}>
                    <Space>
                      <Button
                        type="primary"
                        icon={<PlusOutlined />}
                        onClick={() => {
                          const newData = [
                            ...currentTest.request.body_form_data,
                            { key: "", value: "", disable: false },
                          ];
                          updateRequest("body_form_data", newData);
                        }}
                        size="small"
                      >
                        添加字段
                      </Button>
                      {currentTest.request.body_form_data.length > 0 && (
                        <Button
                          danger
                          icon={<ClearOutlined />}
                          onClick={() => updateRequest("body_form_data", [])}
                          size="small"
                        >
                          清空
                        </Button>
                      )}
                    </Space>
                  </div>

                  {currentTest.request.body_form_data.map((item, index) => (
                    <Card key={index} size="small" style={{ marginBottom: 8 }}>
                      <Space.Compact style={{ width: "100%" }}>
                        <Checkbox
                          checked={!item.disable}
                          onChange={(e) => {
                            const newData = [
                              ...currentTest.request.body_form_data,
                            ];
                            newData[index].disable = !e.target.checked;
                            updateRequest("body_form_data", newData);
                          }}
                          style={{ width: 60, textAlign: "center" }}
                        />
                        <Input
                          value={item.key}
                          onChange={(e) => {
                            const newData = [
                              ...currentTest.request.body_form_data,
                            ];
                            newData[index].key = e.target.value;
                            updateRequest("body_form_data", newData);
                          }}
                          placeholder="字段名"
                          style={{ flex: 1 }}
                        />
                        <Input
                          value={item.value}
                          onChange={(e) => {
                            const newData = [
                              ...currentTest.request.body_form_data,
                            ];
                            newData[index].value = e.target.value;
                            updateRequest("body_form_data", newData);
                          }}
                          placeholder="值 (文件用 @路径)"
                          style={{ flex: 2 }}
                        />
                        <Tooltip title="选择文件">
                          <Button
                            icon={<FolderOpenOutlined />}
                            onClick={async () => {
                              try {
                                const selected = await open({
                                  multiple: false,
                                  directory: false,
                                });
                                if (selected && !Array.isArray(selected)) {
                                  const newData = [
                                    ...currentTest.request.body_form_data,
                                  ];
                                  newData[index].value = `@${selected}`;
                                  updateRequest("body_form_data", newData);
                                }
                              } catch (error) {
                                console.error("选择文件失败:", error);
                              }
                            }}
                          />
                        </Tooltip>
                        <Button
                          type="link"
                          danger
                          icon={<DeleteOutlined />}
                          onClick={() => {
                            const newData =
                              currentTest.request.body_form_data.filter(
                                (_, i) => i !== index
                              );
                            updateRequest("body_form_data", newData);
                          }}
                        />
                      </Space.Compact>
                    </Card>
                  ))}

                  {currentTest.request.body_form_data.length === 0 && (
                    <Empty
                      description='暂无数据，点击"添加字段"按钮添加'
                      image={Empty.PRESENTED_IMAGE_SIMPLE}
                    />
                  )}

                  <Text
                    type="secondary"
                    style={{ display: "block", marginTop: 8, fontSize: 12 }}
                  >
                    💡 提示: 点击文件夹图标选择文件，或手动输入 @文件路径 格式
                  </Text>
                </div>
              )}
            </>
          )}
        </div>
      )}

      {/* Scripts Tab */}
      {activeTab === RequestTab.Scripts && (
        <div style={{ display: "flex", height: "100%" }}>
          <div style={{ paddingRight: 16 }}>
            <Segmented
              vertical
              block
              options={[
                { label: "Pre-Request", value: "pre-request" },
                { label: "Post-Response", value: "post-response" },
              ]}
              value={scriptTab}
              onChange={(value) => setScriptTab(value as string)}
              style={{ marginBottom: 16 }}
            />

            <Button
              type="default"
              icon={<ApiOutlined />}
              onClick={() => setHelpModalVisible(true)}
              style={{ marginBottom: 16 }}
            >
              📖 脚本帮助
            </Button>
          </div>

          <div style={{ flex: 1, paddingLeft: 16 }}>
            {scriptTab === "pre-request" ? (
              <div>
                <Text strong style={{ display: "block", marginBottom: 8 }}>
                  Pre-Request Script (请求前)
                </Text>
                <Text
                  type="secondary"
                  style={{
                    display: "block",
                    marginBottom: 8,
                    fontSize: 12,
                  }}
                >
                  在发送请求前执行，可修改 URL、Headers、Body 等
                </Text>
                <CodeEditor
                  value={currentTest.request.pre_request_script}
                  onChange={(value) =>
                    updateRequest("pre_request_script", value)
                  }
                  language="javascript"
                  placeholder="// 示例: 设置变量\nvars['token'] = 'abc123';"
                  minHeight="400px"
                />
              </div>
            ) : (
              <div>
                <Text strong style={{ display: "block", marginBottom: 8 }}>
                  Post-Response Script (响应后)
                </Text>
                <Text
                  type="secondary"
                  style={{
                    display: "block",
                    marginBottom: 8,
                    fontSize: 12,
                  }}
                >
                  在收到响应后执行，可验证状态码、提取数据等
                </Text>
                <CodeEditor
                  value={currentTest.request.post_response_script}
                  onChange={(value) =>
                    updateRequest("post_response_script", value)
                  }
                  language="javascript"
                  placeholder="// 示例: 解析 JSON\nlet result = parse_json(response.body);\nconsole_log(result);"
                  minHeight="400px"
                />
              </div>
            )}
          </div>
        </div>
      )}

      {/* cURL Tab */}
      {activeTab === RequestTab.Curl && (
        <div>
          <Text strong style={{ display: "block", marginBottom: 8 }}>
            生成的 cURL 命令
          </Text>
          <CodeEditor
            value={generateCurlCommand(currentTest.request)}
            onChange={() => {}}
            language="bash"
            minHeight="200px"
          />
          <Button
            type="primary"
            icon={<CopyOutlined />}
            onClick={() => {
              navigator.clipboard.writeText(
                generateCurlCommand(currentTest.request)
              );
              message.success("cURL 命令已复制到剪贴板");
            }}
            style={{ marginTop: 16 }}
          >
            复制到剪贴板
          </Button>
        </div>
      )}

      {/* 脚本帮助弹窗 */}
      <Modal
        title="📖 脚本帮助 (Rhai语法)"
        open={helpModalVisible}
        onCancel={() => setHelpModalVisible(false)}
        footer={null}
        width={800}
        style={{ top: 20 }}
      >
        <div style={{ fontSize: 14, maxHeight: '70vh', overflowY: 'auto' }}>
          {/* 基础语法 */}
          <div style={{ marginBottom: 24 }}>
            <Text strong style={{ fontSize: 16 }}>📝 基础语法</Text>
            <div style={{ marginTop: 8, padding: 12, borderRadius: 4 }}>
              <pre style={{ margin: 0, fontSize: 12 }}>
{`// 变量声明
let name = "张三";
const age = 25;

// 条件语句
if age > 18 {
    console_log("成年");
} else {
    console_log("未成年");
}

// 函数调用
let data = parse_json(response.body);
console_log(data);`}
              </pre>
            </div>
          </div>

          {/* 可用对象 */}
          <div style={{ marginBottom: 24 }}>
            <Text strong style={{ fontSize: 16 }}>🔧 可用对象</Text>
            <div style={{ marginTop: 8 }}>
              <div style={{ marginBottom: 12 }}>
                <Text strong>request 对象 (请求前可用)</Text>
                <ul style={{ marginLeft: 20, marginTop: 4 }}>
                  <li><code>request.url</code> - 请求URL</li>
                  <li><code>request.method</code> - 请求方法 (GET/POST等)</li>
                  <li><code>request.headers</code> - 请求头对象</li>
                  <li><code>request.body</code> - 请求体内容</li>
                  <li><code>request.params</code> - URL参数对象</li>
                </ul>
              </div>

              <div style={{ marginBottom: 12 }}>
                <Text strong>response 对象 (响应后可用)</Text>
                <ul style={{ marginLeft: 20, marginTop: 4 }}>
                  <li><code>response.status</code> - HTTP状态码</li>
                  <li><code>response.body</code> - 响应体内容</li>
                  <li><code>response.headers</code> - 响应头对象</li>
                  <li><code>response.duration</code> - 响应时间(毫秒)</li>
                </ul>
              </div>

              <div>
                <Text strong>vars 对象 (环境变量)</Text>
                <ul style={{ marginLeft: 20, marginTop: 4 }}>
                  <li><code>vars["key"]</code> - 读取变量</li>
                  <li><code>vars["key"] = "value"</code> - 设置变量</li>
                </ul>
              </div>
            </div>
          </div>

          {/* 内置函数 */}
          <div style={{ marginBottom: 24 }}>
            <Text strong style={{ fontSize: 16 }}>🛠️ 内置函数</Text>

            <div style={{ marginTop: 8 }}>
              <div style={{ marginBottom: 16 }}>
                <Text strong>加密函数</Text>
                <ul style={{ marginLeft: 20, marginTop: 4 }}>
                  <li><code>md5(str)</code> - MD5哈希</li>
                  <li><code>sha256(str)</code> - SHA256哈希</li>
                  <li><code>sha512(str)</code> - SHA512哈希</li>
                  <li><code>hmac_sha256(key, data)</code> - HMAC-SHA256</li>
                </ul>
              </div>

              <div style={{ marginBottom: 16 }}>
                <Text strong>编码函数</Text>
                <ul style={{ marginLeft: 20, marginTop: 4 }}>
                  <li><code>base64_encode(str)</code> - Base64编码</li>
                  <li><code>base64_decode(str)</code> - Base64解码</li>
                  <li><code>url_encode(str)</code> - URL编码</li>
                  <li><code>url_decode(str)</code> - URL解码</li>
                  <li><code>hex_encode(str)</code> - 十六进制编码</li>
                  <li><code>hex_decode(str)</code> - 十六进制解码</li>
                </ul>
              </div>

              <div style={{ marginBottom: 16 }}>
                <Text strong>JSON函数</Text>
                <ul style={{ marginLeft: 20, marginTop: 4 }}>
                  <li><code>parse_json(str)</code> - 解析JSON字符串</li>
                  <li><code>to_json(obj)</code> - 对象转JSON字符串</li>
                  <li><code>json_stringify(obj)</code> - 美化JSON字符串</li>
                  <li><code>is_valid_json(str)</code> - 验证JSON格式</li>
                </ul>
              </div>

              <div style={{ marginBottom: 16 }}>
                <Text strong>工具函数</Text>
                <ul style={{ marginLeft: 20, marginTop: 4 }}>
                  <li><code>random()</code> - 生成随机数(0-1000000)</li>
                  <li><code>random_string(len)</code> - 生成随机字符串</li>
                  <li><code>timestamp()</code> - 当前时间戳(秒)</li>
                  <li><code>timestamp_ms()</code> - 当前时间戳(毫秒)</li>
                  <li><code>uuid()</code> - 生成UUID v4</li>
                </ul>
              </div>

              <div style={{ marginBottom: 16 }}>
                <Text strong>文件操作</Text>
                <ul style={{ marginLeft: 20, marginTop: 4 }}>
                  <li><code>read_file(path)</code> - 读取文件内容</li>
                  <li><code>write_file(path, content)</code> - 写入文件</li>
                  <li><code>append_file(path, content)</code> - 追加到文件</li>
                  <li><code>file_exists(path)</code> - 检查文件是否存在</li>
                  <li><code>delete_file(path)</code> - 删除文件</li>
                  <li><code>read_file_bytes(path)</code> - 读取文件为Base64</li>
                  <li><code>write_file_bytes(path, base64)</code> - 写入Base64数据</li>
                  <li><code>create_dir(path)</code> - 创建目录</li>
                  <li><code>list_files(path)</code> - 列出目录文件</li>
                </ul>
              </div>

              <div style={{ marginBottom: 16 }}>
                <Text strong>网络请求</Text>
                <ul style={{ marginLeft: 20, marginTop: 4 }}>
                  <li><code>http_get(url)</code> - GET请求(文本)</li>
                  <li><code>http_get_bytes(url)</code> - GET请求(二进制)</li>
                  <li><code>http_post(url, body)</code> - POST请求(JSON)</li>
                  <li><code>http_request(url, method, body, headers)</code> - 完整HTTP请求</li>
                </ul>
              </div>

              <div>
                <Text strong>控制台输出</Text>
                <ul style={{ marginLeft: 20, marginTop: 4 }}>
                  <li><code>console_log(msg)</code> - 输出日志(支持所有类型)</li>
                </ul>
              </div>
            </div>
          </div>

          {/* 使用示例 */}
          <div>
            <Text strong style={{ fontSize: 16 }}>💡 使用示例</Text>
            <div style={{ marginTop: 8, padding: 12, borderRadius: 4 }}>
              <pre style={{ margin: 0, fontSize: 12 }}>
{`// Pre-Request Script 示例
// 设置认证头
vars["token"] = "Bearer " + base64_encode("user:pass");
request.headers["Authorization"] = vars["token"];

// 添加时间戳参数
request.params["timestamp"] = timestamp();

// Post-Response Script 示例
// 解析JSON响应
let data = parse_json(response.body);
console_log("响应数据: " + to_json(data));

// 检查响应状态
if response.status == 200 {
    console_log("请求成功");
    // 保存数据到变量
    vars["user_id"] = data["id"];
} else {
    console_log("请求失败: " + response.status);
}

// 文件操作示例
let config = read_file("config.json");
let config_data = parse_json(config);
console_log("配置文件: " + json_stringify(config_data));`}
              </pre>
            </div>
          </div>
        </div>
      </Modal>

      {/* 批量请求配置弹窗 */}
      <Modal
        title="⚙️ 批量请求配置"
        open={batchConfigModalVisible}
        onCancel={() => setBatchConfigModalVisible(false)}
        footer={[
          <Button key="cancel" onClick={() => setBatchConfigModalVisible(false)}>
            取消
          </Button>,
          <Button
            key="ok"
            type="primary"
            onClick={() => setBatchConfigModalVisible(false)}
          >
            确定
          </Button>,
        ]}
        width={600}
      >
        <div style={{ padding: 16 }}>
          <Text type="secondary" style={{ display: "block", marginBottom: 16 }}>
            ⚡ <strong>压力测试配置</strong> - 这是专业的API压力测试工具，用于测试服务器在高并发下的表现
          </Text>

          <Space direction="vertical" size="large" style={{ width: "100%" }}>
            <Card title="🚀 并发控制" size="small">
              <Space direction="vertical" style={{ width: "100%" }}>
                <div>
                  <Text strong>最大并发数: </Text>
                  <Text type="secondary">同时发送的请求数量，越大压力越大</Text>
                </div>
                <InputNumber
                  min={1}
                  max={50000}
                  value={batchConcurrent}
                  onChange={(value) => setBatchConcurrent(value || 1000)}
                  style={{ width: "100%" }}
                  addonAfter="个请求"
                />
                <Space direction="horizontal" size="small" style={{ marginTop: 8 }}>
                  <Button size="small" onClick={() => setBatchConcurrent(100)}>保守(100)</Button>
                  <Button size="small" onClick={() => setBatchConcurrent(1000)}>标准(1000)</Button>
                  <Button size="small" onClick={() => setBatchConcurrent(5000)}>激进(5000)</Button>
                  <Button size="small" onClick={() => setBatchConcurrent(10000)}>极限(10000)</Button>
                </Space>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  🔥 <strong>压力测试建议:</strong> 100-1000正常测试，1000-5000高压测试，5000+极限压测
                </Text>
              </Space>
            </Card>

            <Card title="📦 批次处理" size="small">
              <Space direction="vertical" style={{ width: "100%" }}>
                <div>
                  <Text strong>每批大小: </Text>
                  <Text type="secondary">将大量请求分成批次，内存友好</Text>
                </div>
                <InputNumber
                  min={100}
                  max={100000}
                  value={batchSize}
                  onChange={(value) => setBatchSize(value || 10000)}
                  style={{ width: "100%" }}
                  addonAfter="个请求/批"
                />
                <Space direction="horizontal" size="small" style={{ marginTop: 8 }}>
                  <Button size="small" onClick={() => setBatchSize(5000)}>小批(5000)</Button>
                  <Button size="small" onClick={() => setBatchSize(10000)}>中批(10000)</Button>
                  <Button size="small" onClick={() => setBatchSize(50000)}>大批(50000)</Button>
                </Space>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  💡 <strong>内存优化:</strong> 现在只保存第一个响应，可以放心使用大批次
                </Text>
              </Space>
            </Card>

            <Card title="⏱️ 超时设置" size="small">
              <Space direction="vertical" style={{ width: "100%" }}>
                <div>
                  <Text strong>单个请求超时: </Text>
                  <Text type="secondary">请求的最大等待时间，超时算失败</Text>
                </div>
                <InputNumber
                  min={1}
                  max={300}
                  defaultValue={30}
                  style={{ width: "100%" }}
                  addonAfter="秒"
                />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  ⚠️ <strong>压力测试要点:</strong> 超时请求标记为失败，不会中断整个测试
                </Text>
              </Space>
            </Card>

            <Card title="📊 性能评估" size="small">
              <Space direction="vertical" size="small">
                <Text>
                  🎯 <strong>当前配置:</strong> {parseInt(currentTest.send_count_ui) || 1} 个请求
                </Text>
                <Text>
                  ⚡ <strong>预计处理时间:</strong> 约 {
                    Math.ceil((parseInt(currentTest.send_count_ui) || 1) / 2000) * 1.5
                  } 秒 (基于当前配置估算)
                </Text>
                <Text>
                  💾 <strong>内存占用:</strong> 约 {
                    Math.round((parseInt(currentTest.send_count_ui) || 1) * 2 / 1024)
                  } MB (粗略估算)
                </Text>
                <Text>
                  🔥 <strong>预期QPS:</strong> 约 {
                    Math.round(100 * 1000 / 200)
                  } (基于100并发估算)
                </Text>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  🎯 <strong>测试策略:</strong> 从小批量开始，逐步增加并发数，观察服务器崩溃点
                </Text>
              </Space>
            </Card>
          </Space>
        </div>
      </Modal>
    </div>
  );
}

export default RequestPanel;
