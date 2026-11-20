#!/usr/bin/env python3
"""deploy-go 远端构建与测试运行适配；源码身份、快照、传输与锁由 remote_sync 提供。

以 qfy-finance 的 scripts/dev/qfy_remote_runtime.py 为蓝本改写，按本项目登记命令：
  rust-check                     cargo fmt --check、clippy -D warnings、nextest 全 workspace、文档测试（有代码块时）、
                                 OpenAPI 产物漂移检查（等价本机 make api-check 中除镜像 grep 外的全部步骤）
  rust-fmt                       cargo fmt --all --check
  rust-clippy                    cargo clippy --workspace --all-targets -- -D warnings
  rust-test [nextest 参数]        cargo nextest run；不带参数为 --workspace；只接受白名单参数（见 _rust_test_args）
  web-check                      admin：lint、typecheck、vitest、build（等价本机 make admin-check）
  web-test [测试文件...]          admin：vitest run
所有容器只接入既有 dev-shared-network，不发布宿主端口、不创建网络或 Docker 卷；源码只读挂载。
需要特权的 Linux 专项（cgroup、runner 身份隔离）不在本入口：它们由环境变量门控，这里不传这些变量，会自动跳过。
"""
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import sys
import tempfile
import time

from remote_sync import OperationInterrupted, digest, json_bytes, make_dir, read_file, read_json, safe_dir, write_new

TOOLS = {'rust': 'Dockerfile.rust', 'node': 'Dockerfile.node'}
LABEL = 'qfy.dev.'
NETWORK = 'dev-shared-network'
# qfy-test2 同时承载 Deploy Go 正式环境与其他项目的构建：设硬上限，并把调度权重降到默认的一半，
# CPU 紧张时正式服务与其他容器优先。数值登记在 ~/ai-space/server/dev-environments/REGISTRY.md。
RUST_CPUS, RUST_MEMORY = 6, '12g'
NODE_CPUS, NODE_MEMORY = 4, '6g'
CPU_SHARES = '512'
WEB_MODULE = 'admin'
# admin/tsconfig.json 的 references；新增子项目时同步这里（本地回归会核对）。
TS_PROJECTS = ('tsconfig.app', 'tsconfig.node')
RUST_COMMANDS = {'rust-check', 'rust-fmt', 'rust-clippy', 'rust-test'}
WEB_COMMANDS = {'web-check', 'web-test'}
# 文档测试：整个 workspace 目前没有任何文档代码块，cargo test --doc 只会空编译（本机约 14 秒）；
# 出现代码块时自动恢复执行，因此新增文档测试不会被漏跑。
DOC_FENCE = r"grep -rqE '^[[:space:]]*(///|//!)[[:space:]]*```' --include='*.rs' ."
NEXTEST_FLAGS = {'--workspace', '--lib', '--bins', '--tests', '--no-fail-fast'}
NEXTEST_VALUE_FLAGS = {'-p', '--package', '--test', '-E', '--filterset', '--exclude', '--run-ignored'}
SAFE_TOKEN = re.compile(r'[A-Za-z0-9_:.=()&|!~/ ,*<>-]{1,200}')
SAFE_FILE = re.compile(r'[A-Za-z0-9_./-]{1,200}')
# 工具输出里的终端颜色控制符；解析汇总行前去掉，否则会因匹配不到而误判失败。
ANSI = re.compile(r'\x1b\[[0-9;]*[A-Za-z]')


def _safe_value(value, what):
    if not SAFE_TOKEN.fullmatch(value) or value.startswith('/') or '..' in value.split('/'):
        raise ValueError(what + '含不允许的字符或路径')


def _rust_test_args(args):
    """只放行选择用例的参数；不允许替换 Cargo/工具配置、目标目录、清单或执行器。"""
    index = 0
    while index < len(args):
        value = args[index]
        if value in NEXTEST_VALUE_FLAGS:
            if index + 1 >= len(args):
                raise ValueError(value + ' 缺少取值')
            _safe_value(args[index + 1], value + ' 的取值')
            index += 2
            continue
        if value.startswith('-'):
            if value not in NEXTEST_FLAGS:
                raise ValueError('rust-test 不接受参数：' + value)
        else:
            _safe_value(value, '用例过滤串')
        index += 1


