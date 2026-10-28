#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

function fail(message) {
  throw new Error(message);
}

function parse(argv) {
  const [command, ...args] = argv;
  if (!['init', 'check'].includes(command)) fail('用法：spec-kit.cjs init|check --feature specs/<名称> [--stage clarify|plan|tasks|analyze|implement|converge]');
  const options = { command };
  for (let i = 0; i < args.length; i += 2) {
    const key = args[i];
    if (!['--feature', '--stage'].includes(key) || !args[i + 1] || options[key.slice(2)]) fail(`无效或重复参数：${key}`);
    options[key.slice(2)] = args[i + 1];
  }
  if (!/^specs\/[a-z0-9][a-z0-9-]*$/.test(options.feature || '')) fail('必须显式指定稳定标识 specs/<名称>，名称仅使用小写字母、数字和连字符');
  if (command === 'init' && options.stage) fail('init 不接受 --stage');
  if (command === 'check' && !['clarify', 'plan', 'tasks', 'analyze', 'implement', 'converge'].includes(options.stage)) fail('check 必须指定有效的 --stage');
  return options;
}

// 只检查实际路径，不先 realpath 掩盖符号链接；缺失的尾部仅允许由 init 创建。
function directoryPath(target, required = false) {
  const absolute = path.resolve(target);
  let current = path.parse(absolute).root;
  for (const part of absolute.slice(current.length).split(path.sep).filter(Boolean)) {
    current = path.join(current, part);
    let stat;
    try { stat = fs.lstatSync(current); } catch (error) {
      if (error.code === 'ENOENT' && !required) return false;
      throw error;
    }
    if (stat.isSymbolicLink()) fail(`需求目录不得为符号链接：${current}`);
    if (!stat.isDirectory()) fail(`需求路径不是目录：${current}`);
  }
  return true;
}

function absoluteRoot(value, label) {
  if (!path.isAbsolute(value) || value.split(path.sep).includes('..') || path.resolve(value) === path.parse(path.resolve(value)).root) fail(`${label} 必须为不含上级跳转的绝对目录`);
  directoryPath(value, true);
  return path.resolve(value);
}

function artifactPaths(target) {
  for (const name of ['spec.md', 'plan.md', 'tasks.md']) {
    let stat;
    try { stat = fs.lstatSync(path.join(target, name)); } catch (error) { if (error.code === 'ENOENT') continue; throw error; }
    if (!stat.isFile()) fail(`产物必须为普通文件：${name}`);
  }
}

function featurePath(root, feature) {
  if (!fs.existsSync(path.join(root, '.specify/memory/constitution.md'))) fail('请在当前仓库根目录执行');
  directoryPath(path.join(root, '.specify'), true);
  const configFile = path.join(root, '.specify/spec-kit.json');
  let config = {};
  try {
    if (!fs.lstatSync(configFile).isFile()) fail('spec-kit.json 必须为普通文件');
    config = JSON.parse(fs.readFileSync(configFile, 'utf8'));
    if (!config || Array.isArray(config) || typeof config !== 'object' || Object.keys(config).some(key => key !== 'obsidian_project_path')) fail('无效的 Spec Kit 项目配置');
  } catch (error) { if (error.code !== 'ENOENT') throw error; }
  const project = config.obsidian_project_path;
  if (project !== undefined && (typeof project !== 'string' || !project || project.includes('\\') || path.isAbsolute(project) || project.split('/').some(part => !part || part === '.' || part === '..'))) fail('obsidian_project_path 必须为 Vault 内的相对项目路径');

  const repository = path.join(root, feature);
  const repositoryExists = directoryPath(repository);
  artifactPaths(repository);
  const override = process.env.QFY_SPEC_KIT_OBSIDIAN_ROOT;
  const vault = process.env.QFY_OBSIDIAN_VAULT_ROOT;
  let bases;
  if (override) {
    bases = [absoluteRoot(override, 'QFY_SPEC_KIT_OBSIDIAN_ROOT')];
  } else if (vault && project) {
    const vaultRoot = absoluteRoot(vault, 'QFY_OBSIDIAN_VAULT_ROOT');
    const projectRoot = path.join(vaultRoot, project);
    directoryPath(projectRoot, true);
    bases = [
      '10-项目文档/03-计划与任务/Spec Kit',
      '10-项目文档/01-路线图/Spec Kit',
      '10-项目文档/04-规范与运行手册/Spec Kit',
      '90-历史归档/Spec Kit',
    ].map(relative => path.join(projectRoot, relative));
  }
  if (!bases) return { directory: repository, storage: 'repository' };
  const candidates = bases.map(base => path.join(base, path.basename(feature)));
  const existing = candidates.filter(candidate => {
    const exists = directoryPath(candidate);
    artifactPaths(candidate);
    return exists;
  });
  // 同一稳定标识只能有一份正文；仓库历史不因启用 Vault 而自动复制。
  if (existing.length + Number(repositoryExists) > 1) fail(`重复 feature，须先核对迁移登记：${feature}`);
  if (repositoryExists) return { directory: repository, storage: 'repository' };
  return { directory: existing[0] || candidates[0], storage: 'obsidian' };
}

