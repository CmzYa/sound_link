# Sound Link

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

可视化音频设备切换工具

## 项目介绍

Sound Link 是一个基于 Tauri 和 Vue 3 开发的跨平台音频设备管理工具，允许用户通过直观的界面查看和切换系统音频设备。

## 功能特点

- 可视化显示系统音频设备
- 一键切换音频输入/输出设备
- 设备状态实时监控
- 简洁美观的用户界面
- 跨平台支持（Windows、macOS、Linux）

## 技术栈

- 前端：Vue 3 + Vite
- 后端：Rust + Tauri
- 图标：Lucide Vue Next

## 安装与使用

### 前置要求

- Node.js (v16+)
- Rust (v1.60+)
- Tauri CLI

### 开发环境设置

1. 克隆仓库
   ```bash
   git clone https://github.com/yourusername/sound-link.git
   cd sound-link
   ```

2. 安装依赖
   ```bash
   npm install
   ```

3. 启动开发服务器
   ```bash
   npm run tauri dev
   ```

4. 构建应用
   ```bash
   npm run tauri build
   ```

## 项目结构

```
sound-link/
├── src/                # 前端源代码
│   ├── components/     # Vue 组件
│   │   └── DeviceBall.vue
│   ├── views/          # 视图组件
│   │   ├── MainView.vue
│   │   └── SettingsView.vue
│   ├── styles/         # 样式文件
│   │   └── main.css
│   ├── index.html      # 主 HTML 文件
│   └── main.js         # 前端入口
├── src-tauri/          # Tauri 后端代码
│   ├── src/            # Rust 源代码
│   │   └── main.rs
│   ├── icons/          # 应用图标
│   ├── capabilities/   # Tauri 权限配置
│   └── tauri.conf.json # Tauri 配置
├── package.json        # 项目配置和依赖
├── vite.config.js      # Vite 配置
└── README.md           # 项目说明
```

## 贡献

我们欢迎所有形式的贡献！

- 🐛 提交 Bug 报告
- 💡 提出新功能建议
- 📝 改进文档
- 🔧 提交 Pull Request

## 许可证

本项目基于 [MIT License](LICENSE) 开源。
