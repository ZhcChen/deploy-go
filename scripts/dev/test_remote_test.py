"""远端构建与测试入口的本地回归，不连接远端（由 make dev-remote-check 运行）。"""
import json
from pathlib import Path
import re
import sys
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))

import deploy_go_remote_runtime as runtime  # noqa: E402
import remote_sync  # noqa: E402


class CommandWhitelistTest(unittest.TestCase):
    def test_registered_commands_are_accepted(self):
        for argv in (['rust-check'], ['rust-fmt'], ['rust-clippy'], ['rust-test'],
                     ['rust-test', '-p', 'deploy-go-api', '--test', 'platform'],
                     ['rust-test', '-E', 'test(=users_api::user_list_requires_administrator)'],
                     ['rust-test', '--lib', 'lease_window'], ['web-check'],
                     ['web-test', 'src/test/SseClient.test.ts']):
            with self.subTest(argv=argv):
                runtime.checked_command(argv)

    def test_arguments_that_could_change_tools_paths_or_shell_are_rejected(self):
        for argv in ([], ['shell'], ['rust-check', 'extra'], ['web-check', 'x'],
                     ['rust-test', '--config', 'x'], ['rust-test', '--target-dir', '/tmp/x'],
                     ['rust-test', '--manifest-path', 'x/Cargo.toml'], ['rust-test', '-Z', 'x'],
                     ['rust-test', '-p'], ['rust-test', '../etc'], ['rust-test', '/etc/passwd'],
                     ['rust-test', 'a;rm -rf /'], ['rust-test', '$(id)'], ['rust-test', '`id`'],
                     ['web-test', '/etc/passwd'], ['web-test', '../x.test.ts'], ['web-test', '--reporter=x']):
            with self.subTest(argv=argv):
                with self.assertRaises(ValueError):
                    runtime.checked_command(argv)


class ContractTest(unittest.TestCase):
    def setUp(self):
        self.policy = remote_sync.policy_checked(json.loads((HERE / 'remote-test.json').read_text()))

    def test_contract_maps_to_the_project_root_on_qfy_test2(self):
        self.assertEqual(self.policy['host'], 'qfy-test2')
        self.assertEqual(self.policy['remote_root'], '/opt/qfy-dev/deploy-go')
        self.assertEqual(self.policy['adapter'], 'scripts/dev/deploy_go_remote_runtime.py')

    def test_template_env_files_embedded_by_the_api_are_synced(self):
        # container-template 用 include_str! 嵌入 examples/templates/**；缺了这些文件远端无法编译。
        for path in self.policy['include_tracked']:
            with self.subTest(path=path):
                self.assertTrue((ROOT / path).is_file())
                self.assertFalse(remote_sync.excluded(path, self.policy))

    def test_local_data_symlinks_and_build_outputs_are_not_synced(self):
        for path in ('CLAUDE.md', '.data/deploy-logs/x.log', 'target/debug/x', 'admin/node_modules/x/index.js',
                     'admin/dist/index.html', 'tmp/remote-dev/x/result.json', '.git/config', '.env'):
            with self.subTest(path=path):
                self.assertTrue(remote_sync.excluded(path, self.policy))


