'use strict';

const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawn, spawnSync } = require('node:child_process');
const { test } = require('node:test');

const repo = path.resolve(__dirname, '../..');
const entry = path.join(repo, 'scripts/ops/spec-kit.cjs');

function fixture(t) {
  const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'deploy-go-spec-kit-')));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  for (const directory of ['scripts', 'templates', 'memory']) {
    fs.cpSync(path.join(repo, '.specify', directory), path.join(root, '.specify', directory), { recursive: true });
  }
  return root;
}

function write(root, name, content) {
  const directory = path.join(root, 'specs/001-example');
  fs.mkdirSync(directory, { recursive: true });
  fs.writeFileSync(path.join(directory, name), content);
}

function complete(root) {
  write(root, 'spec.md', '# 测试需求\n\n## 功能要求\n- FR-001：实现需求\n');
  write(root, 'plan.md', '# 实施方案\n\n## 验证\n运行聚焦测试\n');
  write(root, 'tasks.md', '# 任务\n- [X] T001 FR-001 实现功能\n- [ ] T002 FR-001 验证功能\n');
}

function run(root, args) {
  return spawnSync(process.execPath, [entry, ...args], {
    cwd: root, encoding: 'utf8',
    // 调用方的旧环境变量不能把检查重定向到其他项目或需求。
    env: { ...process.env, SPECIFY_INIT_DIR: '/invalid-project', SPECIFY_FEATURE_DIRECTORY: 'specs/stale' },
  });
}

function check(root, stage = 'implement') {
  return run(root, ['check', '--feature', 'specs/001-example', '--stage', stage]);
}

test('所有阶段定位显式需求，检查不改写指针或产物', t => {
  const root = fixture(t);
  complete(root);
  const pointer = path.join(root, '.specify/feature.json');
  fs.writeFileSync(pointer, '{"feature_directory":"specs/other"}\n');
  const before = fs.readFileSync(pointer, 'utf8');
  const taskBefore = fs.readFileSync(path.join(root, 'specs/001-example/tasks.md'), 'utf8');
  for (const stage of ['clarify', 'plan', 'tasks', 'analyze', 'implement', 'converge']) {
    const result = check(root, stage);
    assert.equal(result.status, 0, result.stderr);
    const output = JSON.parse(result.stdout);
    assert.equal(output.FEATURE_DIR, path.join(root, 'specs/001-example'));
    assert.ok(output.REQUIRED_READS.includes(path.join(root, 'specs/001-example/spec.md')));
  }
  assert.equal(fs.readFileSync(pointer, 'utf8'), before);
  assert.equal(fs.readFileSync(path.join(root, 'specs/001-example/tasks.md'), 'utf8'), taskBefore);
});

test('拒绝缺失、空白、模板残留、关键澄清和重复任务', t => {
  const root = fixture(t);
  const cases = [
    ['spec.md', null, '缺少产物'],
    ['plan.md', ' \n', '产物为空'],
    ['plan.md', '# 方案\n## 验证\n<!-- 未完成 -->\n', '只有标题或注释'],
    ['tasks.md', '# 任务\n- [ ] T001 [待填写任务]\n', '未填写模板'],
    ['spec.md', '# 需求\nFR-001 NEEDS CLARIFICATION\n', '未解决澄清'],
    ['spec.md', '# 需求\n未编号需求\n', 'FR-编号'],
    ['plan.md', '# 方案\n无验证章节\n', '验证章节'],
    ['tasks.md', '# 任务\n- [ ] T001 实现\n- [X] T001 验证\n', '重复任务编号'],
    ['tasks.md', '# 任务\n- [ ] T001 实现\n', '验证任务'],
  ];
  for (const [name, content, error] of cases) {
    complete(root);
    if (content === null) fs.unlinkSync(path.join(root, 'specs/001-example', name));
    else write(root, name, content);
    const result = check(root);
    assert.notEqual(result.status, 0);
    assert.ok(result.stderr.includes(error), result.stderr);
    assert.equal(fs.existsSync(path.join(root, '.specify/feature.json')), false);
  }
});

test('澄清可以处理草稿，其他阶段阻断；已解决记录和代码示例不误报', t => {
  const root = fixture(t);
  complete(root);
  write(root, 'spec.md', '# 需求\nFR-001 [NEEDS CLARIFICATION: 业务决策]\n');
  assert.equal(check(root, 'clarify').status, 0);
  assert.notEqual(check(root, 'plan').status, 0);
  write(root, 'spec.md', '# 需求\nFR-001 已解决其他问题，但 NEEDS CLARIFICATION: 业务决策仍未确定\n');
  assert.notEqual(check(root, 'plan').status, 0, '同一行提到已解决不能掩盖当前澄清');
  for (const pending of [
    'FR-001 [NEEDS CLARIFICATION: resolved 状态是否允许重开？]',
    'FR-001 [NEEDS CLARIFICATION: 已解决状态是否允许重开？]',
    '已解决：NEEDS CLARIFICATION 旧问题；NEEDS CLARIFICATION 新问题',
    '已解决：[待填写内容]',
  ]) {
    write(root, 'spec.md', `# 需求\nFR-001 要求\n${pending}\n`);
    assert.notEqual(check(root, 'implement').status, 0, pending);
  }
  write(root, 'spec.md', '# 需求\nFR-001 已确定要求\n已解决：NEEDS CLARIFICATION\n```text\n[待填写示例]\n```\n> NEEDS CLARIFICATION 是历史示例\n');
  assert.equal(check(root).status, 0);
  write(root, 'tasks.md', '# 任务\n- [X] T001 实现\n- [X] T002 验证\n');
  assert.equal(check(root).status, 0, '已完成任务仍可做收敛检查');
});

