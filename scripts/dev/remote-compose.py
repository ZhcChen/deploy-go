#!/usr/bin/env python3
"""Deploy Go 独立远端开发入口；没有正式服务、全局清理或删卷操作。"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import socket
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[2]
CONTEXT = 'qfy-test2'
LOCAL_CONTEXT = 'orbstack'
REGISTRY = Path.home() / 'ai-space/server/dev-environments/REGISTRY.md'
LABELS = ('project', 'branch', 'unit', 'purpose', 'owner-session', 'sha')


def run(args, *, capture=False, cwd=ROOT, env=None):
    return subprocess.run(args, cwd=cwd, check=True, text=True,
                          stdout=subprocess.PIPE if capture else None, env=env).stdout


def docker(*args, capture=False):
    endpoint = run(['docker', 'context', 'inspect', CONTEXT, '--format', '{{.Endpoints.docker.Host}}'], capture=True).strip()
    if endpoint != 'ssh://qfy-test2':
        raise ValueError('qfy-test2 context endpoint 不匹配')
    return run(['docker', '--context', CONTEXT, *args], capture=capture)


def local_args(*args):
    endpoint = run(['docker', 'context', 'inspect', LOCAL_CONTEXT, '--format', '{{.Endpoints.docker.Host}}'], capture=True).strip()
    if not endpoint.startswith('unix://'):
        raise ValueError('本机构建 context 必须指向本机 Unix Socket，禁止落到正式主机')
    return ['docker', '--context', LOCAL_CONTEXT, *args]


def identity(owner, branch, unit, purpose):
    if not re.fullmatch(r'[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}', owner):
        raise ValueError('必须提供完整 owner session UUID')
    if not re.fullmatch(r'[a-z][a-z0-9-]{0,20}', unit) or purpose not in ('dev', 'int', 'accept'):
        raise ValueError('unit/purpose 无效')
    slug = re.sub('[^a-z0-9]+', '-', branch.lower()).strip('-')[:28].rstrip('-')
    if not slug:
        raise ValueError('分支标识无效')
    return f'deploy-go-{slug}-{unit}-{purpose}-{owner[-8:]}'


def validate(config, expected):
    """渲染后继续限定资源，禁止将开发入口变成正式环境操作器。"""
    if config.get('name') != expected['project']:
        raise ValueError('Compose project 不匹配')
    if set(config['services']) != {'initialize', 'api', 'web'}:
        raise ValueError('服务集合不匹配')
    wanted = {f'dev.{k}': expected['owner'] if k == 'owner-session' else expected[k] for k in LABELS}
    allowed = {'image','labels','restart','read_only','security_opt','cap_drop','pids_limit','tmpfs','logging','user','command','depends_on','volumes','networks','network_mode','cpus','mem_limit','environment','healthcheck','pull_policy','entrypoint','cap_add'}
    commands = {'initialize':['python3','/app/initialize.py'], 'api':['/usr/local/bin/deploy-go-api'], 'web':['python3','/app/web_server.py','--root','/app/web','--api','http://api:30100','--bind','0.0.0.0','--port','8080']}
    for name, service in config['services'].items():
        if set(service) - allowed or service.get('entrypoint') is not None or service.get('command') != commands[name]:
            raise ValueError('服务字段/命令越出开发合同')
        if name == 'initialize' and (service.get('user') != '0:0' or set(service.get('cap_add', [])) != {'CHOWN','FOWNER','DAC_OVERRIDE'}):
            raise ValueError('初始化容器权限不匹配')
        if service.get('security_opt') != ['no-new-privileges:true']:
            raise ValueError('禁止额外权限')
        if name != 'initialize' and (set(service.get('networks', {})) != {'isolated'} or service['networks']['isolated']):
            raise ValueError('服务网络配置越界')
        if service.get('image') != expected['image'] or service.get('labels') != wanted:
            raise ValueError('服务镜像或归属不匹配')
        if any(service.get(k) for k in ('ports', 'container_name', 'privileged', 'devices', 'pid', 'ipc', 'extends', 'volumes_from')):
            raise ValueError('禁止端口发布、固定容器名和宿主权限')
        if service.get('network_mode') not in (None, 'none'):
            raise ValueError('禁止共享或宿主网络')
        if not service.get('read_only') or service.get('cap_drop') != ['ALL']:
            raise ValueError('必须使用只读根和最小权限')
        if name != 'initialize' and (service.get('user') != '1000:1000' or service.get('cap_add')):
            raise ValueError('运行服务必须降权')
        expected_mounts = {'initialize': {'data':('/data',False), 'keys':('/keys',False)}, 'api': {'data':('/data',False), 'keys':('/keys',True)}, 'web': {}}[name]
        if {m['source'] for m in service.get('volumes', [])} != set(expected_mounts):
            raise ValueError('挂载集合不匹配')
        for mount in service.get('volumes', []):
            if mount['type'] != 'volume' or mount['source'] not in ('data', 'keys'):
                raise ValueError('禁止宿主挂载或外部卷')
            if (mount['target'], bool(mount.get('read_only'))) != expected_mounts[mount['source']]:
                raise ValueError('挂载目标/读写模式越界')
        if not service.get('cpus') or not service.get('mem_limit') or not service.get('pids_limit'):
            raise ValueError('缺少资源限额')
    for kind, names in [('volumes', ('data', 'keys')), ('networks', ('isolated',))]:
        if set(config[kind]) != set(names):
            raise ValueError('资源集合不匹配')
        for key, resource in config[kind].items():
            fields = {'name','labels','driver'} if kind == 'volumes' else {'name','labels','internal','ipam','driver'}
            if set(resource) - fields or resource.get('driver') not in (None, 'local' if kind == 'volumes' else 'bridge') or resource.get('ipam'):
                raise ValueError('禁止卷驱动挂载宿主或自定义共享网络')
            if resource.get('external') or resource.get('name') != expected['project'] + '_' + key or resource.get('labels') != wanted:
                raise ValueError('资源名称或归属不匹配')
    if not config['networks']['isolated'].get('internal'):
        raise ValueError('必须使用独立内部网络')


def compose(record, directory, *args, capture=False):
    # --env-file 是唯一 DEV_* 来源，避免当前 shell 覆盖已登记配置。
    env = {k: v for k, v in os.environ.items() if not k.startswith(('DEV_', 'COMPOSE_'))}
    return run(['docker', '--context', CONTEXT, 'compose', '-p', record['project'],
                '-f', str(directory / 'compose.json'), '--env-file', str(directory / 'config.env'), *args], capture=capture, env=env)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def owned_resources(record):
    wanted = {f'dev.{k}': record['owner'] if k == 'owner-session' else record[k] for k in LABELS}
    for kind, listing in [('container', ['ps', '-aq']), ('network', ['network', 'ls', '-q']), ('volume', ['volume', 'ls', '-q'])]:
        ids = set(docker(*listing, '--filter', 'label=com.docker.compose.project=' + record['project'], capture=True).split())
        # 未打 Compose 标签的同名卷/网络也可能被 Compose 复用，必须按精确名字检查。
        names = {'container': {record['project'] + '-' + name + '-1' for name in ('initialize','api','web')},
                 'volume': {record['project'] + '_' + name for name in ('data','keys')},
                 'network': {record['project'] + '_isolated'}}[kind]
        all_names = docker(*{'container':['ps','-a'], 'volume':['volume','ls'], 'network':['network','ls']}[kind],
                           '--format', '{{.Names}}' if kind == 'container' else '{{.Name}}', capture=True).split()
        ids.update(names.intersection(all_names))
        for resource in ids:
            labels = json.loads(docker(kind, 'inspect', resource, '--format', '{{json .Config.Labels}}' if kind == 'container' else '{{json .Labels}}', capture=True)) or {}
            if any(labels.get(k) != v for k, v in wanted.items()):
                raise ValueError('远端资源归属/版本不匹配，禁止自动接管')


def check(record, directory, require_image=True):
    if record['context'] != CONTEXT or record['branch'] != run(['git', 'branch', '--show-current'], capture=True).strip():
        raise ValueError('context/分支不匹配')
    if record['project'] != identity(record['owner'], record['branch'], record['unit'], record['purpose']):
        raise ValueError('project 身份不匹配')
    for name in ('compose.json', 'config.env'):
        if digest(directory / name) != record['files'][name]:
            raise ValueError('已登记配置发生变更，请核实归属和版本')
    validate(json.loads(compose(record, directory, 'config', '--format', 'json', capture=True)), record)
    if require_image:
        image_id = docker('image', 'inspect', record['image'], '--format', '{{.Id}}', capture=True).strip()
        if image_id != record['image_id']:
            raise ValueError('远端镜像与登记 ID 不匹配')
    owned_resources(record)


def reserve(record, directory, local_port):
    """在统一事实源串行预留，Obsidian 不复制端口分配。"""
    if not REGISTRY.is_file():
        raise ValueError('统一登记表缺失，禁止创建远端资源')
    with socket.socket() as probe:
        probe.bind(('127.0.0.1', local_port))
    with REGISTRY.with_suffix('.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if record['project'] in REGISTRY.read_text():
            raise ValueError('project 已被登记，核实上次现场；不自动接管或覆盖')
        body = f"""

