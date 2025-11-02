import { useState } from "react";
import {
  Typography,
  Tag,
  Space,
  Button,
  Card,
  Row,
  Col,
  Statistic,
  Segmented,
  message,
  Empty,
} from "antd";
import { CopyOutlined, RocketOutlined } from "@ant-design/icons";
import { useResponseStore } from "../stores/responseStore";
import { ResponseTab } from "../types";
import { Tabs as AntdTabs } from "antd";
import CodeEditor from "./common/CodeEditor";

const { Text } = Typography;

function ResponsePanel() {
  const { currentResponse } = useResponseStore();
  const [activeTab, setActiveTab] = useState<ResponseTab>(ResponseTab.Data);
  const [headerTab, setHeaderTab] = useState<string>("request");

  const tabs = [
    { id: ResponseTab.Data, label: "Data" },
    { id: ResponseTab.Header, label: "Headers" },
    { id: ResponseTab.Stats, label: "Stats" },
  ];

  if (!currentResponse) {
    return (
      <Empty
        image={<RocketOutlined style={{ fontSize: 64 }} />}
        description={
          <Space direction="vertical" size="small">
            <Text type="secondary">等待响应数据</Text>
            <Text type="secondary" style={{ fontSize: 12 }}>
              发送请求后，响应数据将在这里显示
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

  const getContentType = (headersStr: string): string => {
    const lines = headersStr.split("\n");
    for (const line of lines) {
      const [key, value] = line.split(": ");
      if (key?.toLowerCase() === "content-type") {
        return value?.split(";")[0]?.toLowerCase() || "";
      }
    }
    return "";
  };

  const contentType = getContentType(currentResponse.headers_str);

  const isImage = contentType.startsWith("image/");
  const isVideo = contentType.startsWith("video/");
  const isAudio = contentType.startsWith("audio/");

  const getBase64Data = (): string => {
    if (!currentResponse.data_vec) return "";
    const uint8Array = new Uint8Array(currentResponse.data_vec);
    const binaryString = Array.from(uint8Array, (byte) =>
      String.fromCharCode(byte)
    ).join("");
    return btoa(binaryString);
  };

  const responseData = currentResponse.data_vec
    ? new TextDecoder().decode(new Uint8Array(currentResponse.data_vec))
    : "";

  const { formattedData, language } = (() => {
    if (isImage || isVideo || isAudio) {
      return { formattedData: responseData, language: "text" };
    }

    try {
      const parsed = JSON.parse(responseData);
      return {
        formattedData: JSON.stringify(parsed, null, 2),
        language: "json",
      };
    } catch {
      const trimmedData = responseData.trim();
      if (
        trimmedData.startsWith("<?xml") ||
        (trimmedData.startsWith("<") && trimmedData.includes("</"))
      ) {
        return { formattedData: responseData, language: "xml" };
      } else if (trimmedData.startsWith("<") && trimmedData.includes("<html")) {
        return { formattedData: responseData, language: "html" };
      } else if (
        trimmedData.includes("function") ||
        trimmedData.includes("var ") ||
        trimmedData.includes("const ")
      ) {
        return { formattedData: responseData, language: "javascript" };
      } else {
        return { formattedData: responseData, language: "text" };
      }
    }
  })();

  const getStatusColor = (status: number): string => {
    if (status >= 200 && status < 300) return "success";
    if (status >= 300 && status < 400) return "warning";
    if (status >= 400 && status < 500) return "error";
    return "error";
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Status Bar */}
      <div style={{ padding: 16 }}>
        <Space size="large">
          {currentResponse.isBatch ? (
            <>
              <Space>
                <Text type="secondary">批量请求:</Text>
                <Tag
                  color="processing"
                  style={{ fontSize: 16, padding: "2px 12px" }}
                >
                  {currentResponse.success}/{currentResponse.total} 成功
                </Tag>
              </Space>
              <Space>
                <Text type="secondary">平均响应时间:</Text>
                <Text strong>{currentResponse.avgDuration?.toFixed(2)} ms</Text>
              </Space>
              <Space>
                <Text type="secondary">总大小:</Text>
                <Text strong>
                  {currentResponse.responses?.reduce(
                    (sum, r) => sum + r.response_size,
                    0
                  ) / 1024 || 0}{" "}
                  KB
                </Text>
              </Space>
            </>
          ) : (
            <>
              <Space>
                <Text type="secondary">Status:</Text>
                <Tag
                  color={getStatusColor(currentResponse.status)}
                  style={{ fontSize: 16, padding: "2px 12px" }}
                >
                  {currentResponse.status}
                </Tag>
              </Space>
              <Space>
                <Text type="secondary">Time:</Text>
                <Text strong>{currentResponse.duration} ms</Text>
              </Space>
              <Space>
                <Text type="secondary">Size:</Text>
                <Text strong>
                  {(currentResponse.response_size / 1024).toFixed(2)} KB
                </Text>
              </Space>
            </>
          )}
        </Space>
      </div>

      {/* Tabs */}
      <div style={{ padding: "8px 16px 0" }}>
        <AntdTabs
          activeKey={activeTab}
          onChange={(key) => setActiveTab(key as ResponseTab)}
          items={tabs.map((tab) => ({
            key: tab.id,
            label: tab.label,
          }))}
          size="small"
        />
      </div>

      {/* Content Area */}
      <div style={{ flex: 1, overflowY: "auto", padding: 16 }}>
        {/* Data Tab */}
        {activeTab === ResponseTab.Data && (
          <div>
            {currentResponse.data_vec && currentResponse.data_vec.length > 0 ? (
              <div>
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    marginBottom: 16,
                  }}
                >
                  <Space>
                    <Text type="secondary">
                      响应体大小:{" "}
                      {(currentResponse.response_size / 1024).toFixed(2)} KB
                    </Text>
                    <Text type="secondary">类型: {contentType || "未知"}</Text>
                  </Space>
                  <Button
                    icon={<CopyOutlined />}
                    size="small"
                    onClick={() => {
                      navigator.clipboard.writeText(responseData);
                      message.success("已复制到剪贴板");
                    }}
                  >
                    复制
                  </Button>
                </div>

                {isImage ? (
                  <div style={{ textAlign: "center" }}>
                    <img
                      src={`data:${contentType};base64,${getBase64Data()}`}
                      alt="Response"
                      style={{
                        maxWidth: "100%",
                        maxHeight: "500px",
                        borderRadius: 8,
                      }}
                      onError={(e) => console.error("Failed to load image:", e)}
                    />
                    <Text
                      type="secondary"
                      style={{ display: "block", marginTop: 16 }}
                    >
                      🖼️ 图片预览 ({contentType})
                    </Text>
                  </div>
                ) : isVideo ? (
                  <div style={{ textAlign: "center" }}>
                    <video
                      controls
                      style={{
                        maxWidth: "100%",
                        maxHeight: "500px",
                        borderRadius: 8,
                      }}
                      src={`data:${contentType};base64,${getBase64Data()}`}
                      onError={(e) => console.error("Failed to load video:", e)}
                    >
                      您的浏览器不支持视频播放。
                    </video>
                    <Text
                      type="secondary"
                      style={{ display: "block", marginTop: 16 }}
                    >
                      🎬 视频预览 ({contentType})
                    </Text>
                  </div>
                ) : isAudio ? (
                  <div style={{ textAlign: "center" }}>
                    <audio
                      controls
                      style={{ width: "100%", maxWidth: "500px" }}
                      src={`data:${contentType};base64,${getBase64Data()}`}
                      onError={(e) => console.error("Failed to load audio:", e)}
                    >
                      您的浏览器不支持音频播放。
                    </audio>
                    <Text
                      type="secondary"
                      style={{ display: "block", marginTop: 16 }}
                    >
                      🎵 音频预览 ({contentType})
                    </Text>
                  </div>
                ) : (
                  <CodeEditor
                    value={formattedData}
                    onChange={() => {}}
                    language={language}
                    minHeight="400px"
                    readOnly={true}
                  />
                )}
              </div>
            ) : (
              <div style={{ textAlign: "center", padding: 32 }}>
                <Text type="secondary">无响应数据</Text>
              </div>
            )}
          </div>
        )}

        {/* Headers Tab */}
        {activeTab === ResponseTab.Header && (
          <div style={{ display: "flex", height: "100%" }}>
            <div style={{ paddingRight: 16 }}>
              <Segmented
                vertical
                block
                options={[
                  { label: "Request", value: "request" },
                  { label: "Response", value: "response" },
                ]}
                value={headerTab}
                onChange={(value) => setHeaderTab(value as string)}
                style={{ marginBottom: 16 }}
              />
            </div>

            <div style={{ flex: 1, paddingLeft: 16 }}>
              {headerTab === "request" ? (
                <div>
                  <CodeEditor
                    value={currentResponse.request_headers_str}
                    onChange={() => {}}
                    language="text"
                    minHeight="400px"
                    readOnly={true}
                  />
                </div>
              ) : (
                <div>
                  <CodeEditor
                    value={currentResponse.headers_str}
                    onChange={() => {}}
                    language="text"
                    minHeight="400px"
                    readOnly={true}
                  />
                </div>
              )}
            </div>
          </div>
        )}

        {/* Stats Tab */}
        {activeTab === ResponseTab.Stats && (
          <div>
            {currentResponse.isBatch ? (
              // 批量请求统计
              <div>
                <Row gutter={[16, 16]}>
                  <Col span={8}>
                    <Card title="📊 批量请求统计" size="small">
                      <Statistic
                        title="总请求数"
                        value={currentResponse.total}
                      />
                      <Statistic
                        title="成功请求"
                        value={currentResponse.success}
                        suffix={`(${(
                          (currentResponse.success / currentResponse.total) *
                          100
                        ).toFixed(1)}%)`}
                      />
                      <Statistic
                        title="失败请求"
                        value={currentResponse.failed}
                        suffix={`(${(
                          (currentResponse.failed / currentResponse.total) *
                          100
                        ).toFixed(1)}%)`}
                      />
                    </Card>
                  </Col>

                  <Col span={8}>
                    <Card title="⏱️ 响应时间统计" size="small">
                      <Statistic
                        title="平均响应时间"
                        value={currentResponse.avgDuration.toFixed(2)}
                        suffix="ms"
                      />
                      <Statistic
                        title="最快响应"
                        value={currentResponse.minDuration.toFixed(2)}
                        suffix="ms"
                      />
                      <Statistic
                        title="最慢响应"
                        value={currentResponse.maxDuration.toFixed(2)}
                        suffix="ms"
                      />
                    </Card>
                  </Col>

                  <Col span={8}>
                    <Card title="📈 性能指标" size="small">
                      <Statistic
                        title="QPS (估算)"
                        value={(
                          (1000 / currentResponse.avgDuration) *
                          100
                        ).toFixed(0)}
                      />
                      <Statistic
                        title="成功率"
                        value={(
                          (currentResponse.success / currentResponse.total) *
                          100
                        ).toFixed(1)}
                        suffix="%"
                      />
                      <Statistic title="并发数" value="100" suffix="(最大)" />
                    </Card>
                  </Col>
                </Row>

                <Card
                  title="📋 详细响应列表"
                  style={{ marginTop: 16 }}
                  size="small"
                >
                  <div style={{ maxHeight: "400px", overflowY: "auto" }}>
                    {currentResponse.responses.map(
                      (response: any, index: number) => (
                        <div
                          key={index}
                          style={{
                            padding: "8px",
                            marginBottom: "4px",
                          }}
                        >
                          <Space>
                            <Text strong>#{index + 1}</Text>
                            <Tag
                              color={
                                response.status >= 200 && response.status < 300
                                  ? "success"
                                  : "error"
                              }
                            >
                              {response.status}
                            </Tag>
                            <Text>{response.duration}ms</Text>
                            <Text type="secondary" style={{ fontSize: "12px" }}>
                              {(response.response_size / 1024).toFixed(2)}KB
                            </Text>
                          </Space>
                        </div>
                      )
                    )}
                  </div>
                </Card>
              </div>
            ) : (
              // 单个请求统计
              <Row gutter={[16, 16]}>
                <Col span={12}>
                  <Card title="📊 请求信息" size="small">
                    <Statistic
                      title="HTTP 版本"
                      value={currentResponse.version}
                    />
                    <Statistic title="状态码" value={currentResponse.status} />
                    <Statistic
                      title="响应时间"
                      value={currentResponse.duration}
                      suffix="ms"
                    />
                  </Card>
                </Col>

                <Col span={12}>
                  <Card title="📦 数据传输" size="small">
                    <Statistic
                      title="请求大小"
                      value={(currentResponse.request_size / 1024).toFixed(2)}
                      suffix="KB"
                    />
                    <Statistic
                      title="响应大小"
                      value={(currentResponse.response_size / 1024).toFixed(2)}
                      suffix="KB"
                    />
                    <Statistic
                      title="总传输"
                      value={(
                        (currentResponse.request_size +
                          currentResponse.response_size) /
                        1024
                      ).toFixed(2)}
                      suffix="KB"
                    />
                  </Card>
                </Col>
              </Row>
            )}

            {currentResponse.modified_vars &&
              currentResponse.modified_vars.length > 0 && (
                <Card
                  title="🔧 脚本修改的变量"
                  style={{ marginTop: 16 }}
                  size="small"
                >
                  {currentResponse.modified_vars.map((v, i) => (
                    <div key={i} style={{ marginBottom: 8 }}>
                      <Text type="secondary">{v.key}: </Text>
                      <Tag>{v.value}</Tag>
                    </div>
                  ))}
                </Card>
              )}
          </div>
        )}
      </div>
    </div>
  );
}

export default ResponsePanel;
