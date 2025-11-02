import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ConfigProvider, theme as antdTheme, message } from "antd";
import zhCN from "antd/locale/zh_CN";
import { useProjectStore } from "./stores/projectStore";
import { useThemeStore } from "./stores/themeStore";
import { ProjectWithPath, Project } from "./types";
import Layout from "./components/Layout";

function App() {
  const { setProject, saveProject, project } = useProjectStore();
  const { theme } = useThemeStore();

  // 自动保存功能 - 10分钟间隔
  useEffect(() => {
    if (!project) return;

    const autoSaveInterval = setInterval(() => {
      saveProject().catch((error) => {
        console.error("自动保存失败:", error);
      });
    }, 10 * 60 * 1000);

    return () => clearInterval(autoSaveInterval);
  }, [project, saveProject]);

  useEffect(() => {
    // 初始化应用,尝试加载上次打开的项目
    const initApp = async () => {
      try {
        const result = await invoke<ProjectWithPath | null>("get_project");
        if (result) {
          // 项目已加载，同时设置项目和路径
          console.log("加载上次打开的项目:", result.path);
          setProject(result.project, result.path, false);
        } else {
          // 创建默认项目（没有路径）
          const defaultProject = await invoke<Project>("create_project", {
            name: "Default Project",
          });
          setProject(defaultProject, null, false);
        }
      } catch (error) {
        console.error("Failed to initialize app:", error);
        message.error(`初始化失败: ${error}`);
      }
    };

    initApp();
  }, [setProject]);

  // 确定实际使用的主题（处理 system 主题）
  const getActualTheme = () => {
    if (theme === 'system') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }
    return theme;
  };

  const actualTheme = getActualTheme();

  return (
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: actualTheme === 'dark' ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
        token: {
          colorPrimary: '#3b82f6', // blue-500
          borderRadius: 6,
        },
      }}
    >
      <Layout />
    </ConfigProvider>
  );
}

export default App;
