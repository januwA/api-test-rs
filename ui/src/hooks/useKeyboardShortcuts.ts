import { useEffect, useCallback } from 'react';
import { message } from 'antd';

interface KeyboardShortcutsOptions {
  onSave?: () => void;
  onNew?: () => void;
  onOpen?: () => void;
  onSend?: () => void;
  onToggleVariables?: () => void;
}

/**
 * 全局键盘快捷键 Hook
 */
export function useKeyboardShortcuts(options: KeyboardShortcutsOptions) {
  const {
    onSave,
    onNew,
    onOpen,
    onSend,
    onToggleVariables,
  } = options;

  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
      const ctrlKey = isMac ? event.metaKey : event.ctrlKey;

      // Ctrl/Cmd + S: 保存
      if (ctrlKey && event.key === 's') {
        event.preventDefault();
        if (onSave) {
          onSave();
          message.success('快捷键: Ctrl+S');
        }
        return;
      }

      // Ctrl/Cmd + N: 新建项目
      if (ctrlKey && event.key === 'n') {
        event.preventDefault();
        if (onNew) {
          onNew();
        }
        return;
      }

      // Ctrl/Cmd + O: 打开项目
      if (ctrlKey && event.key === 'o') {
        event.preventDefault();
        if (onOpen) {
          onOpen();
        }
        return;
      }

      // Ctrl/Cmd + Enter: 发送请求
      if (ctrlKey && event.key === 'Enter') {
        event.preventDefault();
        if (onSend) {
          onSend();
        }
        return;
      }

      // Ctrl/Cmd + K: 打开变量面板
      if (ctrlKey && event.key === 'k') {
        event.preventDefault();
        if (onToggleVariables) {
          onToggleVariables();
        }
        return;
      }
    },
    [onSave, onNew, onOpen, onSend, onToggleVariables]
  );

  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [handleKeyDown]);
}
