# Chronos-Shadow 端侧资源目录

## privacy_mask.onnx
端侧 CV 隐私遮罩模型。用于在截图发往云端前，对聊天窗口、密码框、网银等敏感区域进行本地像素级打码。

模型规格：
- 格式：ONNX (Open Neural Network Exchange)
- 输入：待检测的屏幕截图 (640×640 RGB)
- 输出：敏感区域边界框 [x, y, width, height, confidence]
- 推荐模型：轻量级目标检测 (NanoDet-Plus / YOLOv8-nano) 转 ONNX
- 预期大小：< 5MB
- 推理引擎：Rust tract-onnx 或 onnxruntime-rs

放置位置：将此文件放在 src-tauri/resources/privacy_mask.onnx
Tauri 构建时自动打包到应用资源中。
