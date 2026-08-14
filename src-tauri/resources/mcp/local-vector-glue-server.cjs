#!/usr/bin/env node
// Local Vector Glue — 本地代码树增量向量索引库 (MCP stdio server)
// 协议: MCP JSON-RPC 2.0 over stdio
// 工具: query_vector_similarity / index_workspace
// 资源: resources://projects/active_tree / resources://projects/diff_snippets

'use strict';

const fs = require('fs');
const path = require('path');
const readline = require('readline');
const { execSync } = require('child_process');

const SERVER_INFO = { name: 'mcp-server-local-vector-glue', version: '1.0.0' };

const TOOLS = [
  {
    name: 'query_vector_similarity',
    description: '对当前工作区的增量代码片段执行向量相似检索，返回最相关的核心上下文。',
    inputSchema: {
      type: 'object',
      properties: {
        query_text: { type: 'string', description: '检索查询文本' },
        top_k: { type: 'integer', default: 5, description: '返回最相关的前 K 个结果' },
        scope: { type: 'string', enum: ['changed', 'all', 'recent'], default: 'changed', description: '检索范围' },
      },
      required: ['query_text'],
    },
  },
  {
    name: 'index_workspace',
    description: '触发对当前项目工作区执行全量或增量向量索引重建。',
    inputSchema: {
      type: 'object',
      properties: {
        mode: { type: 'string', enum: ['full', 'incremental'], default: 'incremental' },
      },
    },
  },
];

const RESOURCES = [
  { uri: 'resources://projects/active_tree', description: '当前活跃项目的目录树拓扑结构' },
  { uri: 'resources://projects/diff_snippets', description: '当前 Git 工作区的增量代码差异片段' },
];

// 内存索引：file_path → 预处理文本
let workspaceIndex = new Map();

// ─── 轻量文本向量化（char n-gram 词袋，无需 ONNX 依赖） ──────────
function ngrams(text, n = 3) {
  const clean = text.toLowerCase().replace(/[^a-z0-9_\u4e00-\u9fa5]/g, ' ');
  const tokens = clean.split(/\s+/).filter(Boolean);
  const bag = new Map();
  for (const t of tokens) {
    for (let i = 0; i <= t.length - n; i++) {
      const g = t.slice(i, i + n);
      bag.set(g, (bag.get(g) || 0) + 1);
    }
  }
  return bag;
}

function cosineSimilarity(a, b) {
  let dot = 0, na = 0, nb = 0;
  for (const [k, v] of a) { na += v * v; if (b.has(k)) dot += v * b.get(k); }
  for (const v of b.values()) nb += v * v;
  if (na === 0 || nb === 0) return 0;
  return dot / (Math.sqrt(na) * Math.sqrt(nb));
}

function collectFiles(root, extensions) {
  const files = [];
  const skip = new Set(['node_modules', '.git', 'target', 'dist', 'build', '.venv', '__pycache__']);
  function walk(dir) {
    let entries;
    try { entries = fs.readdirSync(dir, { withFileTypes: true }); } catch { return; }
    for (const e of entries) {
      if (skip.has(e.name)) continue;
      const p = path.join(dir, e.name);
      if (e.isDirectory()) walk(p);
      else if (extensions.some(ext => e.name.endsWith(ext))) files.push(p);
    }
  }
  walk(root);
  return files;
}

const CODE_EXTS = ['.rs', '.ts', '.tsx', '.js', '.jsx', '.py', '.go', '.java', '.c', '.cpp', '.toml', '.json', '.md', '.yaml', '.yml'];

function indexWorkspace(args) {
  const root = process.cwd();
  const files = collectFiles(root, CODE_EXTS);
  const indexed = new Map();
  for (const f of files) {
    try {
      const text = fs.readFileSync(f, 'utf8');
      if (text.length < 20) continue;
      indexed.set(f, text);
    } catch { /* 跳过不可读文件 */ }
  }
  workspaceIndex = indexed;
  return { mode: args.mode || 'incremental', root, indexed_files: indexed.size };
}

function queryVectorSimilarity(args) {
  const query = args.query_text || '';
  const topK = args.top_k || 5;
  const scope = args.scope || 'changed';
  if (workspaceIndex.size === 0) indexWorkspace({ mode: 'incremental' });

  const queryBag = ngrams(query);
  let candidates = [...workspaceIndex.entries()];

  if (scope === 'changed') {
    // 仅检索 git 变更文件
    try {
      const changed = new Set(execSync('git diff --name-only --diff-filter=ACM HEAD', { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] })
        .split('\n').map(s => s.trim()).filter(Boolean));
      candidates = candidates.filter(([f]) => changed.has(f) || changed.has(path.relative(process.cwd(), f)));
    } catch { /* 非 git 仓库，回退全量 */ }
  }

  const scored = candidates.map(([file, text]) => ({
    file: path.relative(process.cwd(), file),
    similarity: cosineSimilarity(queryBag, ngrams(text.slice(0, 4000))),
  })).sort((a, b) => b.similarity - a.similarity).slice(0, topK);

  const results = scored.map(s => ({
    ...s,
    similarity: Number(s.similarity.toFixed(4)),
    snippet: (() => {
      const text = workspaceIndex.get(path.resolve(process.cwd(), s.file)) || '';
      return text.slice(0, 200);
    })(),
  }));

  return { query, scope, top_k: topK, results };
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
        return respond(id, { protocolVersion: '2024-11-05', capabilities: { tools: {}, resources: {} }, serverInfo: SERVER_INFO });
      case 'ping':
        return respond(id, {});
      case 'tools/list':
        return respond(id, { tools: TOOLS });
      case 'resources/list':
        return respond(id, { resources: RESOURCES });
      case 'tools/call': {
        const { name, arguments: args } = params || {};
        let result;
        if (name === 'query_vector_similarity') result = queryVectorSimilarity(args || {});
        else if (name === 'index_workspace') result = indexWorkspace(args || {});
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