## {record['project']}

- owner `{record['owner']}`；checkout `{ROOT}`；分支 `{record['branch']}`；unit `{record['unit']}`；purpose `{record['purpose']}`；代码 SHA `{record['sha']}`。
- context qfy-test2；配置 `{directory / 'compose.json'}`、`{directory / 'config.env'}`，均不含凭据；镜像 `{record['image']}` / `{record['image_id']}`。
- 零宿主发布端口；本机 SSH 隧道候选 `127.0.0.1:{local_port}`（绑定前再次核验）。网络/卷由 Compose project 自动生成；不挂载正式目录或 Docker socket。
- SQLite/制品/日志在独立 data 卷，独立合成密钥在 keys 卷；常驻上限 1.25CPU / 640MiB，初始化另 0.25CPU / 64MiB。停止保留卷；删除另需具体授权。
- 授权：2026-10-27 用户要求按 Obsidian 模板调整开发环境并注意同机正式环境冲突；唯一写入和清理责任人为本 owner；到期复核 2026-11-03。
- 状态：已预留 / 未启动；镜像传输和启动失败时保留本记录核查，不虚报清理。实际运行/验证另追加证据。
"""
        with REGISTRY.open('a') as target:
            target.write(body)


def build(args, record, directory):
    if run(['git', 'status', '--porcelain'], capture=True).strip():
        raise ValueError('构建要求干净 Git 快照，请先提交')
    if (directory / 'record.json').exists():
        raise ValueError('本 project 已有构建记录；更新需新的 unit/project，不覆盖旧镜像')
    record['sha'] = run(['git', 'rev-parse', 'HEAD'], capture=True).strip()
    record['image'] = 'deploy-go-development:' + record['sha'] + '-' + record['project']
    binaries = 'deploy-go-development-binaries:' + record['sha'] + '-' + record['project']
    for tag in (record['image'],):
        found = subprocess.run(local_args('image', 'inspect', tag), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if found.returncode == 0:
            raise ValueError('SHA 镜像 tag 已存在，不覆盖；先核对上次构建现场')
    with tempfile.TemporaryDirectory(prefix='deploy-go-development-') as temp:
        stage = Path(temp)
        archive = stage / 'source.tar'
        run(['git', 'archive', '--format=tar', '-o', str(archive), record['sha']])
        source = stage / 'source'
        source.mkdir()
        with tarfile.open(archive) as bundle:
            bundle.extractall(source, filter='data')
        found = subprocess.run(local_args('image', 'inspect', binaries), text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        if found.returncode == 0:
            labels = json.loads(found.stdout)[0]['Config'].get('Labels') or {}
            if labels.get('org.opencontainers.image.revision') != record['sha'] or labels.get('dev.owner-session') != record['owner']:
                raise ValueError('已有构建中间镜像归属不匹配')
        else:
            run(local_args('build', '--platform', 'linux/amd64', '--build-arg', 'BUILD_API=1', '--build-arg', 'BUILD_AGENT=1', '--build-arg', 'BUILD_DEPLOYER=1', '--label', 'org.opencontainers.image.revision=' + record['sha'], '--label', 'dev.owner-session=' + record['owner'], '-t', binaries, '-f', 'deploy/docker/release/Dockerfile', '.'), cwd=source)
        run(['npm', 'ci', '--prefix', 'admin'], cwd=source)
        run(['npm', 'run', 'build', '--prefix', 'admin'], cwd=source)
        run(['node', 'scripts/check-client-sensitive-data.mjs', 'admin/dist'], cwd=source)
        runtime = stage / 'runtime'
        shutil.copytree(source / 'deploy/development', runtime)
        shutil.copytree(source / 'admin/dist', runtime / 'web')
        shutil.copy(source / 'deploy/production/web_server.py', runtime / 'web_server.py')
        run(local_args('build', '--platform', 'linux/amd64', '--build-arg', 'BINARIES_IMAGE=' + binaries,
             '--label', 'org.opencontainers.image.revision=' + record['sha'], '-t', record['image'], '.'), cwd=runtime)
    record['image_id'] = run(local_args('image', 'inspect', record['image'], '--format', '{{.Id}}'), capture=True).strip()
    if record['image'] in docker('image', 'ls', '--format', '{{.Repository}}:{{.Tag}}', capture=True).split():
        raise ValueError('远端 SHA tag 已存在，禁止覆盖')
    reserve(record, directory, args.local_port)
    owned_resources(record)
    # 传输镜像，不要求正式主机访问外部 Registry，不共享宿主源码。
    producer = subprocess.Popen(local_args('image', 'save', record['image']), stdout=subprocess.PIPE)
    try:
        subprocess.run(['docker', '--context', CONTEXT, 'image', 'load'], stdin=producer.stdout, check=True)
    finally:
        producer.stdout.close()
        if producer.wait() != 0:
            raise ValueError('镜像传输失败')
    directory.mkdir(parents=True, exist_ok=True)
    shutil.copy(ROOT / 'deploy/development/compose.json', directory / 'compose.json')
    values = {**record, 'origin': f'http://127.0.0.1:{args.local_port}'}
    (directory / 'config.env').write_text(''.join(f'DEV_{k.upper()}={values[k]}\n' for k in ('project','owner','branch','unit','purpose','sha','image','origin')))
    record['local_port'] = args.local_port
    record['files'] = {name: digest(directory / name) for name in ('compose.json', 'config.env')}
    (directory / 'record.json').write_text(json.dumps(record, indent=2) + '\n')
    check(record, directory)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('build', 'up', 'status', 'stop', 'tunnel'))
    parser.add_argument('--owner', default=os.getenv('CODEX_THREAD_ID', ''))
    parser.add_argument('--unit', default='environment')
    parser.add_argument('--purpose', default='dev', choices=('dev','int','accept'))
    parser.add_argument('--local-port', type=int, default=30880)
    args = parser.parse_args()
    if not 1024 <= args.local_port <= 65535 or args.local_port in (30100, 30101):
        raise ValueError('本机隧道端口必须非特权且避开正式端口')
    branch = run(['git','branch','--show-current'], capture=True).strip()
    project = identity(args.owner, branch, args.unit, args.purpose)
    directory = ROOT / '.data/remote-development' / project
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / 'operation.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if args.action == 'build':
            build(args, dict(project=project, owner=args.owner, branch=branch, unit=args.unit, purpose=args.purpose, context=CONTEXT), directory)
            return
        record = json.loads((directory / 'record.json').read_text())
        if record['owner'] != args.owner:
            raise ValueError('禁止接管其他 owner')
        check(record, directory, require_image=args.action == 'up')
        if args.action == 'up':
            registry = REGISTRY.read_text()
            if project not in registry or args.owner not in registry:
                raise ValueError('先在统一登记表预留 project、owner 与资源预算')
            compose(record, directory, 'up', '-d', '--wait', '--wait-timeout', '120')
        elif args.action == 'stop':
            compose(record, directory, 'stop', '--timeout', '20')
        elif args.action == 'status':
            compose(record, directory, 'ps', '-a')
        else:
            container = compose(record, directory, 'ps', '-q', 'web', capture=True).strip()
            if not container:
                raise ValueError('开发 Web 未运行')
            ip = docker('container', 'inspect', container, '--format', '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}', capture=True).strip()
            if not re.fullmatch(r'\d{1,3}(?:\.\d{1,3}){3}', ip):
                raise ValueError('开发 Web 地址无效')
            print(f'ssh -N -o ExitOnForwardFailure=yes -L 127.0.0.1:{record["local_port"]}:{ip}:8080 qfy-test2')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, FileNotFoundError, subprocess.CalledProcessError, BlockingIOError) as error:
        raise SystemExit(str(error)) from None
