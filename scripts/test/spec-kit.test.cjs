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

function run(root, args, overrides = {}) {
  return spawnSync(process.execPath, [entry, ...args], {
    cwd: root, encoding: 'utf8',
    // 调用方的旧环境变量不能把检查重定向到其他项目或需求。
    env: { ...process.env, SPECIFY_INIT_DIR: '/invalid-project', SPECIFY_FEATURE_DIRECTORY: 'specs/stale', QFY_OBSIDIAN_VAULT_ROOT: '', QFY_SPEC_KIT_OBSIDIAN_ROOT: '', ...overrides },
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
    const child = spawn(process.execPath, [entry, 'init', '--feature', 'specs/001-example'], { cwd: root, env: { ...process.env, SPECIFY_INIT_DIR: root, QFY_OBSIDIAN_VAULT_ROOT: '', QFY_SPEC_KIT_OBSIDIAN_ROOT: '' } });
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
  for (const integration of ['claude', 'speckit']) {
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
    const result = spawnSync('bash', [path.join(root, '.specify/scripts/bash/resolve-template.sh'), `${name}-template`, '--json'], { cwd: root, encoding: 'utf8', env: { ...process.env, SPECIFY_INIT_DIR: root, QFY_OBSIDIAN_VAULT_ROOT: '', QFY_SPEC_KIT_OBSIDIAN_ROOT: '' } });
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

const storageLocations = [
  '10-项目文档/03-计划与任务/Spec Kit',
  '10-项目文档/01-路线图/Spec Kit',
  '10-项目文档/04-规范与运行手册/Spec Kit',
  '90-历史归档/Spec Kit',
];

function externalFixture(t) {
  const root = fixture(t);
  const vault = path.join(root, 'vault');
  const project = path.join(vault, 'QFY/项目/deploy-go');
  fs.mkdirSync(project, { recursive: true });
  fs.writeFileSync(path.join(root, '.specify/spec-kit.json'), JSON.stringify({ obsidian_project_path: 'QFY/项目/deploy-go' }));
  return { root, vault, project, env: { QFY_OBSIDIAN_VAULT_ROOT: vault } };
}

const initArgs = ['init', '--feature', 'specs/001-example'];
const checkArgs = ['check', '--feature', 'specs/001-example', '--stage', 'implement'];

function externalComplete(root, directory) {
  complete(root);
  fs.mkdirSync(path.dirname(directory), { recursive: true });
  fs.renameSync(path.join(root, 'specs/001-example'), directory);
  fs.rmdirSync(path.join(root, 'specs'));
}

test('项目配置没有 Vault 时仍用仓库；单次覆盖优先且不泄漏到下一次调用', t => {
  const { root, project, env } = externalFixture(t);
  const result = run(root, initArgs);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(result.stdout).STORAGE, 'repository');
  fs.rmSync(path.join(root, 'specs'), { recursive: true });
  const override = path.join(root, 'one-off');
  fs.mkdirSync(override);
  const custom = run(root, initArgs, { ...env, QFY_SPEC_KIT_OBSIDIAN_ROOT: override });
  assert.equal(custom.status, 0, custom.stderr);
  const output = JSON.parse(custom.stdout);
  assert.equal(output.FEATURE_DIR, path.join(override, '001-example'));
  assert.equal(output.STORAGE, 'obsidian');
  assert.deepEqual(output.REQUIRED_READS, [path.join(override, '001-example/spec.md')]);
  assert.equal(fs.existsSync(path.join(root, 'specs')), false);
  assert.deepEqual(fs.readdirSync(project), []);
  assert.equal(JSON.parse(run(root, initArgs).stdout).STORAGE, 'repository');
});

test('项目级 Vault 初始化只写指定外部目录，续写不覆盖正文与签收状态', t => {
  const { root, project, env } = externalFixture(t);
  const directory = path.join(project, storageLocations[0], '001-example');
  const result = run(root, initArgs, env);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(result.stdout).FEATURE_DIR, directory);
  assert.equal(fs.existsSync(path.join(root, 'specs')), false);
  assert.equal(fs.existsSync(path.join(root, '.specify/feature.json')), false);
  fs.writeFileSync(path.join(directory, 'spec.md'), '# 需求\nFR-001 需求\n');
  fs.writeFileSync(path.join(directory, 'plan.md'), '# 方案\n## 验证\n单测\n');
  fs.writeFileSync(path.join(directory, 'tasks.md'), '# 任务\n- [ ] T001 验证；用户待签收\n');
  const before = fs.readFileSync(path.join(directory, 'tasks.md'), 'utf8');
  for (const stage of ['clarify', 'plan', 'tasks', 'analyze', 'implement', 'converge']) {
    const checked = run(root, ['check', '--feature', 'specs/001-example', '--stage', stage], env);
    assert.equal(checked.status, 0, checked.stderr);
    const output = JSON.parse(checked.stdout);
    assert.equal(output.FEATURE_DIR, directory);
    assert.equal(output.STORAGE, 'obsidian');
    assert.equal(output.REQUIRED_READS.length, 3);
  }
  assert.equal(run(root, initArgs, env).status, 0);
  assert.equal(fs.readFileSync(path.join(directory, 'tasks.md'), 'utf8'), before);
});

test('四个候选目录历史可读，仓库冻结 feature 可读；重名明确拒绝', t => {
  for (const location of storageLocations) {
    const { root, project, env } = externalFixture(t);
    const directory = path.join(project, location, '001-example');
    externalComplete(root, directory);
    const checked = run(root, checkArgs, env);
    assert.equal(checked.status, 0, checked.stderr);
    assert.equal(JSON.parse(checked.stdout).FEATURE_DIR, directory);
    const duplicate = path.join(project, storageLocations.find(item => item !== location), '001-example');
    fs.mkdirSync(duplicate, { recursive: true });
    assert.match(run(root, initArgs, env).stderr, /重复 feature/);
    fs.rmSync(duplicate, { recursive: true });
    complete(root);
    assert.match(run(root, checkArgs, env).stderr, /重复 feature/);
  }
  const { root, env } = externalFixture(t);
  complete(root);
  const checked = run(root, checkArgs, env);
  assert.equal(checked.status, 0, checked.stderr);
  assert.equal(JSON.parse(checked.stdout).STORAGE, 'repository');
});

test('外部根缺失或非目录不静默回退，check 不创建缺失产物', t => {
  const { root, vault, project, env } = externalFixture(t);
  const missing = run(root, checkArgs, env);
  assert.notEqual(missing.status, 0);
  assert.match(missing.stderr, /缺少产物/);
  assert.deepEqual(fs.readdirSync(project), []);
  for (const base of [path.join(root, 'missing'), path.join(root, 'file')]) {
    if (base.endsWith('file')) fs.writeFileSync(base, 'not directory');
    for (const setting of [{ QFY_OBSIDIAN_VAULT_ROOT: base }, { QFY_SPEC_KIT_OBSIDIAN_ROOT: base }]) {
      assert.notEqual(run(root, initArgs, setting).status, 0);
    }
  }
  fs.rmSync(project, { recursive: true });
  assert.notEqual(run(root, initArgs, env).status, 0);
  assert.equal(fs.existsSync(path.join(root, 'specs')), false);
  assert.equal(fs.existsSync(path.join(vault, 'QFY/项目/deploy-go')), false);
});

test('拒绝无效配置与越界路径，显式覆盖也不能掩盖坏配置', t => {
  const { root, vault, env } = externalFixture(t);
  const file = path.join(root, '.specify/spec-kit.json');
  const invalid = ['null', '[]', '{', '{"unexpected":true}', ...['../other', '/tmp/project', 'QFY/../other', 'QFY//other', '.', '', 12, 'QFY\\other'].map(value => JSON.stringify({ obsidian_project_path: value }))];
  for (const config of invalid) {
    fs.writeFileSync(file, config);
    const result = run(root, initArgs, { ...env, QFY_SPEC_KIT_OBSIDIAN_ROOT: vault });
    assert.notEqual(result.status, 0, config);
  }
  fs.writeFileSync(file, '{}');
  for (const value of ['relative', vault + '/../vault', '/', '/./', '//', '///']) {
    assert.notEqual(run(root, initArgs, { QFY_SPEC_KIT_OBSIDIAN_ROOT: value }).status, 0);
  }
  fs.unlinkSync(file);
  fs.symlinkSync(path.join(root, 'absent-config'), file);
  assert.match(run(root, initArgs).stderr, /普通文件/);
});

test('拒绝 Vault、项目、中间路径、feature 和产物的符号链接，包括悬空链接', t => {
  for (const part of ['vault', 'project', 'middle', 'feature', 'artifact']) {
    const { root, vault, project, env } = externalFixture(t);
    const directory = path.join(project, storageLocations[0], '001-example');
    externalComplete(root, directory);
    const targets = { vault, project, middle: path.join(project, '10-项目文档'), feature: directory, artifact: path.join(directory, 'plan.md') };
    const target = targets[part];
    const original = path.join(root, 'saved');
    fs.renameSync(target, original);
    fs.symlinkSync(original, target);
    const result = run(root, checkArgs, env);
    assert.notEqual(result.status, 0, part);
    assert.match(result.stderr, /符号链接|普通文件/);
    fs.unlinkSync(target);
    fs.symlinkSync(path.join(root, 'absent'), target);
    assert.notEqual(run(root, initArgs, env).status, 0, part + ' dangling');
  }
});

test('外部缺规格旧任务、非目录 feature 与非普通产物不能被初始化覆盖', t => {
  const { root, project, env } = externalFixture(t);
  const base = path.join(project, storageLocations[0]);
  fs.mkdirSync(base, { recursive: true });
  const directory = path.join(base, '001-example');
  fs.writeFileSync(directory, 'not a directory');
  assert.match(run(root, initArgs, env).stderr, /不是目录/);
  fs.unlinkSync(directory);
  fs.mkdirSync(directory);
  fs.writeFileSync(path.join(directory, 'tasks.md'), '# 旧任务\n- [ ] T001 验证\n');
  assert.match(run(root, initArgs, env).stderr, /已有计划或任务/);
  assert.equal(fs.existsSync(path.join(directory, 'spec.md')), false);
  fs.mkdirSync(path.join(directory, 'spec.md'));
  assert.match(run(root, initArgs, env).stderr, /普通文件/);
});

test('外部并发初始化排他创建且不写仓库或隐式指针', async t => {
  const { root, project, env } = externalFixture(t);
  const invoke = () => new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [entry, ...initArgs], { cwd: root, env: { ...process.env, QFY_SPEC_KIT_OBSIDIAN_ROOT: '', ...env } });
    let stderr = '';
    child.stderr.on('data', data => { stderr += data; });
    child.on('error', reject);
    child.on('close', status => resolve({ status, stderr }));
  });
  const results = await Promise.all([invoke(), invoke(), invoke()]);
  assert.ok(results.some(result => result.status === 0));
  for (const result of results) if (result.status !== 0) assert.match(result.stderr, /EEXIST/);
  const directory = path.join(project, storageLocations[0], '001-example');
  assert.equal(fs.readFileSync(path.join(directory, 'spec.md'), 'utf8'), fs.readFileSync(path.join(root, '.specify/templates/overrides/spec-template.md'), 'utf8'));
  assert.equal(fs.existsSync(path.join(root, 'specs')), false);
  assert.equal(fs.existsSync(path.join(root, '.specify/feature.json')), false);
});
