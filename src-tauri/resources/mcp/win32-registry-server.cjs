#!/usr/bin/env node
// Win32 Registry Sensor — Windows 注册表与系统环境传感器 (MCP stdio server)
// 协议: MCP JSON-RPC 2.0 over stdio
// 工具: query_registry_value / write_environment_variable / list_installed_applications

'use strict';

const readline = require('readline');
const { execSync, execFileSync } = require('child_process');

const SERVER_INFO = { name: 'mcp-server-win32-registry', version: '1.0.0' };

const TOOLS = [
  {
    name: 'query_registry_value',
    description: '读取 Windows 注册表指定路径下的键值，检测软件安装路径、版本号等系统信息。',
    inputSchema: {
      type: 'object',
      properties: {
        hive: { type: 'string', enum: ['HKLM', 'HKCU', 'HKCR', 'HKU'], description: '注册表根键' },
        path: { type: 'string', description: "注册表子键路径，如 'SOFTWARE\\Python\\PythonCore\\3.12\\InstallPath'" },
        value_name: { type: 'string', description: '键值名称，默认读取 (Default)' },
      },
      required: ['hive', 'path'],
    },
  },
  {
    name: 'write_environment_variable',
    description: '向用户/系统环境变量注入新路径，并广播 WM_SETTINGCHANGE 实现免重启即时生效。',
    inputSchema: {
      type: 'object',
      properties: {
        scope: { type: 'string', enum: ['user', 'system'], description: '作用域' },
        name: { type: 'string', description: "环境变量名，如 'JAVA_HOME'" },
        value: { type: 'string', description: "环境变量值，如 'C:\\Program Files\\Java\\jdk-21'" },
      },
      required: ['scope', 'name', 'value'],
    },
  },
  {
    name: 'list_installed_applications',
    description: '扫描注册表 Uninstall 键，枚举已安装的应用程序列表及版本信息。',
    inputSchema: { type: 'object', properties: {} },
  },
];

function queryRegistryValue(args) {
  const { hive, path: keyPath, value_name } = args;
  const fullKey = `${hive}\\${keyPath}`;
  const cmdArgs = ['query', fullKey];
  if (value_name) cmdArgs.push('/v', value_name);
  else cmdArgs.push('/ve');
  const out = execFileSync('reg', cmdArgs, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  // 解析 "    ValueName    REG_SZ    value" 行
  const lines = out.split(/\r?\n/);
  const parsed = [];
  for (const l of lines) {
    const m = l.trim().match(/^(.+?)\s+(REG_\w+)\s+(.*)$/);
    if (m) parsed.push({ value_name: m[1].trim(), type: m[2], value: m[3].trim() });
  }
  return { key: fullKey, values: parsed };
}

function writeEnvironmentVariable(args) {
  const { scope, name, value } = args;
  const isSystem = scope === 'system';
  if (isSystem) {
    // 系统级需管理员权限
    execFileSync('setx', [name, value, '/M'], { stdio: ['ignore', 'ignore', 'pipe'] });
  } else {
    execFileSync('setx', [name, value], { stdio: ['ignore', 'ignore', 'pipe'] });
  }
  return { scope, name, value, applied: true, note: 'setx 已写入；广播 WM_SETTINGCHANGE 使新进程即时生效' };
}

function listInstalledApplications() {
  const roots = [
    'HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall',
    'HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall',
    'HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall',
  ];
  const apps = [];
  for (const root of roots) {
    try {
      const out = execFileSync('reg', ['query', root, '/s', '/f', 'DisplayName', '/t', 'REG_SZ'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] });
      // 提取 DisplayName / DisplayVersion / InstallLocation
      const entries = out.split(/\r?\n/);
      let current = null;
      for (const l of entries) {
        const keyMatch = l.trim().match(/^HKEY_[^ ]+/);
        if (keyMatch) {
          if (current && current.name) apps.push(current);
          current = {};
        }
        const dm = l.trim().match(/DisplayName\s+REG_SZ\s+(.*)$/);
        if (dm && current) current.name = dm[1].trim();
        const dv = l.trim().match(/DisplayVersion\s+REG_SZ\s+(.*)$/);
        if (dv && current) current.version = dv[1].trim();
      }
      if (current && current.name) apps.push(current);
    } catch { /* 某些根键可能无权限 */ }
  }
  // 去重
  const seen = new Set();
  const deduped = apps.filter(a => !seen.has(a.name) && seen.add(a.name));
  return { total: deduped.length, applications: deduped };
}

// ─── MCP stdio 循环 ──────────────────────────────────────────────
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });

function respond(id, result) {
  process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, result }) + '\n');
}
function respondError(id, code, message) {
  process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, error: { code, message } }) + '\n');
}

rl.on('line', (line) => {
  if (!line.trim()) return;
  let req;
  try { req = JSON.parse(line); } catch { return; }
  const { id, method, params } = req;
  try {
    switch (method) {
      case 'initialize':
        return respond(id, { protocolVersion: '2024-11-05', capabilities: { tools: {} }, serverInfo: SERVER_INFO });
      case 'ping':
        return respond(id, {});
      case 'tools/list':
        return respond(id, { tools: TOOLS });
      case 'tools/call': {
        const { name, arguments: args } = params || {};
        let result;
        if (name === 'query_registry_value') result = queryRegistryValue(args || {});
        else if (name === 'write_environment_variable') result = writeEnvironmentVariable(args || {});
        else if (name === 'list_installed_applications') result = listInstalledApplications();
        else return respondError(id, -32601, `Unknown tool: ${name}`);
        return respond(id, { content: [{ type: 'text', text: JSON.stringify(result) }], isError: false });
      }
      default:
        return respondError(id, -32601, `Unknown method: ${method}`);
    }
  } catch (e) {
    return respondError(id, -32603, `Internal error: ${e.message}`);
  }
});
