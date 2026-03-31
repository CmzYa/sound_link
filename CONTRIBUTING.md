# 贡献指南

感谢您考虑为 Sound Link 做出贡献！

## 如何贡献

### 报告 Bug

如果您发现了 Bug，请通过 [GitHub Issues](https://github.com/CmzYa/sound_link/issues) 报告，并包含以下信息：

- 问题的清晰描述
- 复现步骤
- 预期行为与实际行为
- 系统环境（Windows 版本、应用版本等）
- 相关截图（如有）

### 提出功能建议

我们欢迎新功能建议！请通过 GitHub Issues 提交，并描述：

- 功能的用途
- 预期的工作方式
- 可能的实现方案

### 提交代码

1. Fork 本仓库
2. 创建您的功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交您的更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开一个 Pull Request

## 开发环境设置

### 前置要求

- Windows 10/11
- Node.js (v18+)
- Rust (v1.70+)

### 安装步骤

```bash
# 克隆仓库
git clone https://github.com/CmzYa/sound_link.git
cd sound_link

# 安装依赖
npm install

# 启动开发服务器
npm run tauri dev
```

### 构建应用

```bash
npm run tauri build
```

## 代码规范

### Rust 代码

- 遵循 Rust 官方代码风格
- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码

### Vue/JavaScript 代码

- 遵循项目现有代码风格
- 使用有意义的变量名和函数名
- 添加必要的注释

## 提交信息规范

我们使用约定式提交规范：

- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码格式调整（不影响代码含义）
- `refactor`: 代码重构
- `perf`: 性能优化
- `test`: 测试相关
- `chore`: 构建过程或辅助工具的变动

示例：
```
feat: 添加音频设备搜索功能
fix: 修复路由模式下的内存泄漏问题
docs: 更新 README 中的安装说明
```

## 版本号规范

本项目使用语义化版本号规范 `X.Y.Z`：

- **X** (主版本号): 重大功能更新，不兼容的 API 变更
- **Y** (次版本号): 向后兼容的功能添加
- **Z** (修订号): 向后兼容的问题修复

## 社区

加入我们的 QQ 群与其他开发者交流：

- **QQ群号**：957468536
- **群名称**：Sound Link 交流反馈群

## 许可证

通过贡献代码，您同意您的贡献将在 [GPL-3.0 License](LICENSE) 下发布。
