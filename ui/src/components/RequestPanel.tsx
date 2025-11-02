import { useState } from "react";
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
} from "antd";
import { SendOutlined, CopyOutlined, LinkOutlined, ApiOutlined, FolderOpenOutlined, PlusOutlined, ClearOutlined, DeleteOutlined } from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../stores/projectStore";
import { useResponseStore } from "../stores/responseStore";
import { Method, RequestTab } from "../types";
import { Tabs as AntdTabs } from "antd";
import PairTable from "./common/PairTable";
import CodeEditor from "./common/CodeEditor";
import { DesignTokens } from "../styles/tokens";

const { Text } = Typography;

function RequestPanel() {
  const { project, selectedTest, updateTest } =
    useProjectStore();
  const { setResponse } = useResponseStore();
  const [activeTab, setActiveTab] = useState<RequestTab>(RequestTab.Params);
  const [scriptTab, setScriptTab] = useState<string>("pre-request");
  const [isLoading, setIsLoading] = useState(false);
  const [helpModalVisible, setHelpModalVisible] = useState(false);

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
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
          height: '100%'
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
    setIsLoading(true);
    try {
      const sendCount = parseInt(currentTest.send_count_ui) || 1;

      if (sendCount === 1) {
        // 单个请求
        const result = await invoke("send_http_request", {
          config: currentTest.request,
          variables: project.variables,
        });
        setResponse(result as any);
      } else {
        // 批量请求
        const results = await invoke("send_http_batch", {
          config: currentTest.request,
          variables: project.variables,
          count: sendCount,
        });

        // 创建批量请求的统计信息
        const batchStats = {
          total: sendCount,
          success: 0,
          failed: 0,
          responses: results as any[],
          avgDuration: 0,
          minDuration: Infinity,
          maxDuration: 0,
        };

        (results as any[]).forEach((response: any) => {
          if (response.status >= 200 && response.status < 300) {
            batchStats.success++;
          } else {
            batchStats.failed++;
          }
          batchStats.avgDuration += response.duration;
          batchStats.minDuration = Math.min(batchStats.minDuration, response.duration);
          batchStats.maxDuration = Math.max(batchStats.maxDuration, response.duration);
        });

        batchStats.avgDuration = batchStats.avgDuration / sendCount;

        setResponse({
          ...batchStats,
          isBatch: true,
          status: batchStats.failed === 0 ? 200 : 500,
          duration: batchStats.avgDuration,
          response_size: 0,
          request_size: 0,
          data_vec: null,
          headers_str: "",
          request_headers_str: "",
          version: "HTTP/1.1",
          modified_vars: [],
        });
      }
    } catch (error) {
      console.error("发送请求失败:", error);
      message.error("发送请求失败: " + error);
    } finally {
      setIsLoading(false);
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
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
    }}>
      {/* Request Line - 卡片包裹 */}
      <Card
        size="small"
        style={{
          margin: DesignTokens.spacing.lg,
          boxShadow: DesignTokens.shadow.sm
        }}
        bodyStyle={{ padding: DesignTokens.spacing.md }}
      >
        <Space.Compact style={{ width: "100%" }} size="large">
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
          )}

          <Tooltip title="发送请求 (Ctrl+Enter)">
            <Button
              type="primary"
              icon={<SendOutlined />}
              onClick={handleSendRequest}
              loading={isLoading}
              disabled={!currentTest.request.url}
              style={{ minWidth: 100 }}
            >
              {isLoading ? "发送中" : "发送"}
            </Button>
          </Tooltip>
        </Space.Compact>
      </Card>

      {/* Tabs */}
      <div
        style={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        <div style={{ padding: `0 ${DesignTokens.spacing.lg}px` }}>
          <AntdTabs
            activeKey={activeTab}
            onChange={(key) => setActiveTab(key as RequestTab)}
            items={tabs.map(tab => ({
              key: tab.id,
              label: tab.label,
            }))}
          />
        </div>

        <div style={{
          flex: 1,
          overflowY: "auto",
          padding: DesignTokens.spacing.lg,
        }}>
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
                    onChange={(e) =>
                      updateRequest("body_tab_ui", e.target.value)
                    }
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
                        <Radio.Button value="BinaryFile">
                          BinaryFile
                        </Radio.Button>
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
                              const newData = [...currentTest.request.body_form_data, { key: "", value: "", disable: false }];
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
                                const newData = [...currentTest.request.body_form_data];
                                newData[index].disable = !e.target.checked;
                                updateRequest("body_form_data", newData);
                              }}
                              style={{ width: 60, textAlign: "center" }}
                            />
                            <Input
                              value={item.key}
                              onChange={(e) => {
                                const newData = [...currentTest.request.body_form_data];
                                newData[index].key = e.target.value;
                                updateRequest("body_form_data", newData);
                              }}
                              placeholder="字段名"
                              style={{ flex: 1 }}
                            />
                            <Input
                              value={item.value}
                              onChange={(e) => {
                                const newData = [...currentTest.request.body_form_data];
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
                                      const newData = [...currentTest.request.body_form_data];
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
                                const newData = currentTest.request.body_form_data.filter((_, i) => i !== index);
                                updateRequest("body_form_data", newData);
                              }}
                            />
                          </Space.Compact>
                        </Card>
                      ))}

                      {currentTest.request.body_form_data.length === 0 && (
                        <Empty description='暂无数据，点击"添加字段"按钮添加' image={Empty.PRESENTED_IMAGE_SIMPLE} />
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
                      style={{ display: "block", marginBottom: 8, fontSize: 12 }}
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
                      style={{ display: "block", marginBottom: 8, fontSize: 12 }}
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
        </div>
      </div>

      {/* 脚本帮助弹窗 */}
      <Modal
        title="📖 脚本帮助"
        open={helpModalVisible}
        onCancel={() => setHelpModalVisible(false)}
        footer={null}
        width={600}
      >
        <div style={{ fontSize: 14 }}>
          <div style={{ marginBottom: 16 }}>
            <Text strong>可用对象:</Text>
            <ul style={{ marginLeft: 20, marginTop: 8 }}>
              <li>
                <code>request.url</code>, <code>request.method</code>, <code>request.headers</code>
              </li>
              <li>
                <code>response.status</code>, <code>response.body</code>, <code>response.duration</code>
              </li>
              <li><code>vars</code> - 环境变量对象</li>
            </ul>
          </div>
          <div>
            <Text strong>常用函数:</Text>
            <ul style={{ marginLeft: 20, marginTop: 8 }}>
              <li><code>parse_json(str)</code> - 解析 JSON</li>
              <li>
                <code>md5(str)</code>, <code>sha256(str)</code>, <code>hmac_sha256(key, data)</code>
              </li>
              <li><code>base64_encode(str)</code>, <code>base64_decode(str)</code></li>
              <li><code>timestamp()</code>, <code>uuid()</code>, <code>random_string(len)</code></li>
              <li><code>console_log(msg)</code> - 输出日志</li>
            </ul>
          </div>
        </div>
      </Modal>
    </div>
  );
}

export default RequestPanel;