function readArtifact(directory, name, allowDraft) {
  const file = path.join(directory, name);
  if (!fs.existsSync(file)) fail(`缺少产物：${name}`);
  if (!fs.lstatSync(file).isFile()) fail(`产物必须为普通文件：${name}`);
  const content = fs.readFileSync(file, 'utf8');
  if (!content.trim()) fail(`产物为空：${name}`);
  const prose = content.replace(/<!--[\s\S]*?-->/g, '').replace(/^(```|~~~)[\s\S]*?^\1[^\n]*$/gm, '').replace(/^\s*>.*$/gm, '');
  if (!/^#\s+\S/m.test(prose)) fail(`产物缺少 Markdown 标题：${name}`);
  if (!prose.replace(/^#{1,6}\s+.*$/gm, '').trim()) fail(`产物只有标题或注释：${name}`);
  const unresolved = prose.split('\n').filter(line => {
    const historical = /^\s*(?:[-*]\s+)?(?:已解决|resolved)\s*[:：]/i.test(line);
    const markers = line.match(/NEEDS CLARIFICATION/gi) || [];
    return !historical || markers.length > 1;
  }).join('\n');
  const placeholders = /\[待填写[^\]]*\]|\[FEATURE NAME\]|\[DATE\]|\$ARGUMENTS/i.test(prose);
  if (!allowDraft && (placeholders || /NEEDS CLARIFICATION/i.test(unresolved))) fail(`产物含未填写模板或未解决澄清：${name}`);
  return prose;
}

function run(root, script, args, feature) {
  const result = spawnSync('bash', [path.join(root, '.specify/scripts/bash', script), ...args], {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, SPECIFY_INIT_DIR: root, SPECIFY_FEATURE_DIRECTORY: feature, SPECIFY_FEATURE_NO_PERSIST: '1' },
  });
  if (result.error) throw result.error;
  if (result.status !== 0) fail(result.stderr.trim() || `${script} 失败`);
  return JSON.parse(result.stdout);
}

function main(argv) {
  const options = parse(argv);
  const root = fs.realpathSync(process.cwd());
  const { directory, storage } = featurePath(root, options.feature);
  const audit = { FEATURE: options.feature, STORAGE: storage };
  if (options.command === 'init') {
    const spec = path.join(directory, 'spec.md');
    if (fs.existsSync(spec)) {
      if (!fs.lstatSync(spec).isFile()) fail('已有 spec.md 必须为普通文件');
      const reads = ['spec.md', 'plan.md', 'tasks.md'].map(name => path.join(directory, name)).filter(file => fs.existsSync(file));
      console.log(JSON.stringify({ ...audit, FEATURE_DIR: directory, FEATURE_SPEC: spec, EXISTING: true, REQUIRED_READS: reads, ACTION: '先读取已有产物并增量续写，不复制模板' }));
      return;
    }
    if (['plan.md', 'tasks.md'].some(name => fs.existsSync(path.join(directory, name)))) fail('已有计划或任务但缺少规格，请先核实历史需求，不重新初始化');
    const resolved = run(root, 'resolve-template.sh', ['spec-template', '--json'], directory);
    if (!resolved.TEMPLATE_CONTENT?.trim()) fail('规格模板为空');
    fs.mkdirSync(directory, { recursive: true });
    directoryPath(directory, true);
    artifactPaths(directory);
    // 排他创建，防止并发初始化覆盖另一进程先写入的规格。
    fs.writeFileSync(spec, resolved.TEMPLATE_CONTENT, { flag: 'wx' });
    console.log(JSON.stringify({ ...audit, FEATURE_DIR: directory, FEATURE_SPEC: spec, EXISTING: false, REQUIRED_READS: [spec] }));
    return;
  }

  const early = ['clarify', 'plan'].includes(options.stage);
  const names = early ? ['spec.md'] : options.stage === 'tasks' ? ['spec.md', 'plan.md'] : ['spec.md', 'plan.md', 'tasks.md'];
  const contents = Object.fromEntries(names.map(name => [name, readArtifact(directory, name, options.stage === 'clarify')]));
  if (!/FR-\d+/.test(contents['spec.md'])) fail('spec.md 缺少稳定的 FR-编号');
  if (contents['plan.md'] && !/^#{1,6}\s+.*(?:验证|验收|Validation|Testing)/im.test(contents['plan.md'])) fail('plan.md 缺少验证章节');
  if (contents['tasks.md']) {
    const tasks = [...contents['tasks.md'].matchAll(/^- \[[ xX]\] (T\d+)\b(.*)$/gm)];
    if (!tasks.length) fail('tasks.md 缺少带 T编号 的任务');
    const ids = tasks.map(task => task[1]);
    if (new Set(ids).size !== ids.length) fail('tasks.md 存在重复任务编号');
    if (!tasks.some(task => /验证|验收|测试|\b(?:test|build|lint|typecheck)\b/i.test(task[2]))) fail('tasks.md 缺少明确的验证任务');
  }
  const args = early ? ['--json', '--paths-only'] : ['--json', '--require-spec'];
  if (names.includes('tasks.md')) args.push('--require-tasks', '--include-tasks');
  const resolved = run(root, 'check-prerequisites.sh', args, directory);
  if (resolved.FEATURE_DIR !== directory) fail('上游脚本解析到错误的需求目录');
  const existing = ['spec.md', 'plan.md', 'tasks.md'].filter(name => fs.existsSync(path.join(directory, name)));
  const mode = ['plan', 'tasks'].includes(options.stage) ? (existing.includes(`${options.stage}.md`) ? 'update' : 'create') : 'check';
  console.log(JSON.stringify({ ...resolved, ...audit, STAGE: options.stage, MODE: mode, REQUIRED_READS: existing.map(name => path.join(directory, name)), NOTICE: '只通过结构检查；仍须读取产物、执行语义审查和真实验证' }));
}

try {
  main(process.argv.slice(2));
} catch (error) {
  console.error(`Spec Kit 检查失败：${error.message}`);
  process.exitCode = 1;
}