class RustTreeTest(unittest.TestCase):
    """通用协议把同步文件的修改时间统一设为 1；Cargo 按修改时间判断重编，必须经构建目录纠正。"""

    def snapshot(self, root, files):
        import hashlib
        import os
        for name, data in files.items():
            path = root / 'source' / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            os.utime(path, (1, 1))
        manifest = {'files': {name: {'sha256': hashlib.sha256(data).hexdigest(), 'executable': False}
                              for name, data in files.items()}}
        (root / 'source' / '.qfy-source.json').write_text(json.dumps(manifest))

    def test_only_changed_files_are_rewritten_and_get_a_new_mtime(self):
        import tempfile
        import time
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            (root / 'source').mkdir()
            (root / 'runtime').mkdir()
            instance = runtime.Runtime.__new__(runtime.Runtime)
            instance.root, instance.source, instance.receipt = root, root / 'source', {}
            self.snapshot(root, {'crate/src/lib.rs': b'v1', 'keep.rs': b'same', 'gone.rs': b'x'})
            tree = instance.rust_tree()
            kept = (tree / 'keep.rs').stat().st_mtime
            self.assertGreater(kept, 1000, '首次写入后修改时间应是当前时间，而不是 1970 年')
            time.sleep(1.1)
            (root / 'source' / 'gone.rs').unlink()
            self.snapshot(root, {'crate/src/lib.rs': b'v2', 'keep.rs': b'same'})
            instance.rust_tree()
            self.assertEqual((tree / 'crate/src/lib.rs').read_bytes(), b'v2')
            self.assertGreater((tree / 'crate/src/lib.rs').stat().st_mtime, kept, '内容变化的文件必须变新，Cargo 才会重编')
            self.assertEqual((tree / 'keep.rs').stat().st_mtime, kept, '内容未变的文件保持原修改时间，避免全量重编')
            self.assertFalse((tree / 'gone.rs').exists(), '清单里已移除的文件必须删除')
            self.assertEqual(instance.receipt['rust_tree'], {'changed': 1, 'removed': 1, 'files': 2})

    def test_manifest_paths_cannot_escape_the_tree(self):
        import tempfile
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            (root / 'source').mkdir()
            (root / 'runtime').mkdir()
            (root / 'source' / '.qfy-source.json').write_text(json.dumps({'files': {'../escape': {'sha256': '0', 'executable': False}}}))
            instance = runtime.Runtime.__new__(runtime.Runtime)
            instance.root, instance.source, instance.receipt = root, root / 'source', {}
            with self.assertRaises(ValueError):
                instance.rust_tree()


class WebMountTest(unittest.TestCase):
    def test_every_typescript_project_reference_gets_a_writable_build_info(self):
        # 源码只读挂载；漏掉的子项目会让 tsc -b 报 EROFS。
        config = json.loads((ROOT / 'admin/tsconfig.json').read_text())
        references = {Path(item['path']).stem for item in config['references']}
        self.assertEqual(references, set(runtime.TS_PROJECTS))


class SummaryParsingTest(unittest.TestCase):
    def test_colored_tool_output_is_parsed_after_stripping_ansi(self):
        # 远端首次 web-check：208 个用例全部通过，却因颜色控制符匹配不到汇总行而被判失败。
        colored = '\x1b[2m      Tests \x1b[22m \x1b[1m\x1b[32m208 passed\x1b[39m\x1b[22m\x1b[90m (208)\x1b[39m'
        self.assertEqual(re.findall(r'Tests\s+(\d+) passed \((\d+)\)', runtime.ANSI.sub('', colored)), [('208', '208')])


class ToolImageTest(unittest.TestCase):
    def test_base_images_are_pinned_by_digest(self):
        for name in runtime.TOOLS.values():
            text = (ROOT / 'deploy/dev-tools' / name).read_text()
            with self.subTest(dockerfile=name):
                self.assertRegex(text, r'(?m)^FROM [^ ]+@sha256:[0-9a-f]{64}$')

    def test_rust_toolchain_matches_rust_toolchain_toml(self):
        # 镜像工具链与 rust-toolchain.toml 不一致时，每次运行都会联网下载工具链。
        channel = re.search(r'channel = "([^"]+)"', (ROOT / 'rust-toolchain.toml').read_text()).group(1)
        text = (ROOT / 'deploy/dev-tools/Dockerfile.rust').read_text()
        self.assertIn('FROM rust:' + channel + '-', text)
        self.assertIn('--toolchain ' + channel + ' clippy rustfmt', text)

    def test_resource_limits_leave_room_for_production_on_the_shared_host(self):
        self.assertLessEqual(runtime.RUST_CPUS, 8)
        self.assertLessEqual(runtime.NODE_CPUS, 6)
        self.assertLess(int(runtime.CPU_SHARES), 1024)


if __name__ == '__main__':
    unittest.main()
