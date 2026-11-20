#!/usr/bin/env python3
"""deploy-go 远端构建与测试薄入口：本机编辑，qfy-test2 构建测试。

通用传输协议（快照、增量同步、操作锁、证据）在 remote_sync 中维护，原样复制自 qfy-finance，不在此修改；
项目差异只在 remote-test.json（合同）与 deploy_go_remote_runtime.py（运行适配）。用法见
docs/runbooks/remote-development.md 的“远端构建与测试”。
"""
from pathlib import Path
import os
import shlex
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from remote_sync import run_cli  # noqa: E402
from deploy_go_remote_runtime import checked_command  # noqa: E402

if __name__ == '__main__':
    if len(sys.argv) == 3 and sys.argv[1] == '--make':
        target = sys.argv[2]
        command = shlex.split(os.environ.get('DEPLOY_GO_REMOTE_COMMAND', ''))
        if target in ('remote-test', 'remote-dry-run'):
            sys.argv[1:] = ['dry-run' if target == 'remote-dry-run' else 'test', *command]
        elif target == 'remote-verify':
            sys.argv[1:] = ['verify', '--sha', os.environ.get('DEPLOY_GO_REMOTE_SHA', ''), *command]
        elif target == 'remote-status':
            sys.argv[1:] = ['status']
        else:
            raise SystemExit('未知入口：' + target)
    root = Path(__file__).resolve().parents[2]
    raise SystemExit(run_cli(root, root / 'scripts/dev/remote-test.json', sys.argv[1:], validate_command=checked_command))
