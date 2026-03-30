# Sound Link

[!\[License: GPL v3\](https://img.shields.io/badge/License-GPLv3-blue.svg null)](https://www.gnu.org/licenses/gpl-3.0)
[!\[Platform: Windows\](https://img.shields.io/badge/Platform-Windows-blue.svg null)](https://www.microsoft.com/windows)

可视化音频设备切换与路由工具

## 项目介绍

Sound Link 是一个基于 Tauri 和 Vue 3 开发的 Windows 音频设备管理工具，允许用户通过直观的界面查看和切换系统音频设备，并支持音频路由功能。

## 功能特点

- 可视化显示系统音频设备
- 一键切换音频输出设备
- 设备状态实时监控
- 简洁美观的用户界面
- 系统托盘常驻
- 跟随系统主题（深色/浅色模式）
- 跟随系统主题色
- 高级毛玻璃材质效果
- **音频路由功能**：将系统音频广播到多个设备

## 音频路由功能

音频路由功能允许你将系统音频同时输出到多个设备，实现多设备同步播放。

> **注意**：路由模式可能导致轻微的音频延迟。同时广播到过多设备可能导致卡顿和较高的 CPU 占用，建议目标设备数量不超过 8 个。

### 关于虚拟音频设备

**重要声明**：本软件的代码中**未使用 VB-Cable 的任何专有代码或功能**。音频路由功能仅需要一个虚拟音频设备作为音频中转输出点。理论上，任何可用的音频输出设备都可以作为中转设备，但物理输出设备（如耳机、扬声器）会产生实际声音输出，导致延迟不可调和回音问题，因此**使用虚拟音频设备**作为中转。

### 工作原理

1. 使用虚拟音频设备作为音频源
2. 启动路由时自动将系统默认输出切换到虚拟设备
3. 从虚拟设备捕获音频并广播到选定的目标设备
4. 停止路由时自动恢复原默认设备

### 安装 VB-Cable

音频路由功能推荐使用 [VB-Cable](https://vb-audio.com/Cable/) 虚拟音频设备作为中转。

1. 访问 [VB-Audio 官网](https://vb-audio.com/Cable/)
2. 下载并安装 VB-Cable
3. 重启应用即可使用音频路由功能

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

**首次启动时会自动检测并安装该模块**，由于需要从 PowerShell Gallery 下载，首次启动可能较慢，请耐心等待。

如需手动安装，以管理员身份运行 PowerShell，执行以下命令：

```powershell
Install-Module -Name AudioDeviceCmdlets -Force
```

### 使用教程

详细的使用教程请参考 [使用教程](USAGE.md) 文件：

- 快速开始指南
- 基本操作说明
- 高级功能使用
- 常见问题解决
- 最佳实践建议

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
│   ├── index.html      # 主 HTML 文件
│   └── main.js         # 前端入口
├── src-tauri/          # Tauri 后端代码
│   ├── src/            # Rust 源代码
│   │   ├── main.rs     # 主入口
│   │   ├── devices/    # 设备管理模块
│   │   │   ├── mod.rs  # 模块定义
│   │   │   └── audio.rs # 音频设备管理
│   │   └── router/     # 音频路由模块
│   │       ├── mod.rs  # 模块定义
│   │       ├── router.rs # 路由核心逻辑
│   │       ├── delay_buffer.rs # 延迟缓冲
│   │       └── volume_mixer.rs # 音量混合
│   ├── icons/          # 应用图标
│   ├── capabilities/   # Tauri 权限配置
│   └── tauri.conf.json # Tauri 配置
├── .github/            # GitHub 配置
│   └── workflows/      # GitHub Actions
│       └── release.yml # 自动发布工作流
├── package.json        # 项目配置和依赖
├── vite.config.js      # Vite 配置
├── README.md           # 项目说明
└── USAGE.md            # 使用教程
```

## 贡献

我们欢迎所有形式的贡献！

- 🐛 提交 Bug 报告
- 💡 提出新功能建议
- 📝 改进文档
- 🔧 提交 Pull Request

## 社区交流

加入我们的 QQ 群，与其他用户和开发者交流：

- **QQ群号**：957468536
- **群名称**：Sound Link ✨ 交流反馈群

## 许可证

本项目基于 [GPL-3.0 License](LICENSE) 开源。
