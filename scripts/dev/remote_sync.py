#!/usr/bin/env python3
"""参数化 SSH/rsync 同步协议（deploy-go）。

自 qfy-finance 的 scripts/dev/remote_sync.py（md5 c1cffde9e8fa36b1534cf5c1f1230c97）原样复制，只改本段文件头说明；
qfy-finance 版本又自 qfy-sc @ 56c8d0607759efa2df4bde1d5310689d84cb3131 复制。相对 qfy-sc 的唯一改动：
合同可用 include_tracked 列出 Git 已跟踪的公开环境模板（如 api/.env.example），越过 .env* 排除规则同步；
真实 .env 与未跟踪文件仍一律排除。deploy-go 的项目差异只放在 scripts/dev/remote-test.json 与 scripts/dev/deploy_go_remote_runtime.py。
独立维护协议版本，不调用其他项目的脚本或资源。"""
import argparse
import base64
import contextlib
import datetime
import fcntl
import fnmatch
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import shlex
import shutil
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import uuid

PROTOCOL = 1
class OperationInterrupted(RuntimeError):
    """不要使用 InterruptedError；subprocess/IO 会将其作为 EINTR 自动重试。"""

MANIFEST = '.qfy-source.json'
EXCLUDES = [
    '.git', '.codex', '.claude', '.context', '.local', '.local-data', '.playwright-mcp',
    'node_modules', 'vendor', '__pycache__', '.venv', 'venv', '.npm', '.cache', '.pytest_cache', '.ruff_cache', '.mypy_cache',
    '.nuxt', '.output', '.vite', '.turbo', 'target', 'tmp', 'dist', 'build',
    'coverage', 'test-results', 'playwright-report', 'exports',
    '/config', '/runtime', '/evidence', '.ssh', '.aws', '.kube', '.secrets',
    '.npmrc', '.netrc', 'id_rsa', 'id_ed25519', '.env*', '*.pem', '*.key',
    '*.p12', '*.pfx', '*.sqlite*', '*.db', '*.bak', '*.log', '*.tmp', '*.pyc',
    '.DS_Store', '*.exe', '*.test', '*.out', '*.tsbuildinfo',
]


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode()


def digest(value):
    return hashlib.sha256(value).hexdigest()


def encoded(value):
    return base64.b64encode(json_bytes(value)).decode()


def utc():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def safe_dir(path):
    path = Path(path)
    if not path.is_absolute() or '..' in path.parts:
        raise ValueError('目录必须为规范绝对路径')
    for current in [*reversed(path.parents), path]:
        if current.is_symlink() or not current.is_dir():
            raise ValueError('目录缺失或包含符号链接')
    return path


def make_dir(path):
    path = Path(path)
    if not path.parent.exists():
        make_dir(path.parent)
    safe_dir(path.parent)
    try:
        path.mkdir(mode=0o700)
    except FileExistsError:
        pass
    return safe_dir(path)


def read_file(path, *, dir_fd=None):
    """不跟随链接；读取过程中修改、替换文件时拒绝。"""
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=dir_fd)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
            raise ValueError('只接受单链接普通文件')
        with os.fdopen(fd, 'rb', closefd=False) as stream:
            value = stream.read()
        after = os.fstat(fd)
        fields = ('st_dev', 'st_ino', 'st_mode', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
        if any(getattr(before, k) != getattr(after, k) for k in fields):
            raise ValueError('文件在读取期间变化')
        return value, bool(before.st_mode & 0o111)
    finally:
        os.close(fd)


def read_json(path):
    return json.loads(read_file(path)[0])


def write_new(path, value):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'wb') as stream:
        stream.write(value)


def replace_json(path, value):
    temporary = path.with_name(path.name + '-' + uuid.uuid4().hex)
    write_new(temporary, json_bytes(value))
    os.replace(temporary, path)


def process_identity(pid):
    # starttime 防止取消请求误伤复用 PID；macOS 兼容用于本机协议自测。
    if Path('/proc').is_dir():
        return Path('/proc', str(pid), 'stat').read_text().rsplit(')', 1)[1].split()[19]
    return subprocess.check_output(['ps', '-o', 'lstart=', '-p', str(pid)], text=True).strip()


