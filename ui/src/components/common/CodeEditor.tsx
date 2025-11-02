import { useRef } from 'react';
import Editor from '@monaco-editor/react';
import { Button, Space, Typography, message } from 'antd';
import { FormatPainterOutlined } from '@ant-design/icons';
import { useThemeStore } from '../../stores/themeStore';
import { DesignTokens } from '../../styles/tokens';

const { Text } = Typography;

interface CodeEditorProps {
  value: string;
  onChange: (value: string) => void;
  language?: string;
  placeholder?: string;
  minHeight?: string;
  readOnly?: boolean;
  showFormatButton?: boolean;
}

function CodeEditor({
  value,
  onChange,
  language = "text",
  minHeight = "200px",
  readOnly = false,
  showFormatButton = false,
}: CodeEditorProps) {
  const { theme } = useThemeStore();
  const editorRef = useRef<any>(null);

  // 确定实际使用的主题
  const getActualTheme = () => {
    if (theme === 'system') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }
    return theme;
  };

  const actualTheme = getActualTheme();
  const monacoTheme = actualTheme === 'dark' ? 'vs-dark' : 'light';

  // XML格式化函数
  const formatXML = (xml: string): string => {
    try {
      let formatted = '';
      let indent = 0;
      const tab = '  ';

      xml = xml.replace(/>\s+</g, '><').trim();

      const regex = /(<[^>]+>)/g;
      const parts = xml.split(regex);

      for (const part of parts) {
        if (part.trim() === '') continue;

        if (part.startsWith('</')) {
          indent--;
          formatted += '\n' + tab.repeat(Math.max(0, indent)) + part;
        } else if (part.startsWith('<') && !part.endsWith('/>') && !part.includes('</')) {
          formatted += '\n' + tab.repeat(indent) + part;
          indent++;
        } else if (part.startsWith('<?') || part.startsWith('<!')) {
          formatted += '\n' + part;
        } else {
          formatted += part;
        }
      }

      return formatted.trim();
    } catch (error) {
      throw new Error('XML格式化失败');
    }
  };

  // 格式化文档
  const formatDocument = () => {
    if (!editorRef.current || readOnly) return;

    const model = editorRef.current.getModel();
    if (!model) return;

    const currentValue = model.getValue();
    let formattedValue = currentValue;

    try {
      if (language === 'json') {
        const parsed = JSON.parse(currentValue);
        formattedValue = JSON.stringify(parsed, null, 2);
      } else if (language === 'xml') {
        formattedValue = formatXML(currentValue);
      }

      onChange(formattedValue);
      message.success('格式化成功');
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : '格式化失败';
      message.error(errorMsg);
    }
  };

  // 处理编辑器挂载
  const handleEditorDidMount = (editor: any) => {
    editorRef.current = editor;

    // 配置编辑器选项
    editor.updateOptions({
      fontSize: 14,
      fontFamily: 'JetBrains Mono, Consolas, Monaco, "Courier New", monospace',
      lineNumbers: 'on',
      roundedSelection: false,
      scrollBeyondLastLine: false,
      automaticLayout: true,
      minimap: { enabled: false },
      wordWrap: 'on',
      folding: true,
      foldingStrategy: 'indentation',
      showFoldingControls: 'always',
      lineDecorationsWidth: 10,
      lineNumbersMinChars: 3,
      renderWhitespace: 'selection',
      bracketPairColorization: { enabled: true },
      guides: {
        bracketPairs: true,
        indentation: true,
      },
    });

    // 只读模式配置
    if (readOnly) {
      editor.updateOptions({
        readOnly: true,
        contextmenu: false,
        quickSuggestions: false,
        parameterHints: { enabled: false },
        hover: { enabled: false },
        suggestOnTriggerCharacters: false,
      });
    }
  };

  // 处理编辑器变化
  const handleEditorChange = (value: string | undefined) => {
    onChange(value || '');
  };

  const toolbarHeight = 44;
  const editorHeight = showFormatButton && !readOnly && (language === 'json' || language === 'xml')
    ? `calc(${minHeight} - ${toolbarHeight}px)`
    : minHeight;

  return (
    <div style={{
      borderRadius: DesignTokens.borderRadius.md,
      overflow: 'hidden',
      boxShadow: DesignTokens.shadow.sm
    }}>
      {/* 工具栏 */}
      {showFormatButton && !readOnly && (language === 'json' || language === 'xml') && (
        <div style={{
          padding: `${DesignTokens.spacing.sm}px ${DesignTokens.spacing.md}px`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between'
        }}>
          <Space>
            <Button
              size="small"
              icon={<FormatPainterOutlined />}
              onClick={formatDocument}
              type="primary"
            >
              格式化 {language.toUpperCase()}
            </Button>
          </Space>
          <Text type="secondary" style={{ fontSize: DesignTokens.fontSize.xs }}>
            {language.toUpperCase()} 编辑器
          </Text>
        </div>
      )}

      {/* Monaco Editor */}
      <Editor
        height={editorHeight}
        language={language}
        value={value}
        theme={monacoTheme}
        onChange={handleEditorChange}
        onMount={handleEditorDidMount}
        options={{
          readOnly,
          fontSize: 14,
          fontFamily: 'JetBrains Mono, Consolas, Monaco, "Courier New", monospace',
          lineNumbers: 'on',
          roundedSelection: false,
          scrollBeyondLastLine: false,
          automaticLayout: true,
          minimap: { enabled: false },
          wordWrap: 'on',
          folding: true,
          foldingStrategy: 'indentation',
          showFoldingControls: 'always',
          lineDecorationsWidth: 10,
          lineNumbersMinChars: 3,
          renderWhitespace: 'selection',
          bracketPairColorization: { enabled: true },
          guides: {
            bracketPairs: true,
            indentation: true,
          },
          ...(readOnly && {
            contextmenu: false,
            quickSuggestions: false,
            parameterHints: { enabled: false },
            hover: { enabled: false },
            suggestOnTriggerCharacters: false,
          }),
        }}
        loading={
          <div style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: editorHeight,
          }}>
            <Space>
              <div style={{
                width: 24,
                height: 24,
                borderRadius: '50%',
                animation: 'spin 1s linear infinite'
              }} />
              <Text type="secondary">加载编辑器...</Text>
            </Space>
          </div>
        }
      />
    </div>
  );
}

export default CodeEditor;
