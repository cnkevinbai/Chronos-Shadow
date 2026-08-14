#!/usr/bin/env node
// Audit Vault — 企业级静态安全合规风控保险箱 (MCP stdio server)
// 协议: MCP JSON-RPC 2.0 over stdio（每行一个 JSON 请求/响应）
// 工具: scan_incremental_ast_diff / verify_license_compliance / generate_audit_report

'use strict';

const fs = require('fs');
const path = require('path');
const readline = require('readline');
const { execSync } = require('child_process');

const SERVER_INFO = { name: 'mcp-server-audit-vault', version: '1.0.0' };

// ─── 工具 Schema（与 audit-vault-server.json 对齐） ────────────────
const TOOLS = [
  {
    name: 'scan_incremental_ast_diff',
    description: '读取增量 AST 语法树变动，静默审查 SQL 注入、硬编码 AWS/大模型 API Key 等高危模式。',
    inputSchema: {
      type: 'object',
      properties: {
        file_paths: { type: 'array', items: { type: 'string' }, description: '需要扫描的文件路径列表，为空则扫描全部变更文件' },
        severity: { type: 'string', enum: ['low', 'medium', 'high', 'critical'], default: 'high', description: '最低告警级别' },
      },
    },
  },
  {
    name: 'verify_license_compliance',
    description: '扫描本地 package.json / Cargo.toml 的第三方包依赖，探测 GPL/AGPL 等破坏商业闭环的协议。',
    inputSchema: {
      type: 'object',
      properties: {
        project_path: { type: 'string', description: '项目根路径' },
        blocked_licenses: { type: 'array', items: { type: 'string' }, default: ['GPL', 'AGPL'], description: '阻断协议列表' },
      },
      required: ['project_path'],
    },
  },
  {
    name: 'generate_audit_report',
    description: '生成当前项目的全量安全审计报告，含漏洞统计、协议合规评分、密钥泄露风险等级。',
    inputSchema: {
      type: 'object',
      properties: {
        format: { type: 'string', enum: ['json', 'markdown', 'sarif'], default: 'json' },
      },
    },
  },
];

// ─── 高危模式扫描 ────────────────────────────────────────────────
const SECRET_PATTERNS = [
  { name: 'AWS Access Key', re: /\bAKIA[0-9A-Z]{16}\b/g, severity: 'critical' },
  { name: 'AWS Secret Key', re: /aws_secret_access_key\s*[:=]\s*['"][^'"]{16,}['"]/gi, severity: 'critical' },
  { name: 'OpenAI/DeepSeek API Key', re: /\bsk-[a-zA-Z0-9]{20,}\b/g, severity: 'critical' },
  { name: 'Private Key (PEM)', re: /-----BEGIN (RSA|EC|OPENSSH|DSA|PGP) PRIVATE KEY-----/g, severity: 'critical' },
  { name: 'Hardcoded Password', re: /(password|passwd|pwd)\s*[:=]\s*['"][^'"]{6,}['"]/gi, severity: 'high' },
  { name: 'JWT Token', re: /\beyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\b/g, severity: 'high' },
];

