# Sound Link

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
   npm run dev
   ```

4. 构建应用
   ```bash
   npm run build
   npm run tauri build
   ```

## 项目结构

```
sound-link/
├── src/             # 前端源代码
│   ├── components/  # Vue 组件
│   ├── App.vue      # 主应用组件
│   ├── Settings.vue # 设置页面组件
│   ├── main.js      # 前端入口
│   └── style.css    # 全局样式
├── src-tauri/       # Tauri 后端代码
│   ├── src/         # Rust 源代码
│   ├── icons/       # 应用图标
│   └── tauri.conf.json # Tauri 配置
├── index.html       # 主 HTML 文件
├── package.json     # 项目配置和依赖
└── README.md        # 项目说明
```

## 贡献

欢迎提交 Issue 和 Pull Request 来帮助改进这个项目！

## 许可证

MIT License