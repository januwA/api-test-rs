# API Test - Tauri版本

这是使用 **Tauri 2.0 + React + TypeScript** 重构的 API 测试工具,核心逻辑保持不变。

## 项目结构

```
api-test-rs/
├── src-tauri/           # Tauri Rust 后端
│   ├── src/
│   │   ├── main.rs      # Tauri 应用入口
│   │   ├── lib.rs       # 核心业务逻辑
│   │   ├── commands.rs  # Tauri Commands API
│   │   ├── util.rs      # 工具函数
│   │   └── script_engine.rs  # Rhai 脚本引擎
│   ├── Cargo.toml       # Rust 依赖配置
│   ├── tauri.conf.json  # Tauri 配置
│   └── build.rs         # 构建脚本
│
├── ui/                  # React 前端
│   ├── src/
│   │   ├── components/  # React 组件
│   │   ├── stores/      # Zustand 状态管理
│   │   ├── types/       # TypeScript 类型定义
│   │   └── styles/      # 样式文件
│   └── index.html
│
├── package.json         # 前端依赖配置
├── tsconfig.json        # TypeScript 配置
└── vite.config.ts       # Vite 配置
```

## 技术栈

### 后端 (Rust)
- **Tauri 2.0** - 跨平台桌面应用框架
- **Tokio** - 异步运行时
- **Reqwest** - HTTP 客户端
- **Rhai** - 嵌入式脚本引擎

### 前端 (React)
- **React 18** - UI 框架
- **TypeScript** - 类型安全
- **Vite** - 构建工具
- **Tailwind CSS** - 样式框架

## 开发和构建

### 安装依赖
```bash
npm install
```

### 开发模式
```bash
npm run tauri:dev
```

### 构建发布版
```bash
npm run tauri:build
```