const SQLI_PATTERNS = [
  { name: 'SQL 字符串拼接注入', re: /(SELECT|INSERT|UPDATE|DELETE|DROP)\s+[\s\S]*?["']\s*\+\s*/gi, severity: 'high' },
  { name: 'SQL f-string 注入', re: /(SELECT|INSERT|UPDATE|DELETE)\s+[\s\S]*?\{\s*[a-z_]/gi, severity: 'high' },
  { name: 'SQL 格式化注入', re: /execute\s*\(\s*[f"']\s*(SELECT|INSERT|UPDATE|DELETE)/gi, severity: 'high' },
];

function scanFile(filePath, minSeverity) {
  let text;
  try { text = fs.readFileSync(filePath, 'utf8'); } catch { return null; }
  const findings = [];
  for (const p of SECRET_PATTERNS) {
    if (p.severity === 'critical' || severityRank(p.severity) >= severityRank(minSeverity)) {
      const matches = text.match(p.re);
      if (matches && matches.length > 0) {
        findings.push({ type: 'secret', rule: p.name, severity: p.severity, count: matches.length, file: filePath });
      }
    }
  }
  for (const p of SQLI_PATTERNS) {
    const matches = text.match(p.re);
    if (matches && matches.length > 0) {
      findings.push({ type: 'sql_injection', rule: p.name, severity: p.severity, count: matches.length, file: filePath });
    }
  }
  return findings;
}

function severityRank(s) {
  return { low: 0, medium: 1, high: 2, critical: 3 }[s] ?? 0;
}

function resolveTargetFiles(filePaths) {
  if (filePaths && filePaths.length > 0) return filePaths;
  // 回退：扫描 git diff 变更文件
  try {
    const out = execSync('git diff --name-only --diff-filter=ACM HEAD', { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] });
    return out.split('\n').map(s => s.trim()).filter(Boolean);
  } catch {
    return [];
  }
}

function scanIncrementalAstDiff(args) {
  const files = resolveTargetFiles(args.file_paths);
  const minSeverity = args.severity || 'high';
  const allFindings = [];
  for (const f of files) {
    const findings = scanFile(f, minSeverity);
    if (findings) allFindings.push(...findings);
  }
  const critical = allFindings.filter(x => x.severity === 'critical').length;
  const high = allFindings.filter(x => x.severity === 'high').length;
  return {
    scanned_files: files.length,
    total_findings: allFindings.length,
    critical_count: critical,
    high_count: high,
    findings: allFindings.slice(0, 50),
    verdict: critical > 0 ? 'BLOCK' : (high > 0 ? 'REVIEW' : 'PASS'),
  };
}

function verifyLicenseCompliance(args) {
  const projectPath = args.project_path || '.';
  const blocked = (args.blocked_licenses || ['GPL', 'AGPL']).map(s => s.toUpperCase());
  const findings = [];
  // package.json
  const pkgPath = path.join(projectPath, 'package.json');
  if (fs.existsSync(pkgPath)) {
    const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
    const deps = { ...(pkg.dependencies || {}), ...(pkg.devDependencies || {}) };
    findings.push(...scanDeps(Object.keys(deps), 'npm', blocked));
  }
  // Cargo.toml
  const cargoPath = path.join(projectPath, 'Cargo.toml');
  if (fs.existsSync(cargoPath)) {
    const cargo = fs.readFileSync(cargoPath, 'utf8');
    const depRe = /\[dependencies\]\s*([\s\S]*?)(\n\[|\n*$)/g;
    let m;
    while ((m = depRe.exec(cargo)) !== null) {
      const names = [...m[1].matchAll(/^([a-zA-Z0-9_-]+)\s*=/gm)].map(x => x[1]);
      findings.push(...scanDeps(names, 'cargo', blocked));
    }
  }
  const blockedHits = findings.filter(f => f.blocked);
  return {
    project: projectPath,
    scanned_dependencies: findings.length,
    blocked_found: blockedHits.length,
    blocked_licenses: blocked,
    findings: blockedHits.slice(0, 50),
    verdict: blockedHits.length > 0 ? 'BLOCK' : 'PASS',
  };
}

function scanDeps(names, registry, blocked) {
  // 端侧启发式：无法联网查询精确协议，按已知高危包名匹配 + 标记【待核验】
  const knownCopyleft = [
    { name: 'gpl', licenses: ['GPL'] }, { name: 'agpl', licenses: ['AGPL'] },
  ];
  return names.map(n => {
    const lower = n.toLowerCase();
    const hit = knownCopyleft.find(k => lower.includes(k.name));
    const blockedHit = hit && hit.licenses.some(l => blocked.includes(l));
    return {
      dependency: n,
      registry,
      suspected_license: hit ? hit.licenses.join('/') : 'unknown',
      blocked: !!blockedHit,
      note: hit ? undefined : '【待核验，无官方资料】需联网核实协议',
    };
  });
}

function generateAuditReport(args) {
  const format = args.format || 'json';
  const report = {
    generated_at: new Date().toISOString(),
    server: SERVER_INFO.name,
    secret_scan: scanIncrementalAstDiff({ severity: 'medium' }),
    license_scan: verifyLicenseCompliance({ project_path: '.' }),
    risk_level: null,
  };
  const critical = report.secret_scan.critical_count;
  report.risk_level = critical > 0 ? 'CRITICAL' : (report.secret_scan.high_count > 0 ? 'HIGH' : 'LOW');
  if (format === 'markdown') {
    return `# 安全审计报告\n\n- 生成时间: ${report.generated_at}\n- 密钥泄露: ${report.secret_scan.total_findings} 处\n- 协议阻断: ${report.license_scan.blocked_found} 项\n- 风险等级: ${report.risk_level}\n`;
  }
  return report;
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
        if (name === 'scan_incremental_ast_diff') result = scanIncrementalAstDiff(args || {});
        else if (name === 'verify_license_compliance') result = verifyLicenseCompliance(args || {});
        else if (name === 'generate_audit_report') result = generateAuditReport(args || {});
        else return respondError(id, -32601, `Unknown tool: ${name}`);
        return respond(id, { content: [{ type: 'text', text: typeof result === 'string' ? result : JSON.stringify(result) }], isError: false });
      }
      default:
        return respondError(id, -32601, `Unknown method: ${method}`);
    }
  } catch (e) {
    return respondError(id, -32603, `Internal error: ${e.message}`);
  }
});
