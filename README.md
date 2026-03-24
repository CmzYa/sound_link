# Sound Link

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows-blue.svg)](https://www.microsoft.com/windows)

可视化音频设备切换工具

## 项目介绍

Sound Link 是一个基于 Tauri 和 Vue 3 开发的 Windows 音频设备管理工具，允许用户通过直观的界面查看和切换系统音频设备。

## 功能特点

- 可视化显示系统音频设备
- 一键切换音频输出设备
- 设备状态实时监控
- 简洁美观的用户界面
- 系统托盘常驻
- 跟随系统主题（深色/浅色模式）
- 跟随系统主题色
- 高级毛玻璃材质效果

## 技术栈

- 前端：Vue 3 + Vite
- 后端：Rust + Tauri 2
- 图标：Lucide Vue Next

## 安装与使用

### 前置要求

- Windows 10/11
- Node.js (v18+)
- Rust (v1.70+)

### 安装 AudioDeviceCmdlets 模块

本软件依赖 [AudioDeviceCmdlets](https://github.com/frgnca/AudioDeviceCmdlets) PowerShell 模块来控制音频设备。

以管理员身份运行 PowerShell，执行以下命令：

```powershell
Install-Module -Name AudioDeviceCmdlets -Force
```

### 开发环境设置

1. 克隆仓库
   ```bash
   git clone https://github.com/CmzYa/sound_link.git
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
│   │   ├── main.rs     # 主入口
│   │   └── devices/    # 设备管理模块
│   │       ├── mod.rs  # 模块定义
│   │       └── audio.rs # 音频设备管理
│   ├── icons/          # 应用图标
│   ├── capabilities/   # Tauri 权限配置
│   └── tauri.conf.json # Tauri 配置
├── .github/            # GitHub 配置
│   └── workflows/      # GitHub Actions
│       └── release.yml # 自动发布工作流
├── package.json        # 项目配置和依赖
├── vite.config.js      # Vite 配置
└── README.md           # 项目说明
```

## 发布

项目使用 GitHub Actions 自动构建和发布。创建新的版本标签即可触发自动构建：

```bash
git tag v1.0.0
git push origin v1.0.0
```

也可以在 GitHub Actions 页面手动触发构建。

## 贡献

我们欢迎所有形式的贡献！

- 🐛 提交 Bug 报告
- 💡 提出新功能建议
- 📝 改进文档
- 🔧 提交 Pull Request

## 许可证

本项目基于 [MIT License](LICENSE) 开源。
