// 应用版本号 — 单一来源（从 package.json 读取，与 tauri.conf.json / Cargo.toml 保持一致）
import pkg from "../../package.json";

export const APP_VERSION: string = pkg.version;