def policy_checked(value):
    value = dict(value)
    if value.get('protocol') != PROTOCOL or not re.fullmatch(r'[a-z][a-z0-9-]{1,23}', value.get('project', '')):
        raise ValueError('项目或协议版本非法')
    if not re.fullmatch(r'[a-zA-Z0-9][a-zA-Z0-9.-]*', value.get('host', '')):
        raise ValueError('SSH 目标必须为明确主机别名')
    if value.get('remote_root') != '/opt/qfy-dev/' + value['project']:
        raise ValueError('远端根目录不符合项目映射')
    adapter = PurePosixPath(value.get('adapter', ''))
    if adapter.is_absolute() or '..' in adapter.parts or adapter.suffix != '.py':
        raise ValueError('运行适配器路径非法')
    extras = value.get('extra_excludes', [])
    if not isinstance(extras, list) or any(not isinstance(x, str) or not x or '\n' in x for x in extras):
        raise ValueError('额外排除规则非法')
    # 仅允许带后缀的环境模板（.env.example、.env.production 等），不允许裸 .env。
    includes = value.get('include_tracked', [])
    if not isinstance(includes, list) or any(not isinstance(x, str) or not re.fullmatch(r'(?:[a-z0-9_-]+/)*\.env\.[a-z]+', x) for x in includes):
        raise ValueError('include_tracked 只能列出带后缀的环境模板相对路径')
    for key, maximum in [('timeout_seconds', 7200), ('sync_timeout_seconds', 1800)]:
        if type(value.get(key)) is not int or not 1 <= value[key] <= maximum:
            raise ValueError('运行/同步超时非法')
    return value


def excludes(policy):
    return EXCLUDES + policy.get('extra_excludes', [])


def excluded(relative, policy):
    if relative in policy.get('include_tracked', []):
        return False
    parts = PurePosixPath(relative).parts
    for pattern in excludes(policy):
        if pattern.startswith('/'):
            if fnmatch.fnmatchcase('/' + relative, pattern) or any(
                    fnmatch.fnmatchcase('/' + '/'.join(parts[:i]), pattern) for i in range(1, len(parts))):
                return True
        elif any(fnmatch.fnmatchcase(part, pattern) for part in parts):
            return True
    return False


@contextlib.contextmanager
def directory_fd(path):
    """逐级打开目录，目录被替换为链接时也不穿透。"""
    path = Path(path)
    if not path.is_absolute() or '..' in path.parts:
        raise ValueError('目录必须为规范绝对路径')
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY)
    try:
        for part in path.parts[1:]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = child
        yield fd
    finally:
        os.close(fd)


def inventory(root, policy, copy_to=None):
    root = safe_dir(root)
    result = {}
    def visit(fd, prefix=''):
        before = os.fstat(fd)
        for name in sorted(os.listdir(fd)):
            relative = prefix + name
            if relative == MANIFEST or excluded(relative, policy):
                continue
            mode = os.stat(name, dir_fd=fd, follow_symlinks=False).st_mode
            if stat.S_ISDIR(mode):
                child = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
                try:
                    visit(child, relative + '/')
                finally:
                    os.close(child)
            elif stat.S_ISREG(mode):
                value, executable = read_file(name, dir_fd=fd)
                result[relative] = {'sha256': digest(value), 'executable': executable}
                if copy_to is not None:
                    target = copy_to / relative
                    target.parent.mkdir(parents=True, exist_ok=True)
                    write_new(target, value)
                    target.chmod(0o700 if executable else 0o600)
                    os.utime(target, (1, 1))
            else:
                raise ValueError('源码包含符号链接或非普通文件：' + relative)
        after = os.fstat(fd)
        if any(getattr(before, key) != getattr(after, key) for key in ('st_dev', 'st_ino', 'st_mtime_ns', 'st_ctime_ns')):
            raise ValueError('源码目录在读取期间变化')
    with directory_fd(root) as fd:
        visit(fd)
    return result


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], stderr=subprocess.DEVNULL).decode().strip()


def branch_key(project, branch):
    slug = re.sub('[^a-z0-9]+', '-', branch.lower()).strip('-') or 'branch'
    budget = 48 - len(project) - len('-dev') - 1 - 9
    return slug[:budget].rstrip('-') + '-' + digest(branch.encode())[:8]