test('plan/tasks 分别只要求本阶段输入', t => {
  const root = fixture(t);
  write(root, 'spec.md', '# 需求\nFR-001 要求\n');
  assert.equal(check(root, 'plan').status, 0);
  assert.notEqual(check(root, 'tasks').status, 0);
  write(root, 'plan.md', '# 方案\n## Validation\n聚焦测试\n');
  assert.equal(check(root, 'tasks').status, 0);
  assert.notEqual(check(root, 'implement').status, 0);
});

test('示例、注释和引用不能冒充正式要求或任务，也不造成编号冲突', t => {
  const root = fixture(t);
  const cases = [
    ['spec.md', '# 需求\n正文无编号\n```text\nFR-001 示例要求\n```\n', 'FR-编号'],
    ['spec.md', '# 需求\n正文无编号\n<!-- FR-001 注释 -->\n> FR-001 引用\n', 'FR-编号'],
    ['plan.md', '# 方案\n正文\n```text\n## 验证\n示例\n```\n', '验证章节'],
    ['tasks.md', '# 任务\n正文\n```text\n- [ ] T001 验证示例\n```\n', '带 T编号'],
    ['spec.md', '<!-- # 伪标题 -->\nFR-001 正文\n', 'Markdown 标题'],
  ];
  for (const [name, content, error] of cases) {
    complete(root);
    write(root, name, content);
    const result = check(root);
    assert.notEqual(result.status, 0);
    assert.ok(result.stderr.includes(error), result.stderr);
  }
  complete(root);
  write(root, 'tasks.md', '# 任务\n- [ ] T001 验证真实需求\n```text\n- [ ] T001 示例任务\n```\n<!-- - [ ] T001 注释 -->\n> - [ ] T001 引用\n');
  assert.equal(check(root).status, 0, '示例编号不能导致正式任务被误判重复');
});

test('初始化使用覆盖模板，重复初始化保留规格、完成状态和证据', t => {
  const root = fixture(t);
  const args = ['init', '--feature', 'specs/001-example'];
  const initial = run(root, args);
  assert.equal(initial.status, 0, initial.stderr);
  assert.equal(JSON.parse(initial.stdout).EXISTING, false);
  assert.ok(fs.readFileSync(path.join(root, 'specs/001-example/spec.md'), 'utf8').includes('风险与验证要求'));
  assert.notEqual(check(root, 'plan').status, 0, '初始化草稿不得被认为就绪');
  complete(root);
  const names = ['spec.md', 'plan.md', 'tasks.md'];
  const before = names.map(name => fs.readFileSync(path.join(root, 'specs/001-example', name), 'utf8'));
  const again = run(root, args);
  assert.equal(again.status, 0, again.stderr);
  assert.equal(JSON.parse(again.stdout).EXISTING, true);
  assert.deepEqual(JSON.parse(again.stdout).REQUIRED_READS, names.map(name => path.join(root, 'specs/001-example', name)));
  assert.deepEqual(names.map(name => fs.readFileSync(path.join(root, 'specs/001-example', name), 'utf8')), before);
  assert.equal(fs.existsSync(path.join(root, '.specify/feature.json')), false);
});

test('缺规格的旧任务目录不能重新初始化', t => {
  const root = fixture(t);
  write(root, 'tasks.md', '# 原有任务\n- [X] T001 验证\n');
  const result = run(root, ['init', '--feature', 'specs/001-example']);
  assert.notEqual(result.status, 0);
  assert.ok(result.stderr.includes('已有计划或任务'));
  assert.equal(fs.existsSync(path.join(root, 'specs/001-example/spec.md')), false);
});

test('并发初始化不会覆盖另一进程创建的规格', async t => {
  const root = fixture(t);
  const invoke = () => new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [entry, 'init', '--feature', 'specs/001-example'], { cwd: root, env: { ...process.env, SPECIFY_INIT_DIR: root } });
    let stderr = '';
    child.stderr.on('data', data => { stderr += data; });
    child.on('error', reject);
    child.on('close', status => resolve({ status, stderr }));
  });
  const results = await Promise.all([invoke(), invoke()]);
  assert.ok(results.some(result => result.status === 0));
  for (const result of results) {
    if (result.status !== 0) assert.ok(result.stderr.includes('EEXIST'), result.stderr);
  }
  const expected = fs.readFileSync(path.join(root, '.specify/templates/overrides/spec-template.md'), 'utf8');
  assert.equal(fs.readFileSync(path.join(root, 'specs/001-example/spec.md'), 'utf8'), expected);
  assert.equal(fs.existsSync(path.join(root, '.specify/feature.json')), false);
});

