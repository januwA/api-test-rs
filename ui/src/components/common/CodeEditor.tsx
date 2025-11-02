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
  const handleEditorDidMount = (editor: any, monaco: any) => {
    editorRef.current = editor;

    // 为JavaScript语言添加Rhai脚本的智能提示
    if (language === 'javascript') {
      // 注册代码提示提供者
      monaco.languages.registerCompletionItemProvider('javascript', {
        provideCompletionItems: (model: any, position: any) => {
          const word = model.getWordUntilPosition(position);
          const range = {
            startLineNumber: position.lineNumber,
            endLineNumber: position.lineNumber,
            startColumn: word.startColumn,
            endColumn: word.endColumn,
          };

          // 内置函数提示
          const builtinFunctions = [
            // 加密函数
            { label: 'md5', kind: monaco.languages.CompletionItemKind.Function, detail: 'md5(str) - MD5哈希', insertText: 'md5(${1:str})' },
            { label: 'sha256', kind: monaco.languages.CompletionItemKind.Function, detail: 'sha256(str) - SHA256哈希', insertText: 'sha256(${1:str})' },
            { label: 'sha512', kind: monaco.languages.CompletionItemKind.Function, detail: 'sha512(str) - SHA512哈希', insertText: 'sha512(${1:str})' },
            { label: 'hmac_sha256', kind: monaco.languages.CompletionItemKind.Function, detail: 'hmac_sha256(key, data) - HMAC-SHA256', insertText: 'hmac_sha256(${1:key}, ${2:data})' },

            // 编码函数
            { label: 'base64_encode', kind: monaco.languages.CompletionItemKind.Function, detail: 'base64_encode(str) - Base64编码', insertText: 'base64_encode(${1:str})' },
            { label: 'base64_decode', kind: monaco.languages.CompletionItemKind.Function, detail: 'base64_decode(str) - Base64解码', insertText: 'base64_decode(${1:str})' },
            { label: 'url_encode', kind: monaco.languages.CompletionItemKind.Function, detail: 'url_encode(str) - URL编码', insertText: 'url_encode(${1:str})' },
            { label: 'url_decode', kind: monaco.languages.CompletionItemKind.Function, detail: 'url_decode(str) - URL解码', insertText: 'url_decode(${1:str})' },
            { label: 'hex_encode', kind: monaco.languages.CompletionItemKind.Function, detail: 'hex_encode(str) - 十六进制编码', insertText: 'hex_encode(${1:str})' },
            { label: 'hex_decode', kind: monaco.languages.CompletionItemKind.Function, detail: 'hex_decode(str) - 十六进制解码', insertText: 'hex_decode(${1:str})' },

            // JSON函数
            { label: 'parse_json', kind: monaco.languages.CompletionItemKind.Function, detail: 'parse_json(str) - 解析JSON字符串', insertText: 'parse_json(${1:str})' },
            { label: 'to_json', kind: monaco.languages.CompletionItemKind.Function, detail: 'to_json(obj) - 对象转JSON字符串', insertText: 'to_json(${1:obj})' },
            { label: 'json_stringify', kind: monaco.languages.CompletionItemKind.Function, detail: 'json_stringify(obj) - 美化JSON字符串', insertText: 'json_stringify(${1:obj})' },
            { label: 'is_valid_json', kind: monaco.languages.CompletionItemKind.Function, detail: 'is_valid_json(str) - 验证JSON格式', insertText: 'is_valid_json(${1:str})' },

            // 工具函数
            { label: 'random', kind: monaco.languages.CompletionItemKind.Function, detail: 'random() - 生成随机数', insertText: 'random()' },
            { label: 'random_string', kind: monaco.languages.CompletionItemKind.Function, detail: 'random_string(len) - 生成随机字符串', insertText: 'random_string(${1:len})' },
            { label: 'timestamp', kind: monaco.languages.CompletionItemKind.Function, detail: 'timestamp() - 当前时间戳(秒)', insertText: 'timestamp()' },
            { label: 'timestamp_ms', kind: monaco.languages.CompletionItemKind.Function, detail: 'timestamp_ms() - 当前时间戳(毫秒)', insertText: 'timestamp_ms()' },
            { label: 'uuid', kind: monaco.languages.CompletionItemKind.Function, detail: 'uuid() - 生成UUID v4', insertText: 'uuid()' },

            // 文件操作
            { label: 'read_file', kind: monaco.languages.CompletionItemKind.Function, detail: 'read_file(path) - 读取文件内容', insertText: 'read_file(${1:path})' },
            { label: 'write_file', kind: monaco.languages.CompletionItemKind.Function, detail: 'write_file(path, content) - 写入文件', insertText: 'write_file(${1:path}, ${2:content})' },
            { label: 'append_file', kind: monaco.languages.CompletionItemKind.Function, detail: 'append_file(path, content) - 追加到文件', insertText: 'append_file(${1:path}, ${2:content})' },
            { label: 'file_exists', kind: monaco.languages.CompletionItemKind.Function, detail: 'file_exists(path) - 检查文件是否存在', insertText: 'file_exists(${1:path})' },
            { label: 'delete_file', kind: monaco.languages.CompletionItemKind.Function, detail: 'delete_file(path) - 删除文件', insertText: 'delete_file(${1:path})' },
            { label: 'read_file_bytes', kind: monaco.languages.CompletionItemKind.Function, detail: 'read_file_bytes(path) - 读取文件为Base64', insertText: 'read_file_bytes(${1:path})' },
            { label: 'write_file_bytes', kind: monaco.languages.CompletionItemKind.Function, detail: 'write_file_bytes(path, base64) - 写入Base64数据', insertText: 'write_file_bytes(${1:path}, ${2:base64})' },
            { label: 'create_dir', kind: monaco.languages.CompletionItemKind.Function, detail: 'create_dir(path) - 创建目录', insertText: 'create_dir(${1:path})' },
            { label: 'list_files', kind: monaco.languages.CompletionItemKind.Function, detail: 'list_files(path) - 列出目录文件', insertText: 'list_files(${1:path})' },

            // 网络请求
            { label: 'http_get', kind: monaco.languages.CompletionItemKind.Function, detail: 'http_get(url) - GET请求(文本)', insertText: 'http_get(${1:url})' },
            { label: 'http_get_bytes', kind: monaco.languages.CompletionItemKind.Function, detail: 'http_get_bytes(url) - GET请求(二进制)', insertText: 'http_get_bytes(${1:url})' },
            { label: 'http_post', kind: monaco.languages.CompletionItemKind.Function, detail: 'http_post(url, body) - POST请求(JSON)', insertText: 'http_post(${1:url}, ${2:body})' },
            { label: 'http_request', kind: monaco.languages.CompletionItemKind.Function, detail: 'http_request(url, method, body, headers) - 完整HTTP请求', insertText: 'http_request(${1:url}, ${2:method}, ${3:body}, ${4:headers})' },

            // 控制台输出
            { label: 'console_log', kind: monaco.languages.CompletionItemKind.Function, detail: 'console_log(msg) - 输出日志', insertText: 'console_log(${1:msg})' },
          ];

          // 对象属性提示
          const objectProperties = [
            // request对象属性
            { label: 'request.url', kind: monaco.languages.CompletionItemKind.Property, detail: 'request.url - 请求URL', insertText: 'request.url' },
            { label: 'request.method', kind: monaco.languages.CompletionItemKind.Property, detail: 'request.method - 请求方法', insertText: 'request.method' },
            { label: 'request.headers', kind: monaco.languages.CompletionItemKind.Property, detail: 'request.headers - 请求头对象', insertText: 'request.headers' },
            { label: 'request.body', kind: monaco.languages.CompletionItemKind.Property, detail: 'request.body - 请求体内容', insertText: 'request.body' },
            { label: 'request.params', kind: monaco.languages.CompletionItemKind.Property, detail: 'request.params - URL参数对象', insertText: 'request.params' },

            // response对象属性
            { label: 'response.status', kind: monaco.languages.CompletionItemKind.Property, detail: 'response.status - HTTP状态码', insertText: 'response.status' },
            { label: 'response.body', kind: monaco.languages.CompletionItemKind.Property, detail: 'response.body - 响应体内容', insertText: 'response.body' },
            { label: 'response.headers', kind: monaco.languages.CompletionItemKind.Property, detail: 'response.headers - 响应头对象', insertText: 'response.headers' },
            { label: 'response.duration', kind: monaco.languages.CompletionItemKind.Property, detail: 'response.duration - 响应时间(毫秒)', insertText: 'response.duration' },

            // vars对象
            { label: 'vars', kind: monaco.languages.CompletionItemKind.Variable, detail: 'vars - 环境变量对象', insertText: 'vars' },
          ];

          // 关键字提示
          const keywords = [
            { label: 'let', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'let - 声明变量', insertText: 'let ${1:name} = ${2:value};' },
            { label: 'const', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'const - 声明常量', insertText: 'const ${1:name} = ${2:value};' },
            { label: 'if', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'if - 条件语句', insertText: 'if ${1:condition} {\n\t${2}\n}' },
            { label: 'else', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'else - 条件语句', insertText: 'else {\n\t${1}\n}' },
            { label: 'while', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'while - 循环语句', insertText: 'while ${1:condition} {\n\t${2}\n}' },
            { label: 'for', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'for - 循环语句', insertText: 'for ${1:item} in ${2:collection} {\n\t${3}\n}' },
            { label: 'break', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'break - 跳出循环', insertText: 'break;' },
            { label: 'continue', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'continue - 继续下次循环', insertText: 'continue;' },
            { label: 'return', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'return - 返回值', insertText: 'return ${1:value};' },
            { label: 'true', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'true - 布尔真值', insertText: 'true' },
            { label: 'false', kind: monaco.languages.CompletionItemKind.Keyword, detail: 'false - 布尔假值', insertText: 'false' },
          ];

          // 合并所有提示项
          const suggestions = [
            ...builtinFunctions.map(item => ({ ...item, range })),
            ...objectProperties.map(item => ({ ...item, range })),
            ...keywords.map(item => ({ ...item, range })),
          ];

          return { suggestions };
        }
      });

      // 注册悬停提示提供者
      monaco.languages.registerHoverProvider('javascript', {
        provideHover: (model: any, position: any) => {
          const word = model.getWordAtPosition(position);
          if (!word) return null;

          const hoverTexts: { [key: string]: string } = {
            // 函数悬停提示
            'md5': 'md5(str: string): string\n计算字符串的MD5哈希值',
            'sha256': 'sha256(str: string): string\n计算字符串的SHA256哈希值',
            'parse_json': 'parse_json(str: string): object\n解析JSON字符串为对象',
            'console_log': 'console_log(msg: any): void\n输出消息到控制台',
            'timestamp': 'timestamp(): number\n获取当前时间戳(秒)',
            'random': 'random(): number\n生成0-1000000之间的随机数',

            // 对象属性悬停提示
            'request': 'request: object\n当前请求的对象，包含url, method, headers等属性',
            'response': 'response: object\n当前响应的对象，包含status, body, headers等属性',
            'vars': 'vars: object\n环境变量对象，用于存储和读取变量',
          };

          const hoverText = hoverTexts[word.word];
          if (hoverText) {
            return {
              contents: [
                { value: '```rhai\n' + hoverText + '\n```' }
              ]
            };
          }

          return null;
        }
      });
    }

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
