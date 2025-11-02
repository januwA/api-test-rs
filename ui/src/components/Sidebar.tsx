import { useState, useRef, useEffect } from "react";
import {
  Button,
  Input,
  Typography,
  Space,
  Popconfirm,
  Dropdown,
  MenuProps,
  Card,
  Tooltip,
  Menu,
  Tag,
} from "antd";
import {
  FolderOutlined,
  EditOutlined,
  PlusOutlined,
  DeleteOutlined,
  CopyOutlined,
  MoreOutlined,
} from "@ant-design/icons";
import { useProjectStore } from "../stores/projectStore";
import { Modal as AntdModal } from "antd";
import { DesignTokens } from "../styles/tokens";

const { Text } = Typography;

function Sidebar() {
  const {
    project,
    selectedTest,
    selectTest,
    addGroup,
    addTest,
    deleteGroup,
    deleteTest,
    copyGroup,
    updateProjectName,
    updateGroupName,
    updateTestName,
    copyTest,
  } = useProjectStore();
  const [newGroupName, setNewGroupName] = useState("");
  const [expandedGroups, setExpandedGroups] = useState<Set<number>>(
    new Set([0])
  );

  // State for Project Rename Modal
  const [isRenameProjectModalOpen, setIsRenameProjectModalOpen] =
    useState(false);
  const [renameProjectInput, setRenameProjectInput] = useState("");

  // State for Group Rename Modal
  const [isRenameGroupModalOpen, setIsRenameGroupModalOpen] = useState(false);
  const [renameGroupInput, setRenameGroupInput] = useState("");
  const [currentRenamingGroupIndex, setCurrentRenamingGroupIndex] = useState<
    number | null
  >(null);

  // State for Test Rename Modal
  const [isRenameTestModalOpen, setIsRenameTestModalOpen] = useState(false);
  const [renameTestInput, setRenameTestInput] = useState("");
  const [currentRenamingTestGroupIndex, setCurrentRenamingTestGroupIndex] =
    useState<number | null>(null);
  const [currentRenamingTestIndex, setCurrentRenamingTestIndex] = useState<
    number | null
  >(null);

  // Ref for test input auto-focus
  const testInputRef = useRef<any>(null);

  // Auto-focus test input when modal opens
  useEffect(() => {
    if (isRenameTestModalOpen && testInputRef.current) {
      setTimeout(() => {
        testInputRef.current?.focus();
      }, 100);
    }
  }, [isRenameTestModalOpen]);

  const handleAddGroup = () => {
    if (newGroupName.trim()) {
      addGroup(newGroupName);
      setNewGroupName("");
    }
  };

  const handleRenameProject = () => {
    if (!project) return;
    setRenameProjectInput(project.name);
    setIsRenameProjectModalOpen(true);
  };

  const handleRenameGroup = (groupIndex: number, currentName: string) => {
    setRenameGroupInput(currentName);
    setCurrentRenamingGroupIndex(groupIndex);
    setIsRenameGroupModalOpen(true);
  };

  const handleRenameTest = (
    groupIndex: number,
    testIndex: number,
    currentName: string
  ) => {
    setRenameTestInput(currentName);
    setCurrentRenamingTestGroupIndex(groupIndex);
    setCurrentRenamingTestIndex(testIndex);
    setIsRenameTestModalOpen(true);
  };

  const handleAddTestClick = (groupIndex: number) => {
    setCurrentRenamingTestGroupIndex(groupIndex);
    setCurrentRenamingTestIndex(null);
    setRenameTestInput("");
    setIsRenameTestModalOpen(true);
  };

  const saveProjectRename = () => {
    if (
      renameProjectInput.trim() !== "" &&
      renameProjectInput !== project?.name
    ) {
      updateProjectName(renameProjectInput.trim());
    }
    setIsRenameProjectModalOpen(false);
  };

  const saveGroupRename = () => {
    if (
      currentRenamingGroupIndex !== null &&
      renameGroupInput.trim() !== "" &&
      renameGroupInput !== project?.groups[currentRenamingGroupIndex]?.name
    ) {
      updateGroupName(currentRenamingGroupIndex, renameGroupInput.trim());
    }
    setIsRenameGroupModalOpen(false);
  };

  const saveTestRename = () => {
    if (currentRenamingTestIndex !== null) {
      if (
        currentRenamingTestGroupIndex !== null &&
        renameTestInput.trim() !== "" &&
        renameTestInput !==
          project?.groups[currentRenamingTestGroupIndex]?.children[
            currentRenamingTestIndex
          ]?.name
      ) {
        updateTestName(
          currentRenamingTestGroupIndex,
          currentRenamingTestIndex,
          renameTestInput.trim()
        );
      }
    } else {
      if (
        currentRenamingTestGroupIndex !== null &&
        renameTestInput.trim() !== ""
      ) {
        addTest(currentRenamingTestGroupIndex, {
          name: renameTestInput.trim(),
          tab_ui: "Params" as any,
          send_count_ui: "1",
          request: {
            method: "GET" as any,
            url: "",
            body_tab_ui: "Raw" as any,
            query: [],
            header: [],
            body_form: [],
            body_form_data: [],
            body_raw: "",
            body_raw_type: "Json" as any,
            pre_request_script: "",
            post_response_script: "",
            script_enabled: false,
          },
        });
      }
    }
    setIsRenameTestModalOpen(false);
  };

  if (!project) {
    return (
      <div
        style={{
          padding: DesignTokens.spacing.lg,
          textAlign: "center",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <Text type="secondary">请先创建或加载项目</Text>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Header Section - 优化视觉层级 */}
      <div
        style={{
          padding: DesignTokens.spacing.lg,
        }}
      >
        {/* 项目标题区 - 增强视觉权重 */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: DesignTokens.spacing.lg,
            padding: `${DesignTokens.spacing.sm}px ${DesignTokens.spacing.md}px`,
            borderRadius: DesignTokens.borderRadius.md,
          }}
        >
          <Space>
            <FolderOutlined
              style={{
                fontSize: DesignTokens.fontSize.lg,
              }}
            />
            <Text strong style={{ fontSize: DesignTokens.fontSize.md }}>
              {project.name}
            </Text>
          </Space>
          <Tooltip title="重命名项目">
            <Button
              type="text"
              size="small"
              icon={<EditOutlined />}
              onClick={handleRenameProject}
            />
          </Tooltip>
        </div>

        {/* 添加组输入框 - 卡片样式 */}
        <Card size="small" styles={{ body: { padding: DesignTokens.spacing.sm } }}>
          <Space.Compact style={{ width: "100%" }}>
            <Input
              placeholder="新建分组..."
              value={newGroupName}
              onChange={(e) => setNewGroupName(e.target.value)}
              onPressEnter={handleAddGroup}
              prefix={<PlusOutlined />}
            />
            <Button type="primary" onClick={handleAddGroup}>
              添加
            </Button>
          </Space.Compact>
        </Card>
      </div>

      {/* Groups and Tests List */}
      <div
        style={{
          flex: 1,
          overflowY: "auto",
        }}
      >
        <Menu
          mode="inline"
          selectedKeys={
            selectedTest
              ? [`test-${selectedTest.groupIndex}-${selectedTest.testIndex}`]
              : []
          }
          openKeys={Array.from(expandedGroups).map((index) => `group-${index}`)}
          onOpenChange={(keys) => {
            const newExpanded = new Set<number>();
            keys.forEach((key) => {
              if (key.startsWith("group-")) {
                const groupIndex = parseInt(key.replace("group-", ""));
                newExpanded.add(groupIndex);
              }
            });
            setExpandedGroups(newExpanded);
          }}
          onClick={({ key }) => {
            if (key.startsWith("test-")) {
              const [, groupIndexStr, testIndexStr] = key.split("-");
              const groupIndex = parseInt(groupIndexStr);
              const testIndex = parseInt(testIndexStr);
              selectTest(groupIndex, testIndex);
            }
          }}
          style={{
            border: "none",
            background: "transparent",
          }}
          items={project.groups.map((group, groupIndex) => ({
            key: `group-${groupIndex}`,
            icon: <FolderOutlined />,
            label: (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  width: "100%",
                }}
              >
                <Space>
                  <Text strong>
                    {group.name} ({group.children?.length || 0})
                  </Text>
                </Space>
                <Space size="small">
                  <Tooltip title="复制分组">
                    <Button
                      type="text"
                      size="small"
                      icon={<CopyOutlined />}
                      onClick={(e) => {
                        e.stopPropagation();
                        copyGroup(groupIndex);
                      }}
                    />
                  </Tooltip>
                  <Tooltip title="重命名分组">
                    <Button
                      type="text"
                      size="small"
                      icon={<EditOutlined />}
                      onClick={(e) => {
                        e.stopPropagation();
                        handleRenameGroup(groupIndex, group.name);
                      }}
                    />
                  </Tooltip>
                  <Popconfirm
                    title="确定要删除这个分组吗？"
                    description="删除后无法恢复"
                    onConfirm={(e) => {
                      e?.stopPropagation();
                      deleteGroup(groupIndex);
                    }}
                    onCancel={(e) => e?.stopPropagation()}
                    okText="确定"
                    cancelText="取消"
                  >
                    <Button
                      type="text"
                      size="small"
                      danger
                      icon={<DeleteOutlined />}
                      onClick={(e) => e.stopPropagation()}
                      title="删除分组"
                    />
                  </Popconfirm>
                </Space>
              </div>
            ),
            children: [
              ...(group.children || []).map((test, testIndex) => {
                const testMenuItems: MenuProps["items"] = [
                  {
                    key: "copy",
                    icon: <CopyOutlined />,
                    label: "复制测试",
                    onClick: (e) => {
                      e.domEvent.stopPropagation();
                      copyTest(groupIndex, testIndex);
                    },
                  },
                  {
                    key: "rename",
                    icon: <EditOutlined />,
                    label: "重命名测试",
                    onClick: (e) => {
                      e.domEvent.stopPropagation();
                      handleRenameTest(groupIndex, testIndex, test.name);
                    },
                  },
                  {
                    type: "divider",
                  },
                  {
                    key: "delete",
                    icon: <DeleteOutlined />,
                    label: (
                      <Popconfirm
                        title="确定要删除这个测试吗？"
                        description="删除后无法恢复"
                        onConfirm={(e) => {
                          e?.stopPropagation();
                          deleteTest(groupIndex, testIndex);
                        }}
                        onCancel={(e) => e?.stopPropagation()}
                        okText="确定"
                        cancelText="取消"
                      >
                        删除测试
                      </Popconfirm>
                    ),
                    danger: true,
                  },
                ];

                return {
                  key: `test-${groupIndex}-${testIndex}`,
                  label: (
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        paddingRight: DesignTokens.spacing.md,
                      }}
                    >
                      <span style={{ flex: "1" }}>
                        <Tag bordered={false} color="processing">{test.request.method}</Tag>
                        {test.name}
                      </span>
                      <Dropdown
                        menu={{ items: testMenuItems }}
                        trigger={["click"]}
                        placement="bottomRight"
                      >
                        <Button
                          type="text"
                          size="small"
                          icon={<MoreOutlined />}
                          onClick={(e) => e.stopPropagation()}
                        />
                      </Dropdown>
                    </div>
                  ),
                };
              }),
              // Add Test Button
              {
                key: `add-test-${groupIndex}`,
                label: (
                  <Button
                    type="dashed"
                    size="small"
                    block
                    icon={<PlusOutlined />}
                    onClick={() => handleAddTestClick(groupIndex)}
                    style={{
                      border: "none",
                      background: "transparent",
                      marginTop: DesignTokens.spacing.xs,
                    }}
                  >
                    添加测试
                  </Button>
                ),
              },
            ],
          }))}
        />
      </div>

      {/* Modals */}
      <AntdModal
        open={isRenameProjectModalOpen}
        onCancel={() => setIsRenameProjectModalOpen(false)}
        onOk={saveProjectRename}
        title="重命名项目"
        okText="保存"
        cancelText="取消"
      >
        <Input
          value={renameProjectInput}
          onChange={(e) => setRenameProjectInput(e.target.value)}
          onPressEnter={saveProjectRename}
          placeholder="输入项目名称"
          autoFocus
        />
      </AntdModal>

      <AntdModal
        open={isRenameGroupModalOpen}
        onCancel={() => setIsRenameGroupModalOpen(false)}
        onOk={saveGroupRename}
        title="重命名分组"
        okText="保存"
        cancelText="取消"
      >
        <Input
          value={renameGroupInput}
          onChange={(e) => setRenameGroupInput(e.target.value)}
          onPressEnter={saveGroupRename}
          placeholder="输入分组名称"
          autoFocus
        />
      </AntdModal>

      <AntdModal
        open={isRenameTestModalOpen}
        onCancel={() => setIsRenameTestModalOpen(false)}
        onOk={saveTestRename}
        title={currentRenamingTestIndex !== null ? "重命名测试" : "添加测试"}
        okText="保存"
        cancelText="取消"
      >
        <Input
          ref={testInputRef}
          value={renameTestInput}
          onChange={(e) => setRenameTestInput(e.target.value)}
          onPressEnter={saveTestRename}
          placeholder="输入测试名称"
        />
      </AntdModal>
    </div>
  );
}

export default Sidebar;
