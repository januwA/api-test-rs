import { useState, useEffect, useRef } from "react";
import { Layout as AntdLayout } from "antd";
import Sidebar from "./Sidebar";
import RequestPanel from "./RequestPanel";
import ResponsePanel from "./ResponsePanel";
import TopMenu from "./TopMenu";
import VariablesPanel from "./VariablesPanel";
import { DesignTokens } from "../styles/tokens";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import { useProjectStore } from "../stores/projectStore";

const { Header, Content } = AntdLayout;

function Layout() {
  const [sidebarWidth, setSidebarWidth] = useState<number>(DesignTokens.size.sidebarDefault);
  const [isResizingHorizontal, setIsResizingHorizontal] = useState(false);
  const [requestPanelHeight, setRequestPanelHeight] = useState<number>(50); // 百分比
  const [isResizingVertical, setIsResizingVertical] = useState(false);
  const contentAreaRef = useRef<HTMLDivElement>(null);

  const { saveProject, project, projectPath } = useProjectStore();

  // 键盘快捷键处理
  const handleSave = async () => {
    if (!project) return;
    if (projectPath) {
      try {
        await saveProject();
      } catch (error) {
        // 错误已在store中处理
      }
    }
  };

  // 使用键盘快捷键
  useKeyboardShortcuts({
    onSave: handleSave,
  });

  const handleHorizontalMouseDown = () => {
    setIsResizingHorizontal(true);
  };

  const handleVerticalMouseDown = () => {
    setIsResizingVertical(true);
  };

  useEffect(() => {
    const handleGlobalMouseMove = (e: MouseEvent) => {
      // 水平拖拽（侧边栏宽度）
      if (isResizingHorizontal) {
        const newWidth = e.clientX;
        if (newWidth >= DesignTokens.size.sidebarMin && newWidth <= DesignTokens.size.sidebarMax) {
          setSidebarWidth(newWidth);
        }
      }

      // 垂直拖拽（Request/Response 面板高度）
      if (isResizingVertical && contentAreaRef.current) {
        const rect = contentAreaRef.current.getBoundingClientRect();
        const percentage = ((e.clientY - rect.top) / rect.height) * 100;
        if (percentage >= 20 && percentage <= 80) {
          setRequestPanelHeight(percentage);
        }
      }
    };

    const handleGlobalMouseUp = () => {
      setIsResizingHorizontal(false);
      setIsResizingVertical(false);
    };

    if (isResizingHorizontal || isResizingVertical) {
      document.addEventListener("mousemove", handleGlobalMouseMove);
      document.addEventListener("mouseup", handleGlobalMouseUp);
      // 防止拖拽时选中文本
      document.body.style.userSelect = 'none';
      document.body.style.cursor = isResizingHorizontal ? 'col-resize' : 'row-resize';
    } else {
      document.body.style.userSelect = '';
      document.body.style.cursor = '';
    }

    return () => {
      document.removeEventListener("mousemove", handleGlobalMouseMove);
      document.removeEventListener("mouseup", handleGlobalMouseUp);
      document.body.style.userSelect = '';
      document.body.style.cursor = '';
    };
  }, [isResizingHorizontal, isResizingVertical]);

  return (
    <AntdLayout style={{ height: '100vh' }}>
      <Header style={{
        padding: 0,
        height: 'auto',
        lineHeight: 'normal',
        paddingLeft: 12,
        paddingRight: 12,
      }}>
        <TopMenu />
      </Header>

      <AntdLayout>
        <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
          {/* 侧边栏 */}
          <div style={{
            width: sidebarWidth,
          }}>
            <Sidebar />
          </div>

          {/* 水平拖动分隔符 */}
          <div
            style={{
              width: DesignTokens.size.dragHandleSize,
              cursor: 'col-resize',
              background: isResizingHorizontal
                ? 'var(--ant-color-primary)'
                : 'transparent',
              transition: DesignTokens.transition.normal,
              position: 'relative',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
            onMouseDown={handleHorizontalMouseDown}
            onMouseEnter={(e) => {
              if (!isResizingHorizontal) {
                e.currentTarget.style.background = 'var(--ant-color-primary-bg)';
              }
            }}
            onMouseLeave={(e) => {
              if (!isResizingHorizontal) {
                e.currentTarget.style.background = 'transparent';
              }
            }}
          >
            {/* 三个点视觉提示 */}
            <div style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 2,
              pointerEvents: 'none'
            }}>
              {[1, 2, 3].map(i => (
                <div key={i} style={{
                  width: 3,
                  height: 3,
                  borderRadius: '50%',
                }} />
              ))}
            </div>
          </div>

          {/* 右侧内容区 */}
          <AntdLayout style={{ flex: 1 }}>
            <Content
              ref={contentAreaRef}
              style={{
                display: 'flex',
                flexDirection: 'column',
                overflow: 'hidden',
              }}
            >
              {/* Request Panel */}
              <div style={{
                height: `${requestPanelHeight}%`,
                overflow: 'auto',
              }}>
                <RequestPanel />
              </div>

              {/* 垂直拖拽分隔符 */}
              <div
                style={{
                  height: DesignTokens.size.dragHandleSize,
                  cursor: 'row-resize',
                  background: isResizingVertical
                    ? 'var(--ant-color-primary)'
                    : 'transparent',
                  borderTop: '1px solid var(--ant-color-border)',
                  borderBottom: '1px solid var(--ant-color-border)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  transition: DesignTokens.transition.normal
                }}
                onMouseDown={handleVerticalMouseDown}
                onMouseEnter={(e) => {
                  if (!isResizingVertical) {
                    e.currentTarget.style.background = 'var(--ant-color-primary-bg)';
                  }
                }}
                onMouseLeave={(e) => {
                  if (!isResizingVertical) {
                    e.currentTarget.style.background = 'transparent';
                  }
                }}
              >
                {/* 水平三个点 */}
                <div style={{
                  display: 'flex',
                  gap: 2,
                  pointerEvents: 'none'
                }}>
                  {[1, 2, 3].map(i => (
                    <div key={i} style={{
                      width: 3,
                      height: 3,
                      borderRadius: '50%',
                      background: 'var(--ant-color-text-tertiary)'
                    }} />
                  ))}
                </div>
              </div>

              {/* Response Panel */}
              <div style={{
                height: `${100 - requestPanelHeight}%`,
                overflow: 'auto',
                background: 'var(--ant-color-bg-container)'
              }}>
                <ResponsePanel />
              </div>
            </Content>
          </AntdLayout>
        </div>
      </AntdLayout>

      <VariablesPanel />
    </AntdLayout>
  );
}

export default Layout;
