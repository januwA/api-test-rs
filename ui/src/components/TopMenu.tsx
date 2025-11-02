import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  Button,
  Space,
  Typography,
  message,
  Divider,
  Tooltip,
  Modal,
  Form,
  Input,
  Dropdown,
  MenuProps,
} from "antd";
import {
  FileAddOutlined,
  FolderOpenOutlined,
  FolderOutlined,
  SaveOutlined,
  SunOutlined,
  MoonOutlined,
  DesktopOutlined,
  DownOutlined,
} from "@ant-design/icons";
import { useProjectStore } from "../stores/projectStore";
import { useThemeStore } from "../stores/themeStore";
import { ProjectWithPath, Project } from "../types";
import { DesignTokens } from "../styles/tokens";

const { Text } = Typography;

function TopMenu() {
  const { project, projectPath, setProject, saveProject, saveProjectAs } =
    useProjectStore();
  const { theme, toggleTheme } = useThemeStore();

  // 新建项目 Modal
  const [isNewProjectModalOpen, setIsNewProjectModalOpen] = useState(false);
  const [newProjectName, setNewProjectName] = useState("");

  const themeIcons: Record<string, React.ReactNode> = {
    light: <SunOutlined />,
    dark: <MoonOutlined />,
    system: <DesktopOutlined />,
  };

  const themeLabels: Record<string, string> = {
    light: "浅色",
    dark: "深色",
    system: "跟随系统",
  };

  // 静默保存（Ctrl+S）
  const handleSaveProject = async () => {
    if (!project) {
      message.warning("没有可保存的项目");
      return;
    }

    // 如果有路径，静默保存；否则提示另存为
    if (projectPath) {
      try {
        await saveProject();
      } catch (error) {
        // 错误已在 store 中处理
      }
    } else {
      message.info("请选择保存位置");
      handleSaveProjectAs();
    }
  };

  // 另存为
  const handleSaveProjectAs = async () => {
    if (!project) {
      message.warning("没有可保存的项目");
      return;
    }

    try {
      const savePath = await save({
        title: "另存为",
        defaultPath: `${project.name}.json`,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });

      if (savePath) {
        await saveProjectAs(savePath);
      }
    } catch (error) {
      console.error("保存项目失败:", error);
      message.error(`保存失败: ${error}`);
    }
  };

  const handleLoadProject = async () => {
    try {
      const selected = await open({
        title: "加载项目",
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });

      if (selected) {
        const result = await invoke<ProjectWithPath>("load_project", {
          path: selected,
        });
        // 设置项目和路径，重置选择
        setProject(result.project, result.path, true);
        message.success("项目加载成功");
      }
    } catch (error) {
      console.error("加载项目失败:", error);
      message.error(`加载失败: ${error}`);
    }
  };

  const handleNewProject = () => {
    setNewProjectName("");
    setIsNewProjectModalOpen(true);
  };

  const confirmNewProject = async () => {
    if (newProjectName.trim()) {
      try {
        const newProject = await invoke<Project>("create_project", {
          name: newProjectName.trim(),
        });
        // 新项目没有路径，重置选择
        setProject(newProject, null, true);
        message.success("新项目创建成功");
        setIsNewProjectModalOpen(false);
        setNewProjectName("");
      } catch (error) {
        console.error("创建新项目失败:", error);
        message.error(`创建失败: ${error}`);
      }
    }
  };

  // 保存按钮下拉菜单
  const saveMenuItems: MenuProps["items"] = [
    {
      key: "save",
      icon: <SaveOutlined />,
      label: "保存",
      onClick: handleSaveProject,
    },
    {
      key: "saveAs",
      icon: <FolderOutlined />,
      label: "另存为...",
      onClick: handleSaveProjectAs,
    },
  ];

  return (
    <>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          minHeight: DesignTokens.size.headerHeight,
        }}
      >
        {/* 左侧：主要操作区 */}
        <Space size="middle">
          {/* 文件操作组 */}
          <Space.Compact>
            <Tooltip title="新建项目 (Ctrl+N)">
              <Button icon={<FileAddOutlined />} onClick={handleNewProject}>
                新建
              </Button>
            </Tooltip>
            <Tooltip title="打开项目 (Ctrl+O)">
              <Button icon={<FolderOpenOutlined />} onClick={handleLoadProject}>
                打开
              </Button>
            </Tooltip>
          </Space.Compact>

          <Divider type="vertical" style={{ height: 28, margin: 0 }} />

          {/* 保存按钮组 - 下拉菜单 */}
          <Dropdown.Button
            type="primary"
            icon={<DownOutlined />}
            onClick={handleSaveProject}
            menu={{ items: saveMenuItems }}
            disabled={!project}
          >
            <SaveOutlined /> 保存
          </Dropdown.Button>
        </Space>

        {/* 右侧：信息区 */}
        <Space
          size="large"
          split={<Divider type="vertical" style={{ margin: 0 }} />}
        >
          {/* 项目名称和状态 */}
          {project ? (
            <Space>
              {projectPath && <Text type="secondary">(已保存)</Text>}
              {!projectPath && <Text type="warning">(未保存)</Text>}
            </Space>
          ) : (
            <Text type="secondary">无项目</Text>
          )}

          {/* 主题切换 */}
          <Tooltip title={`切换主题 (当前: ${themeLabels[theme]})`}>
            <Button
              type="primary"
              icon={themeIcons[theme]}
              onClick={toggleTheme}
            />
          </Tooltip>
        </Space>
      </div>

      {/* 新建项目 Modal */}
      <Modal
        title="新建项目"
        open={isNewProjectModalOpen}
        onOk={confirmNewProject}
        onCancel={() => {
          setIsNewProjectModalOpen(false);
          setNewProjectName("");
        }}
        okText="创建"
        cancelText="取消"
        okButtonProps={{ disabled: !newProjectName.trim() }}
      >
        <Form layout="vertical" style={{ marginTop: DesignTokens.spacing.lg }}>
          <Form.Item
            label="项目名称"
            required
            tooltip="为你的 API 测试项目起个好记的名字"
          >
            <Input
              value={newProjectName}
              onChange={(e) => setNewProjectName(e.target.value)}
              placeholder="例如：我的 API 测试项目"
              onPressEnter={confirmNewProject}
              autoFocus
              maxLength={50}
              showCount
            />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
}

export default TopMenu;
