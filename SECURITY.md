# 安全策略 / Security Policy

## 支持的版本

| 版本 | 支持状态 |
|------|---------|
| 0.1.x | ✅ 活跃支持 |

## 报告漏洞

如果您发现安全漏洞，请**不要**通过公开 Issue 报告。请通过以下渠道私密报告：

- 📧 Email: cnkevinbai@gmail.com
- 🔐 建议使用 PGP 加密（如有）

我们会尽快（通常在 48 小时内）确认并给出修复时间线。

## 安全设计

Chronos-Shadow 的安全架构包括：

- **API Key 存储**：Windows Credential Manager 原生加密（`CredWriteW` / `CredReadW`）
- **会话加密**：AES-256-GCM 分块流式加密
- **沙盒隔离**：文件系统操作受 C-VFS 沙盒约束
- **防幻觉**：三红线拦截器（Schema 校验 + 路径拦截 + 自愈熔断）
- **依赖审计**：内置 GPL/AGPL 传染性协议检测

## 致谢

我们将在 README 中致谢负责任披露漏洞的安全研究人员（经报告者同意）。