def identity(root, policy, commit=None):
    root = safe_dir(Path(root).absolute())
    if Path(git(root, 'rev-parse', '--show-toplevel')).resolve() != root:
        raise ValueError('入口必须绑定仓库根目录')
    try:
        branch = git(root, 'symbolic-ref', '--quiet', '--short', 'HEAD')
    except subprocess.CalledProcessError:
        if not commit:
            raise ValueError('日常同步不接受 detached HEAD') from None
        branch = 'commit/' + commit
    key = branch_key(policy['project'], branch)
    return {'protocol': PROTOCOL, 'project': policy['project'], 'branch': branch,
            'branch_key': key, 'instance': policy['project'] + '-' + key + '-dev',
            'checkout': str(root), 'head': git(root, 'rev-parse', 'HEAD'),
            'dirty': bool(git(root, 'status', '--porcelain', '--untracked-files=all'))}


def materialize(root, policy, commit, stage, pathspec=(), prefix=''):
    """把固定提交（可限定路径）的普通文件写入 stage/prefix；返回写入的相对路径。"""
    entries = subprocess.check_output(['git', '-C', str(root), 'ls-tree', '-rz', '--full-tree', commit, '--', *pathspec]).split(b'\0')
    paths = []
    for entry in entries:
        if not entry:
            continue
        header, raw_name = entry.split(b'\t', 1)
        name = raw_name.decode()
        if excluded(name, policy):
            continue
        if header.split()[0] not in (b'100644', b'100755') or PurePosixPath(name).is_absolute() or '..' in PurePosixPath(name).parts:
            raise ValueError('固定提交包含不支持的文件类型/路径')
        paths.append(name)
    base = stage / prefix if prefix else stage
    for offset in range(0, len(paths), 150):
        data = subprocess.check_output(['git', '-C', str(root), 'archive', '--format=tar', commit, '--', *paths[offset:offset + 150]])
        with tarfile.open(fileobj=io.BytesIO(data)) as archive:
            for item in archive:
                if item.isdir():
                    continue
                if not item.isfile() or item.name not in paths[offset:offset + 150]:
                    raise ValueError('Git 归档文件类型不符')
                target = base / item.name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.extractfile(item).read())
                target.chmod(0o700 if item.mode & 0o111 else 0o600)
                os.utime(target, (1, 1))
    return paths


OVERLAY_PREFIX = re.compile(r'\.qfy-overlay-[a-z0-9-]{1,40}')