test('已有空规格不被覆盖，早期阶段也拒绝其他产物的符号链接', t => {
  const root = fixture(t);
  write(root, 'spec.md', '');
  assert.equal(run(root, ['init', '--feature', 'specs/001-example']).status, 0);
  assert.equal(fs.readFileSync(path.join(root, 'specs/001-example/spec.md'), 'utf8'), '');
  complete(root);
  const plan = path.join(root, 'specs/001-example/plan.md');
  fs.unlinkSync(plan);
  fs.symlinkSync(path.join(root, 'not-created'), plan);
  const result = check(root, 'plan');
  assert.notEqual(result.status, 0);
  assert.ok(result.stderr.includes('普通文件'), result.stderr);
});

test('拒绝隐式目录、越界路径、符号链接和未知阶段', t => {
  const root = fixture(t);
  for (const feature of ['/tmp/feature', '../feature', 'specs/../feature', 'specs/a/b']) {
    assert.notEqual(run(root, ['init', '--feature', feature]).status, 0);
  }
  assert.notEqual(run(root, ['init']).status, 0);
  assert.notEqual(check(root, 'unknown').status, 0);
  const outside = fs.mkdtempSync(path.join(os.tmpdir(), 'deploy-go-spec-outside-'));
  t.after(() => fs.rmSync(outside, { recursive: true, force: true }));
  fs.symlinkSync(outside, path.join(root, 'specs'), 'dir');
  assert.notEqual(run(root, ['init', '--feature', 'specs/001-example']).status, 0);
  assert.deepEqual(fs.readdirSync(outside), []);
});

test('受管理资产匹配 manifest，Shell 语法和执行位有效', () => {
  const registry = JSON.parse(fs.readFileSync(path.join(repo, '.specify/workflows/workflow-registry.json'), 'utf8'));
  for (const name of Object.keys(registry.workflows)) {
    assert.ok(fs.statSync(path.join(repo, '.specify/workflows', name, 'workflow.yml')).isFile(), `缺少注册工作流：${name}`);
  }
  for (const integration of ['codex', 'mcode', 'speckit']) {
    const manifest = JSON.parse(fs.readFileSync(path.join(repo, '.specify/integrations', `${integration}.manifest.json`), 'utf8'));
    assert.equal(manifest.version, '1.0.13');
    for (const [name, expected] of Object.entries(manifest.files)) {
      const file = path.join(repo, name);
      assert.equal(createHash('sha256').update(fs.readFileSync(file)).digest('hex'), expected, `资产漂移：${name}`);
      if (name.endsWith('.sh')) {
        assert.ok(fs.statSync(file).mode & 0o111, `缺少执行位：${name}`);
        const result = spawnSync('bash', ['-n', file], { encoding: 'utf8' });
        assert.equal(result.status, 0, result.stderr);
      }
    }
  }
});

test('plan/tasks 覆盖模板可被上游解析', t => {
  const root = fixture(t);
  for (const name of ['plan', 'tasks']) {
    const result = spawnSync('bash', [path.join(root, '.specify/scripts/bash/resolve-template.sh'), `${name}-template`, '--json'], { cwd: root, encoding: 'utf8', env: { ...process.env, SPECIFY_INIT_DIR: root } });
    assert.equal(result.status, 0, result.stderr);
    const content = JSON.parse(result.stdout).TEMPLATE_CONTENT;
    assert.ok(content.includes(name === 'plan' ? '验证与验收' : '必须包含匹配风险的验证任务'));
  }
});

test('在真实 Git fixture 中初始化和阶段检查不创建或切换分支', t => {
  const root = fixture(t);
  const init = spawnSync('git', ['init', '--initial-branch=main', root], { encoding: 'utf8' });
  assert.equal(init.status, 0, init.stderr);
  const head = path.join(root, '.git/HEAD');
  const before = fs.readFileSync(head, 'utf8');
  const result = run(root, ['init', '--feature', 'specs/001-example']);
  assert.equal(result.status, 0, result.stderr);
  complete(root);
  for (const stage of ['plan', 'tasks', 'analyze', 'implement', 'converge']) {
    const checked = check(root, stage);
    assert.equal(checked.status, 0, checked.stderr);
  }
  assert.equal(fs.readFileSync(head, 'utf8'), before);
  assert.deepEqual(fs.readdirSync(path.join(root, '.git/refs/heads')), []);
  assert.equal(fs.existsSync(path.join(root, '.specify/feature.json')), false);
});
