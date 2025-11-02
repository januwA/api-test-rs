import { useState } from "react";
import { Drawer, FloatButton, Typography, Button, Collapse, Space } from "antd";
import { SettingOutlined, PlusOutlined } from "@ant-design/icons";
import { useProjectStore } from "../stores/projectStore";
import PairTable from "./common/PairTable";

const { Text } = Typography;

function VariablesPanel() {
  const { project, addVariable, setProject } = useProjectStore();
  const [showPanel, setShowPanel] = useState(false);

  if (!project) {
    return null;
  }

  return (
    <>
      {/* Float Button */}
      <FloatButton
        icon={<SettingOutlined />}
        tooltip="变量管理"
        type="primary"
        style={{ right: 24, bottom: 24 }}
        onClick={() => setShowPanel(true)}
      />

      {/* Drawer Panel */}
      <Drawer
        title={
          <Space>
            <SettingOutlined />
            <span>环境变量</span>
          </Space>
        }
        placement="right"
        width={480}
        onClose={() => setShowPanel(false)}
        open={showPanel}
      >
        <div style={{ marginBottom: 16 }}>
          <Text type="secondary" style={{ fontSize: 12 }}>
            <div style={{ marginBottom: 8 }}>💡 提示:</div>
            <ul style={{ marginLeft: 20, paddingLeft: 0 }}>
              <li style={{ marginBottom: 4 }}>
                在 URL、Headers、Body 中使用{" "}
                <code style={{ padding: "2px 6px", borderRadius: 4 }}>
                  {"{{变量名}}"}
                </code>{" "}
                语法引用变量，变量名不需要引号
              </li>
              <li>
                脚本中通过{" "}
                <code style={{ padding: "2px 6px", borderRadius: 4 }}>
                  vars['变量名']
                </code>{" "}
                读写变量，变量名用引号包围
              </li>
            </ul>
          </Text>
        </div>

        <PairTable
          data={project.variables}
          onChange={(newVars) => {
            setProject({
              ...project,
              variables: newVars,
            });
          }}
          placeholder={{ key: "变量名", value: "变量值" }}
        />

        {/* Common Variable Templates */}
        <Collapse
          style={{ marginTop: 16 }}
          items={[
            {
              key: "templates",
              label: "📋 常用变量模板",
              children: (
                <Space direction="vertical" style={{ width: "100%" }}>
                  <Button
                    block
                    type="text"
                    style={{ textAlign: "left" }}
                    onClick={() => {
                      addVariable({
                        key: "baseUrl",
                        value: "https://api.example.com",
                        disable: false,
                      });
                    }}
                  >
                    <PlusOutlined /> baseUrl (基础URL)
                  </Button>
                  <Button
                    block
                    type="text"
                    style={{ textAlign: "left" }}
                    onClick={() => {
                      addVariable({ key: "token", value: "", disable: false });
                    }}
                  >
                    <PlusOutlined /> token (认证令牌)
                  </Button>
                  <Button
                    block
                    type="text"
                    style={{ textAlign: "left" }}
                    onClick={() => {
                      addVariable({
                        key: "userId",
                        value: "12345",
                        disable: false,
                      });
                    }}
                  >
                    <PlusOutlined /> userId (用户ID)
                  </Button>
                </Space>
              ),
            },
          ]}
        />
      </Drawer>
    </>
  );
}

export default VariablesPanel;