def checked_command(argv):
    if not argv:
        raise ValueError('需要运行命令：' + ', '.join(sorted(RUST_COMMANDS | WEB_COMMANDS)))
    command, *args = argv
    if command in ('rust-check', 'rust-fmt', 'rust-clippy', 'web-check'):
        if args:
            raise ValueError(command + ' 不接受参数')
        return command, args
    if command == 'rust-test':
        _rust_test_args(args)
        return command, args
    if command == 'web-test':
        for value in args:
            if value.startswith('-') or not SAFE_FILE.fullmatch(value) or value.startswith('/') or '..' in value.split('/'):
                raise ValueError('web-test 只接受 admin 下的测试文件相对路径')
        return command, args
    raise ValueError('未登记命令：' + command)


class Runtime:
    def __init__(self):
        self.request = read_json(Path(os.environ['QFY_SYNC_REQUEST']))
        self.root = safe_dir(Path(os.environ['QFY_SYNC_ROOT']))
        self.source = safe_dir(self.root / 'source')
        self.evidence = safe_dir(Path(os.environ['QFY_SYNC_EVIDENCE']))
        self.project_root = safe_dir(Path(os.environ['QFY_SYNC_PROJECT_ROOT']))
        self.instance = self.request['instance']
        self.run = self.request['run_id'][:8]
        self.containers = set()
        self.output = []
        self.receipt = {'toolchain': {'python': platform.python_version(), 'architecture': platform.machine()},
                        'docker_context': 'qfy-test2'}
        # 私有客户端配置：不读取宿主 Docker 登录凭据，不修改全局 context。
        config = make_dir(self.root / 'runtime' / 'docker-client')
        prefix = ['docker', '--config', str(config)]
        self.environment = {k: v for k, v in os.environ.items() if not k.startswith('DOCKER_')}
        if subprocess.run([*prefix, 'context', 'inspect', 'qfy-test2'], env=self.environment,
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
            subprocess.run([*prefix, 'context', 'create', 'qfy-test2', '--docker', 'host=unix:///var/run/docker.sock'],
                           env=self.environment, stdout=subprocess.DEVNULL, check=True)
        self.docker = [*prefix, '--context', 'qfy-test2']
        network = json.loads(subprocess.check_output([*self.docker, 'network', 'inspect', NETWORK],
                                                     env=self.environment, stderr=subprocess.DEVNULL))[0]
        if network['Name'] != NETWORK:
            raise ValueError('共享开发网络名称不符')
        self.receipt['network'] = {'name': network['Name'], 'id': network['Id']}

    def labels(self, role):
        values = []
        for key, value in [('project', self.request['project']), ('instance', self.instance),
                           ('branch', self.request['branch']), ('run', self.request['run_id']), ('role', role)]:
            values += ['--label', LABEL + key + '=' + value]
        return values

    def call(self, args, check=False):
        with subprocess.Popen([*self.docker, *args], env=self.environment,
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT) as process:
            try:
                for raw in process.stdout:
                    text = raw.decode(errors='replace')
                    self.output.append(text)
                    sys.stdout.write(text)
                    sys.stdout.flush()
                code = process.wait()
            except BaseException:
                process.terminate()
                process.wait(timeout=10)
                raise
        if code and check:
            raise RuntimeError('Docker 步骤失败，退出码 ' + str(code))
        return code

    def tool(self, kind):
        dockerfile = read_file(self.source / 'deploy/dev-tools' / TOOLS[kind])[0]
        key = digest(json_bytes({'Dockerfile': digest(dockerfile)}))
        image = self.request['project'] + '-dev-tools-' + kind + ':' + key[:24]
        probe = subprocess.run([*self.docker, 'image', 'inspect', image, '--format', '{{.Id}}'],
                               env=self.environment, capture_output=True, text=True)
        if probe.returncode:
            print('构建工具镜像 ' + image + '（只在 Dockerfile 变化后执行一次）', flush=True)
            with tempfile.TemporaryDirectory(prefix='build-', dir=self.root / 'runtime') as folder:
                write_new(Path(folder) / 'Dockerfile', dockerfile)
                # 经典 builder 的 RUN 使用现有共享网络，避免隐式创建项目私网。
                self.environment['DOCKER_BUILDKIT'] = '0'
                try:
                    self.call(['build', '--force-rm', '--network', NETWORK, '--cpu-shares', CPU_SHARES,
                               '--label', LABEL + 'project=' + self.request['project'], '--label', LABEL + 'role=tools',
                               '--tag', image, '--file', str(Path(folder) / 'Dockerfile'), folder], check=True)
                finally:
                    self.environment.pop('DOCKER_BUILDKIT', None)
            image_id = subprocess.check_output([*self.docker, 'image', 'inspect', image, '--format', '{{.Id}}'],
                                               env=self.environment, text=True).strip()
        else:
            image_id = probe.stdout.strip()
        self.receipt['toolchain'][kind] = {'image': image, 'id': image_id}
        return image

    def cache(self, kind, extra=None):
        """可重建的依赖与编译缓存，放在 /opt/qfy-dev/deploy-go/cache/<kind>/<key>，按工具镜像与架构划分。

        Rust 的 target 不按 Cargo.lock 划分：cargo 按指纹增量判断是否重编，锁文件变化时只重编受影响的 crate；
        若按锁文件划分，每次依赖升级都会冷编译全部依赖（十几分钟）并遗留几十 GB 旧目录。"""
        key = digest(json_bytes({'tool': self.receipt['toolchain'][kind], 'architecture': platform.machine(),
                                 'extra': extra or {}}))
        self.receipt.setdefault('cache_keys', {})[kind] = key
        return make_dir(self.project_root / 'cache' / kind / key)

    def run_container(self, image, argv, mounts, workdir, cpus, memory, extra=()):
        name = self.instance + '-run-' + self.run
        if subprocess.run([*self.docker, 'container', 'inspect', name], env=self.environment,
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
            raise ValueError('同名运行容器已存在，拒绝接管')
        args = ['run', '--rm', '--name', name, '--init', '--cpus', str(cpus), '--cpu-shares', CPU_SHARES,
                '--memory', memory, '--pids-limit', '4096', '--security-opt', 'no-new-privileges',
                '--log-driver', 'local', '--log-opt', 'max-size=10m', '--log-opt', 'max-file=3',
                *self.labels('runner'), '--workdir', workdir, '-e', 'CI=1', '--network', NETWORK,
                *mounts, *extra, image, *argv]
        self.containers.add(name)
        try:
            return self.call(args)
        finally:
            subprocess.run([*self.docker, 'rm', '--force', name], env=self.environment,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            self.containers.discard(name)

    # ---- Rust ----

    def rust_tree(self):
        """给 Cargo 用的构建目录：内容变化的文件才写入并刷新修改时间，未变化的保持原样。

        通用协议为保证快照可复现，把同步过来的所有文件修改时间都设为 1（1970 年）。Cargo 按修改时间判断是否
        重编：改过的文件时间仍是 1970 年，早于上次编译产物，会被当成“没变”而沿用旧产物，测试结果不可信。
        这里按快照清单的内容摘要逐文件对比，只改写变化的文件，并删除清单里已不存在的文件。"""
        tree = make_dir(self.root / 'runtime' / 'rust-tree')
        state_path = self.root / 'runtime' / 'rust-tree.json'
        previous = read_json(state_path) if state_path.exists() else {}
        files = read_json(self.source / '.qfy-source.json')['files']
        changed = removed = 0
        for name in [*files, *previous]:
            if name.startswith('/') or '..' in name.split('/') or '\x00' in name:
                raise ValueError('清单路径越界：' + name)
        for name, info in files.items():
            if previous.get(name) == info['sha256'] and (tree / name).is_file():
                continue
            data = read_file(self.source / name)[0]
            if digest(data) != info['sha256']:
                raise ValueError('源码与快照清单不一致：' + name)
            target = tree / name
            make_dir(target.parent)
            staging = target.with_name('.' + target.name + '.deploy-go-tmp')
            if staging.exists():
                staging.unlink()
            write_new(staging, data)
            staging.chmod(0o755 if info['executable'] else 0o644)
            os.replace(staging, target)
            changed += 1
        for name in set(previous) - set(files):
            path = tree / name
            if path.is_file() and not path.is_symlink():
                path.unlink()
                removed += 1
        replace = state_path.with_name(state_path.name + '.tmp')
        if replace.exists():
            replace.unlink()
        write_new(replace, json_bytes({name: info['sha256'] for name, info in files.items()}))
        os.replace(replace, state_path)
        self.receipt['rust_tree'] = {'changed': changed, 'removed': removed, 'files': len(files)}
        print('构建目录：更新 %d 个文件，删除 %d 个，共 %d 个' % (changed, removed, len(files)), flush=True)
        return tree

    def rust(self, command, args):
        image = self.tool('rust')
        cache = self.cache('rust')
        tree = self.rust_tree()
        mounts = ['--mount', 'type=bind,source=' + str(tree) + ',target=/workspace,readonly']
        for name, target in [('registry', '/usr/local/cargo/registry'), ('git', '/usr/local/cargo/git'),
                             ('target', '/cache/target')]:
            mounts += ['--mount', 'type=bind,source=' + str(make_dir(cache / name)) + ',target=' + target]
        jobs = str(RUST_CPUS)
        # (固定标签, 命令)：标签只含字母，打印时不经过 shell 解释；命令里的引号与反引号不进入 printf。
        steps = {
            'rust-fmt': [('fmt', 'cargo fmt --all --check')],
            'rust-clippy': [('clippy', 'cargo clippy --locked --workspace --all-targets -- -D warnings')],
            'rust-test': [('nextest', 'cargo nextest run --locked --no-fail-fast "$@"')],
        }
        # 与本机 make api-check 对齐：fmt、clippy、全 workspace 测试、文档测试，以及 OpenAPI 产物漂移检查
        # （openapi-check 同时比对 api/openapi/openapi.json 与 external.json）。
        steps['rust-check'] = [*steps['rust-fmt'], *steps['rust-clippy'],
                               ('nextest', 'cargo nextest run --locked --workspace --no-fail-fast'),
                               ('doctest', 'if ' + DOC_FENCE + '; then cargo test --locked --workspace --doc; '
                                           'else echo "文档测试：workspace 没有文档代码块，跳过"; fi'),
                               ('openapi', 'cargo run --locked --quiet -p deploy-go-api -- openapi-check')]
        if command == 'rust-test' and not args:
            args = ['--workspace']
        script = 'set -eu\n' + ''.join(
            'printf "\\n==== %s ====\\n" ' + label + '\nstart=$(date +%s)\n' + step +
            '\nprintf "==== %s 耗时 %ss ====\\n" ' + label + ' "$(( $(date +%s) - start ))"\n'
            for label, step in steps[command])
        self.output = []
        started = time.monotonic()
        code = self.run_container(image, ['sh', '-c', script, 'deploy-go-rust', *args], mounts, '/workspace',
                                  RUST_CPUS, RUST_MEMORY,
                                  extra=['-e', 'CARGO_TARGET_DIR=/cache/target', '-e', 'CARGO_BUILD_JOBS=' + jobs,
                                         '-e', 'NEXTEST_TEST_THREADS=' + jobs, '-e', 'CARGO_INCREMENTAL=0',
                                         '-e', 'CARGO_TERM_COLOR=never'])
        text = ANSI.sub('', ''.join(self.output))
        summary = re.findall(r'Summary \[\s*([0-9.]+)s\]\s+(\d+) tests? run: (\d+) passed', text)
        evidence = {'command': command, 'seconds': round(time.monotonic() - started, 1)}
        if summary:
            seconds, ran, passed = summary[-1]
            evidence.update(nextest_seconds=float(seconds), tests_run=int(ran), tests_passed=int(passed))
        self.receipt['business_evidence'] = evidence
        if code == 0 and command in ('rust-check', 'rust-test') and not summary:
            print('没有找到 nextest 汇总行，按失败处理', file=sys.stderr)
            return 1
        if code == 0 and summary and int(summary[-1][1]) == 0:
            print('没有运行任何用例（过滤条件可能写错），按失败处理', file=sys.stderr)
            return 1
        return code

    # ---- Web ----

    def web(self, command, args):
        image = self.tool('node')
        manifest = read_json(self.source / '.qfy-source.json')
        locks = {name: info for name, info in manifest['files'].items()
                 if name in (WEB_MODULE + '/package.json', WEB_MODULE + '/package-lock.json')}
        lock_digest = digest(json_bytes(locks))
        cache = self.cache('node', {'locks': lock_digest})
        modules = make_dir(cache / 'node_modules')
        # 只读源码里只为挂载点预建空目录/空文件；这些路径都在同步排除规则里，不会进入源码清单。
        make_dir(self.source / WEB_MODULE / 'node_modules')
        make_dir(self.source / WEB_MODULE / 'dist')
        outputs = make_dir(self.root / 'runtime' / 'outputs' / WEB_MODULE)
        dist = make_dir(outputs / 'dist')
        mounts = ['--mount', 'type=bind,source=' + str(self.source) + ',target=/workspace,readonly',
                  '--mount', 'type=bind,source=' + str(modules) + ',target=/workspace/' + WEB_MODULE + '/node_modules',
                  '--mount', 'type=bind,source=' + str(dist) + ',target=/workspace/' + WEB_MODULE + '/dist',
                  '--mount', 'type=bind,source=' + str(make_dir(cache / 'npm')) + ',target=/root/.npm']
        # tsc -b 为 tsconfig.json 引用的每个子项目在 admin/ 下写 <名>.tsbuildinfo；源码只读，逐个映射到可写文件。
        for project in TS_PROJECTS:
            name = project + '.tsbuildinfo'
            if not (self.source / WEB_MODULE / name).exists():
                write_new(self.source / WEB_MODULE / name, b'')
            writable = outputs / name
            if not writable.exists():
                write_new(writable, b'')
            mounts += ['--mount', 'type=bind,source=' + str(writable) + ',target=/workspace/' + WEB_MODULE + '/' + name]
        # node_modules 按锁文件摘要缓存，标记一致时跳过 npm ci。
        # --ignore-scripts：postinstall 只用于下载 OpenAPI 生成器（API client 漂移检查仍在本机执行），远端不需要。
        install = ('if [ "$(cat node_modules/.deploy-go-lock 2>/dev/null)" != "$DEPLOY_GO_LOCK" ]; then '
                   'npm ci --ignore-scripts && echo "$DEPLOY_GO_LOCK" > node_modules/.deploy-go-lock; fi\n')
        if command == 'web-check':
            run = ('npm run lint\nnpm run typecheck\nnpx vitest run --maxWorkers=' + str(NODE_CPUS) +
                   '\nnpx vite build\n')
            argv = []
        else:
            run = 'exec npx vitest run --maxWorkers=' + str(NODE_CPUS) + ' "$@"\n'
            argv = list(args)
        shell = 'set -eu\ncd "/workspace/' + WEB_MODULE + '"\n' + install + run
        self.output = []
        started = time.monotonic()
        code = self.run_container(image, ['sh', '-c', shell, 'deploy-go-web', *argv], mounts, '/workspace/' + WEB_MODULE,
                                  NODE_CPUS, NODE_MEMORY, extra=['-e', 'DEPLOY_GO_LOCK=' + lock_digest, '-e', 'NO_COLOR=1',
                                                                         '-e', 'FORCE_COLOR=0'])
        text = ANSI.sub('', ''.join(self.output))
        tests = re.findall(r'Tests\s+(\d+) passed \((\d+)\)', text)
        evidence = {'command': command, 'seconds': round(time.monotonic() - started, 1)}
        if tests:
            evidence.update(tests_passed=int(tests[-1][0]), tests_total=int(tests[-1][1]))
        self.receipt['business_evidence'] = evidence
        if code == 0 and not tests:
            print('没有找到 vitest 汇总行，按失败处理', file=sys.stderr)
            return 1
        return code

    def close(self):
        for name in list(self.containers):
            subprocess.run([*self.docker, 'rm', '--force', name], env=self.environment,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def main(argv):
    command, args = checked_command(argv)
    runtime = Runtime()

    def interrupted(signum, _frame):
        raise OperationInterrupted(signum)
    for signum in [signal.SIGTERM, signal.SIGINT, signal.SIGHUP]:
        signal.signal(signum, interrupted)
    try:
        if command in RUST_COMMANDS:
            return runtime.rust(command, args)
        return runtime.web(command, args)
    finally:
        runtime.close()
        write_new(runtime.evidence / 'runtime-result.json', json_bytes(runtime.receipt))


if __name__ == '__main__':
    os.umask(0o077)
    try:
        raise SystemExit(main(sys.argv[1:]))
    except (ValueError, RuntimeError, OSError) as error:
        print('deploy-go 远端运行失败：' + str(error), file=sys.stderr)
        raise SystemExit(2)
    except subprocess.SubprocessError as error:
        print('deploy-go 远端运行失败：子进程 ' + type(error).__name__, file=sys.stderr)
        raise SystemExit(2)