def snapshot(root, policy, commit=None, after_copy=None, overlay=None):
    """overlay={'prefix': '.qfy-overlay-<名>', 'commit': <完整SHA>, 'paths': [...]}：额外把另一提交的指定路径放进快照保留目录，
    随清单一起校验与同步（用于升级演练等需要基线源码的命令）；只在显式传入时生效。"""
    root = safe_dir(Path(root).absolute())
    before = identity(root, policy, commit)
    if commit and (not re.fullmatch('[a-f0-9]{40}', commit) or git(root, 'rev-parse', commit + '^{commit}') != commit):
        raise ValueError('verify 必须指定存在的完整 commit SHA')
    if overlay is not None:
        if (not isinstance(overlay, dict) or not OVERLAY_PREFIX.fullmatch(str(overlay.get('prefix', '')))
                or not re.fullmatch('[a-f0-9]{40}', str(overlay.get('commit', '')))
                or git(root, 'rev-parse', overlay['commit'] + '^{commit}') != overlay['commit']
                or not overlay.get('paths') or any(not isinstance(x, str) or not re.fullmatch(r'[A-Za-z0-9_][A-Za-z0-9._/-]{0,100}', x)
                                                   or '..' in x.split('/') for x in overlay['paths'])):
            raise ValueError('附加基线源码参数非法')
        if (root / overlay['prefix']).exists():
            raise ValueError('仓库内已存在保留目录：' + overlay['prefix'])
    # macOS 的系统临时目录经 /var 链接到 /private/var；仅规范化自己刚创建的目录。
    stage = Path(tempfile.mkdtemp(prefix='qfy-source-')).resolve(strict=True)
    stage.chmod(0o700)
    try:
        if commit:
            materialize(root, policy, commit, stage)
            before.update(head=commit, dirty=False, kind='fixed-commit')
        else:
            for path in policy.get('include_tracked', []):
                # 只放行 Git 已跟踪的模板，防止同名的本地私有文件被同步。
                if subprocess.run(['git', '-C', str(root), 'ls-files', '--error-unmatch', '--', path],
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
                    raise ValueError('include_tracked 文件未被 Git 跟踪：' + path)
            files = inventory(root, policy, copy_to=stage)
            if after_copy:
                after_copy()
            if before != identity(root, policy) or files != inventory(root, policy) or files != inventory(stage, policy):
                raise ValueError('源码或分支在快照期间变化')
            before['kind'] = 'development-snapshot'
        if overlay is not None:
            materialize(root, policy, overlay['commit'], stage, overlay['paths'], overlay['prefix'])
            marker = stage / overlay['prefix'] / 'COMMIT'
            marker.write_text(overlay['commit'] + '\n')
            marker.chmod(0o600)
            os.utime(marker, (1, 1))
            before['overlay'] = {'prefix': overlay['prefix'], 'commit': overlay['commit'], 'paths': list(overlay['paths'])}
        manifest = {**before, 'run_id': uuid.uuid4().hex, 'files': inventory(stage, policy)}
        manifest['snapshot_id'] = digest(json_bytes(manifest['files']))
        data = json_bytes(manifest)
        write_new(stage / MANIFEST, data)
        return stage, {k: v for k, v in manifest.items() if k != 'files'} | {'manifest_sha256': digest(data)}
    except BaseException:
        shutil.rmtree(stage)
        raise


def binding(request):
    return {key: request[key] for key in ['protocol', 'project', 'branch', 'branch_key', 'instance', 'checkout', 'kind']}


def target_root(request, policy):
    if request.get('protocol') != PROTOCOL or request.get('project') != policy['project']:
        raise ValueError('接收项目或协议不匹配')
    if request.get('branch_key') != branch_key(policy['project'], request.get('branch', '')):
        raise ValueError('分支映射不符')
    if not Path(request.get('checkout', '')).is_absolute():
        raise ValueError('本地目录绑定非法')
    if not re.fullmatch('[a-f0-9]{32}', request.get('run_id', '')) or not re.fullmatch('[a-f0-9]{40}', request.get('head', '')):
        raise ValueError('运行编号或源码提交非法')
    base = Path(policy['remote_root'])
    if request['kind'] == 'development-snapshot':
        if request['instance'] != policy['project'] + '-' + request['branch_key'] + '-dev':
            raise ValueError('开发实例映射非法')
        return base / 'dev' / request['branch_key']
    if request['kind'] != 'fixed-commit' or request['dirty'] is not False:
        raise ValueError('固定提交身份非法')
    expected = policy['project'] + '-v-' + request['head'][:12] + '-' + request['run_id'][:8]
    if request['instance'] != expected:
        raise ValueError('验收实例映射非法')
    return base / 'verify' / request['head'] / request['run_id']


def check_binding(root, request):
    safe_dir(root)
    if root.stat().st_uid != os.getuid() or root.stat().st_mode & 0o022:
        raise ValueError('远端目录归属或权限不符')
    if read_json(root / 'binding.json') != binding(request):
        raise ValueError('分支已绑定其他 checkout，拒绝覆盖；请先规范分支或显式交接')
    for name in ['source', 'config', 'runtime', 'evidence']:
        safe_dir(root / name)


@contextlib.contextmanager
def operation_lock(root):
    safe_dir(root)
    fd = os.open(root / 'operation.lock', os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK, 0o600)
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_uid != os.getuid() or info.st_mode & 0o022:
            raise ValueError('锁文件归属或类型不符')
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise ValueError('目标分支正在同步或测试，请稍后重试') from None
        yield
    finally:
        os.close(fd)


def rsync_version(argv):
    value = subprocess.check_output([*argv, '--version'], stderr=subprocess.STDOUT, text=True)
    match = re.search(r'protocol(?: version)?\s+(\d+)', value)
    if not match or int(match[1]) < 29:
        raise ValueError('rsync 必须支持 protocol 29 或以上')
    return value.splitlines()[0]


def install(request, policy, files):
    policy_checked(policy)
    root = target_root(request, policy)
    make_dir(root.parent)
    try:
        root.mkdir(mode=0o700)
    except FileExistsError:
        check_binding(root, request)
    else:
        for name in ['source', 'config', 'runtime', 'evidence']:
            (root / name).mkdir(mode=0o700)
        write_new(root / 'binding.json', json_bytes(binding(request)))
    with operation_lock(root):
        check_binding(root, request)
        rsync_version(['rsync'])
        bundle = digest(json_bytes(files))
        directory = make_dir(root / 'runtime' / ('tools-' + bundle))
        for name, content in files.items():
            if not re.fullmatch(r'[a-zA-Z0-9_-]+\.(py|json)', name):
                raise ValueError('工具包文件名非法')
            value = base64.b64decode(content, validate=True)
            target = directory / name
            if target.exists() or target.is_symlink():
                if read_file(target)[0] != value:
                    raise ValueError('工具包摘要冲突')
            else:
                write_new(target, value)
        return {'root': str(root), 'bundle': bundle, 'helper': str(directory / 'remote_sync.py')}


def verify_snapshot(root, request, policy):
    source = safe_dir(root / 'source')
    if (source / '.git').exists() or (source / '.git').is_symlink():
        raise ValueError('source 不得含 Git 工作区或指针')
    raw = read_file(source / MANIFEST)[0]
    if digest(raw) != request['manifest_sha256']:
        raise ValueError('同步清单摘要不一致')
    manifest = json.loads(raw)
    if any(manifest.get(k) != request[k] for k in ['head', 'dirty', 'run_id', 'snapshot_id', *binding(request)]):
        raise ValueError('同步来源绑定不一致')
    if manifest['files'] != inventory(source, policy):
        raise ValueError('同步文件清单不一致，停止执行')
    return manifest


def terminate(process):
    if process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
    except ProcessLookupError:
        process.wait()


def managed_run(argv, timeout, **kwargs):
    with subprocess.Popen(argv, start_new_session=True, **kwargs) as process:
        try:
            return process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            terminate(process)
            return 124
        except BaseException:
            terminate(process)
            raise


def receive(request, server_args, policy, transfer=None):
    root = target_root(request, policy)
    check_binding(root, request)
    if not server_args or server_args[0] != '--server' or server_args[-2:] != ['.', str(root / 'source') + '/']:
        raise ValueError('rsync 目标越界')
    # 参数来自本机固定客户端；额外文件列表、发送端及链接跟随均不允许。
    if any(x.startswith(('--sender', '--delete-excluded', '--copy-links', '--keep-dirlinks', '--remove-source-files', '--files-from', '--include-from', '--exclude-from')) for x in server_args):
        raise ValueError('rsync 参数越界')
    dry = request['action'] == 'dry-run'
    is_dry = '--dry-run' in server_args or any(re.fullmatch(r'-[A-Za-z.]*n[A-Za-z.]*', x) for x in server_args)
    if dry != is_dry:
        raise ValueError('预览协议不匹配')
    with operation_lock(root):
        check_binding(root, request)
        if (root / 'source/.git').exists() or (root / 'source/.git').is_symlink():
            raise ValueError('拒绝包含 .git 的目标源码目录')
        inventory(root / 'source', policy)
        evidence = root / 'evidence' / request['run_id']
        evidence.mkdir(mode=0o700)
        receipt = {k: request[k] for k in ['protocol', 'project', 'branch', 'instance', 'checkout', 'head', 'dirty', 'kind', 'run_id', 'snapshot_id', 'manifest_sha256', 'bundle']}
        receipt.update(started=utc(), status='failed', test='not-run', command=request['command'], rsync=rsync_version(['rsync']))
        previous = {}
        def interrupted(signum, _frame):
            raise OperationInterrupted(signum)
        for signum in [signal.SIGTERM, signal.SIGINT, signal.SIGHUP]:
            previous[signum] = signal.signal(signum, interrupted)
        code = 2
        active = root / 'active-run.json'
        replace_json(active, {key: request[key] for key in ('run_id', 'checkout', 'bundle')} |
                     {'pid': os.getpid(), 'starttime': process_identity(os.getpid())})
        try:
            sync_start = time.monotonic()
            code = transfer(server_args) if transfer else managed_run(['rsync', *server_args], policy['sync_timeout_seconds'])
            receipt.update(sync_exit_code=code, sync_seconds=round(time.monotonic() - sync_start, 3))
            if code:
                return code
            if dry:
                receipt['status'] = 'dry-run'
                return 0
            verify_snapshot(root, request, policy)
            request_file = evidence / 'request.json'
            write_new(request_file, json_bytes(request))
            tools = Path(__file__).resolve().parent
            adapter = tools / Path(policy['adapter']).name
            environment = {k: v for k, v in os.environ.items() if k in ('PATH', 'HOME', 'LANG', 'TMPDIR')}
            environment.update(PYTHONDONTWRITEBYTECODE='1', QFY_SYNC_REQUEST=str(request_file),
                               QFY_SYNC_ROOT=str(root), QFY_SYNC_EVIDENCE=str(evidence),
                               QFY_SYNC_PROJECT_ROOT=policy['remote_root'])
            run_start = time.monotonic()
            receipt['test'] = 'running'
            with (evidence / 'test.log').open('xb') as log:
                code = managed_run([sys.executable, str(adapter), *request['command']], policy['timeout_seconds'],
                                   cwd=root / 'source', env=environment, stdout=log, stderr=log)
            receipt.update(test_exit_code=code, test='passed' if code == 0 else 'failed',
                           status='passed' if code == 0 else 'timeout' if code == 124 else 'failed',
                           test_seconds=round(time.monotonic() - run_start, 3))
            extra = evidence / 'runtime-result.json'
            if extra.exists():
                values = read_json(extra)
                receipt.update({key: values[key] for key in ('toolchain', 'cache_key', 'docker_context', 'network', 'business_evidence') if key in values})
            return code if code >= 0 else 128 - code
        except OperationInterrupted as error:
            receipt.update(status='interrupted', test='interrupted' if receipt['test'] == 'running' else receipt['test'],
                           test_exit_code=128 + int(error.args[0]))
            return receipt['test_exit_code']
        except Exception as error:
            receipt['error'] = str(error)
            raise
        finally:
            receipt['finished'] = utc()
            write_new(evidence / 'result.json', json_bytes(receipt))
            active.unlink()
            for signum, handler in previous.items():
                signal.signal(signum, handler)


def remote_main(request, args):
    policy = policy_checked(read_json(Path(__file__).with_name('policy.json')))
    if request['action'] in ('test', 'dry-run'):
        return receive(request, args, policy)
    root = target_root(request, policy)
    check_binding(root, request)
    if request['action'] == 'cancel':
        active = root / 'active-run.json'
        if active.exists():
            running = read_json(active)
            if any(running.get(k) != request[k] for k in ('run_id', 'checkout', 'bundle')):
                raise ValueError('活动运行身份不同，拒绝取消')
            try:
                if process_identity(running['pid']) != running['starttime']:
                    raise ValueError('活动进程身份已变化，拒绝取消')
                os.kill(running['pid'], signal.SIGTERM)
            except (ProcessLookupError, FileNotFoundError):
                pass
        return 0
    evidence = safe_dir(root / 'evidence' / request['run_id'])
    if request['action'] == 'result':
        print(json.dumps(read_json(evidence / 'result.json'), ensure_ascii=False))
    elif request['action'] == 'log':
        path = evidence / 'test.log'
        if path.exists():
            sys.stdout.write(read_file(path)[0].decode(errors='replace'))
    else:
        raise ValueError('不支持的远端操作')
    return 0


def branch_status(request, policy):
    """只读状态可在操作锁被占用时查询，不同步或改写正在验证的源码。"""
    root = target_root(request, policy)
    if not root.exists():
        return {'instance': request['instance'], 'state': 'unbound'}
    check_binding(root, request)
    active = root / 'active-run.json'
    try:
        running = read_json(active)
    except FileNotFoundError:
        running = None
    if running:
        if running['checkout'] != request['checkout']:
            raise ValueError('活动运行 checkout 不一致')
        output = {'instance': request['instance'], 'state': 'running', 'run_id': running['run_id']}
        current = root / 'evidence' / running['run_id'] / 'request.json'
        if current.exists():
            value = read_json(current)
            output.update({key: value[key] for key in ('command', 'head', 'snapshot_id', 'dirty')})
        else:
            output['phase'] = 'sync'
        return output
    results = list((root / 'evidence').glob('*/result.json'))
    return {'instance': request['instance'], 'state': 'idle',
            'last_result': read_json(max(results, key=lambda p: p.stat().st_mtime_ns)) if results else None}


def ssh(policy, program, **kwargs):
    return subprocess.run(['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=15', policy['host'], program],
                          timeout=kwargs.pop('timeout', 30), **kwargs)


def run_sync(root, policy, action, command, commit=None, overlay=None):
    started = time.monotonic()
    stage, request = snapshot(root, policy, commit, overlay=overlay)
    try:
        if commit:
            request['instance'] = policy['project'] + '-v-' + commit[:12] + '-' + request['run_id'][:8]
            manifest = read_json(stage / MANIFEST)
            manifest['instance'] = request['instance']
            raw = json_bytes(manifest)
            (stage / MANIFEST).write_bytes(raw)
            request['manifest_sha256'] = digest(raw)
        request.update(action=action, command=command)
        if not command or any(not isinstance(x, str) or not x or '\x00' in x or '\n' in x for x in command):
            raise ValueError('需要明确的命令参数数组')
        files = {name: base64.b64encode(content).decode() for name, content in {
            'remote_sync.py': Path(__file__).read_bytes(),
            Path(policy['adapter']).name: read_file(root / policy['adapter'])[0],
            'policy.json': json_bytes(policy),
        }.items()}
        request['bundle'] = digest(json_bytes(files))
        program = "__name__ = 'qfy_bootstrap'\n" + Path(__file__).read_text() + '\n'
        program += 'print(json.dumps(install(' + repr(request) + ',' + repr(policy) + ',' + repr(files) + ')))\n'
        installed = ssh(policy, 'python3 -', input=program, capture_output=True, text=True)
        if installed.returncode:
            detail = installed.stderr.strip().splitlines()
            raise RuntimeError('远端绑定/前置检查失败：' + (detail[-1] if detail else 'SSH 退出码 ' + str(installed.returncode)))
        helper = json.loads(installed.stdout)
        remote_root = target_root(request, policy)
        expected = remote_root / 'runtime' / ('tools-' + request['bundle']) / 'remote_sync.py'
        if helper != {'root': str(remote_root), 'bundle': request['bundle'], 'helper': str(expected)}:
            raise ValueError('远端接收器摘要或目录不符')
        local_rsync = rsync_version(['rsync'])
        receiver = shlex.join(['python3', str(expected), '--remote', encoded(request)])
        argv = ['rsync', '-rptO', '--checksum', '--delete', '--itemize-changes', '-e', 'ssh -o BatchMode=yes -o ConnectTimeout=15', '--rsync-path=' + receiver]
        # rsync 按首个匹配生效：已跟踪模板的锚定 include 必须在排除规则之前。
        argv += ['--include=/' + value for value in policy.get('include_tracked', [])]
        argv += ['--exclude=' + value for value in excludes(policy)]
        if action == 'dry-run':
            argv.append('--dry-run')
        argv += [str(stage) + '/', policy['host'] + ':' + str(remote_root / 'source') + '/']
        output = make_dir(root / 'tmp' / 'remote-dev' / request['run_id'])
        print('远端验证：' + request['instance'] + '，源码 ' + request['snapshot_id'][:12], flush=True)
        cancelled = None
        with subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True) as process:
            try:
                stdout, stderr = process.communicate(timeout=policy['sync_timeout_seconds'] + policy['timeout_seconds'] + 60)
            except (subprocess.TimeoutExpired, OperationInterrupted, KeyboardInterrupt) as error:
                cancelled = 124 if isinstance(error, subprocess.TimeoutExpired) else 128 + int(error.args[0]) if isinstance(error, OperationInterrupted) else 130
                cancel = shlex.join(['python3', str(expected), '--remote', encoded(request | {'action': 'cancel'})])
                try:
                    ssh(policy, cancel, capture_output=True, check=True)
                finally:
                    terminate(process)
                stdout, stderr = process.communicate()
        transferred = subprocess.CompletedProcess(argv, process.returncode, stdout, stderr)
        write_new(output / 'rsync.log', (transferred.stdout + transferred.stderr).encode())
        fetch = shlex.join(['python3', str(expected), '--remote', encoded(request | {'action': 'result'})])
        result = ssh(policy, fetch, capture_output=True, text=True)
        if cancelled:
            deadline = time.monotonic() + 15
            while result.returncode and time.monotonic() < deadline:
                time.sleep(.5)
                result = ssh(policy, fetch, capture_output=True, text=True)
        if result.returncode:
            raise RuntimeError('远端未生成运行结果；详见 ' + str(output / 'rsync.log'))
        receipt = json.loads(result.stdout)
        if any(receipt.get(k) != request[k] for k in ['project', 'checkout', 'instance', 'head', 'run_id', 'snapshot_id', 'bundle']):
            raise ValueError('回传结果来源不一致')
        receipt.update(local_rsync=local_rsync, elapsed_seconds=round(time.monotonic() - started, 3))
        write_new(output / 'result.json', json_bytes(receipt))
        if action != 'dry-run' and receipt.get('test') != 'not-run':
            fetch_log = shlex.join(['python3', str(expected), '--remote', encoded(request | {'action': 'log'})])
            log = ssh(policy, fetch_log, capture_output=True, check=True)
            write_new(output / 'test.log', log.stdout)
            sys.stdout.write(log.stdout.decode(errors='replace'))
        print(json.dumps(receipt, ensure_ascii=False))
        print('本机结果：' + str(output))
        # rsync 的远端进程状态和测试状态都必须成功。
        return cancelled or receipt.get('test_exit_code', receipt.get('sync_exit_code', transferred.returncode)) or transferred.returncode
    finally:
        shutil.rmtree(stage)


