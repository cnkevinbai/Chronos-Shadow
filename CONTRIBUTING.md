# Contributing to Chronos-Shadow

感谢您对 Chronos-Shadow 的关注！我们欢迎任何形式的贡献。

## 开发环境

```bash
# 克隆仓库
git clone https://github.com/cnkevinbai/Chronos-Shadow.git
cd chronos-shadow

# 安装依赖
npm install

# 启动开发模式
npm run tauri dev

# 运行检查
npm run build          # TypeScript + Vite
npm run lint           # Oxlint
cargo check            # Rust
```

## 分支策略

- `main` — 稳定发布分支
- `dev` — 开发分支
- `feature/*` — 新功能
- `fix/*` — Bug 修复

## 提交规范

使用 [Conventional Commits](https://conventionalcommits.org)：

```
feat: 添加 AES-256-GCM 会话加密
fix: 修复托盘图标崩溃
docs: 更新 README
refactor: 重构 detector.rs
test: 添加 vault 单元测试
```

## 代码风格

- **TypeScript**: `tsc --strict` + Oxlint
- **Rust**: `cargo fmt` + `cargo clippy`
- **Tailwind**: 遵循项目现有暗色主题规范

## Pull Request 流程

1. Fork 仓库，从 `dev` 分支创建 feature 分支
2. 编写代码，确保 `npm run build` 和 `cargo check` 通过
3. 提交 PR 到 `dev` 分支
4. 等待 Code Review

## 目录约定

```
src/views/          # 面板级组件（每个面板一个文件）
src/components/     # 可复用组件
src/lib/            # IPC 层、类型定义、i18n、工具函数
src-tauri/src/agent/# Rust 核心模块
src-tauri/skills/   # Skill JSON 定义
```

## 新增 Tauri 命令

1. 在 `src-tauri/src/agent/` 下实现命令函数
2. 在 `src-tauri/src/agent/mod.rs` 注册模块
3. 在 `src-tauri/src/lib.rs` 注册到 `invoke_handler`
4. 在 `src/lib/tauri.ts` 添加 TypeScript IPC 封装

## 许可证

贡献代码即表示您同意在 Apache 2.0 许可证下发布您的贡献。