def run_cli(root, policy_path, argv, validate_command=None, overlay_for=None):
    os.umask(0o077)
    previous = {}
    def interrupted(signum, _frame):
        raise OperationInterrupted(signum)
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        previous[signum] = signal.signal(signum, interrupted)
    try:
        policy = policy_checked(read_json(policy_path))
        if argv == ['status']:
            request = identity(root, policy) | {'kind': 'development-snapshot', 'run_id': uuid.uuid4().hex}
            program = "__name__ = 'qfy_status'\n" + Path(__file__).read_text() + '\n'
            program += 'print(json.dumps(branch_status(' + repr(request) + ',' + repr(policy) + '),ensure_ascii=False))\n'
            result = ssh(policy, 'python3 -', input=program, capture_output=True, text=True)
            if result.returncode:
                raise RuntimeError('状态查询失败：' + result.stderr.strip().splitlines()[-1])
            print(result.stdout.strip())
            return 0
        if not argv or argv[0] not in ('test', 'dry-run', 'verify'):
            raise ValueError('用法：remote-dev.py status；test|dry-run <command> [args]；verify --sha <完整SHA> <command> [args]')
        action, command, commit = argv[0], argv[1:], None
        if action == 'verify':
            parser = argparse.ArgumentParser()
            parser.add_argument('--sha', required=True)
            parser.add_argument('command', nargs=argparse.REMAINDER)
            parsed = parser.parse_args(command)
            commit, command, action = parsed.sha, parsed.command, 'test'
        if validate_command:
            validate_command(command)
        return run_sync(root, policy, action, command, commit, overlay_for(command) if overlay_for else None)
    except (ValueError, RuntimeError, OSError, subprocess.SubprocessError) as error:
        print('远端开发失败：' + str(error), file=sys.stderr)
        return 2
    finally:
        for signum, handler in previous.items():
            signal.signal(signum, handler)


if __name__ == '__main__':
    os.umask(0o077)
    try:
        if len(sys.argv) < 3 or sys.argv[1] != '--remote':
            raise ValueError('请通过项目薄入口调用')
        raise SystemExit(remote_main(json.loads(base64.b64decode(sys.argv[2])), sys.argv[3:]))
    except (ValueError, RuntimeError, OSError, subprocess.SubprocessError) as error:
        print('远端接收失败：' + str(error), file=sys.stderr)
        raise SystemExit(2)
