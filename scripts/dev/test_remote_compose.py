"""开发入口的隔离回归，不连接远端。"""
import copy
import importlib.util
import json
import hashlib
import io
import tarfile
import tempfile
import os
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

PATH = Path(__file__).with_name('remote-compose.py')
spec = importlib.util.spec_from_file_location('remote_compose', PATH)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class IsolationTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.record = dict(project='deploy-go-main-environment-dev-12345678',
                          owner='00000000-0000-0000-0000-000012345678', branch='main',
                          unit='environment', purpose='dev', sha='a' * 40,
                          image='deploy-go-development:' + 'a' * 40)
        values = {**cls.record, 'origin': 'http://127.0.0.1:30880'}
        env = {k: v for k, v in os.environ.items() if not k.startswith(('DEV_', 'COMPOSE_'))}
        env.update({'DEV_' + k.upper(): v for k, v in values.items()})
        result = subprocess.run(['docker', '--context', 'orbstack', 'compose', '-p', cls.record['project'], '-f',
                                 str(module.ROOT / 'deploy/development/compose.json'),
                                 '--env-file', '/dev/null', 'config', '--format', 'json'],
                                check=True, text=True, stdout=subprocess.PIPE, env=env)
        cls.config = json.loads(result.stdout)

    def test_rendered_stack_is_isolated(self):
        module.validate(self.config, self.record)
        for service in self.config['services'].values():
            self.assertFalse(service.get('ports'))
        self.assertTrue(self.config['networks']['isolated']['internal'])

    def test_rejects_dangerous_resource_changes(self):
        for name, value in [('ports', [{'target': 30100, 'published': '30100'}]),
                            ('container_name', 'deploy-go-api'), ('network_mode', 'host'),
                            ('privileged', True), ('use_api_socket', True), ('image', 'production:latest'),
                            ('volumes', [{'type': 'bind', 'source': '/var/lib/deploy-go', 'target': '/data'}])]:
            with self.subTest(name=name):
                config = copy.deepcopy(self.config)
                config['services']['api'][name] = value
                with self.assertRaises(ValueError):
                    module.validate(config, self.record)
        for kind, key in [('networks', 'isolated'), ('volumes', 'data')]:
            config = copy.deepcopy(self.config)
            config[kind][key]['name'] = 'production'
            with self.assertRaises(ValueError):
                module.validate(config, self.record)

    def test_transfer_uses_archive_config_digest(self):
        config = b'{"architecture":"amd64","os":"linux"}'
        with tempfile.TemporaryDirectory() as temp:
            archive = Path(temp) / 'image.tar'
            with tarfile.open(archive, 'w') as bundle:
                for name, content in [('manifest.json', b'[{"Config":"blobs/config.json"}]'), ('blobs/config.json', config)]:
                    entry = tarfile.TarInfo(name)
                    entry.size = len(content)
                    bundle.addfile(entry, io.BytesIO(content))
            self.assertEqual(module.image_config_id(archive), 'sha256:' + hashlib.sha256(config).hexdigest())

    def test_named_volume_cannot_hide_host_bind(self):
        config = copy.deepcopy(self.config)
        config['volumes']['data']['driver_opts'] = dict(type='none', o='bind', device='/var/lib/deploy-go')
        with self.assertRaises(ValueError):
            module.validate(config, self.record)

    def test_existing_unmanaged_resource_is_rejected(self):
        for kind, suffix in [('volume','_data'), ('network','_isolated'), ('container','-api-1')]:
            def fake(*args, **kwargs):
                if 'inspect' in args:
                    return '{}'
                if '--format' in args and args[0] == {'volume':'volume','network':'network','container':'ps'}[kind]:
                    return self.record['project'] + suffix
                return ''
            with self.subTest(kind=kind), patch.object(module, 'docker', side_effect=fake):
                with self.assertRaises(ValueError):
                    module.owned_resources(self.record)

    def test_cannot_adopt_another_owner(self):
        record = self.record | {'owner': '11111111-1111-1111-1111-111111111111'}
        with self.assertRaises(ValueError):
            module.validate(self.config, record)

    def test_build_never_uses_remote_default_context(self):
        with patch.object(module, 'run', return_value='unix:///local/docker.sock'):
            self.assertEqual(module.local_args('build'), ['docker', '--context', 'orbstack', 'build'])
        with patch.object(module, 'run', return_value='ssh://qfy-test2'):
            with self.assertRaises(ValueError):
                module.local_args('build')

    def test_identity_uses_real_branch_and_full_owner(self):
        self.assertEqual(module.identity(self.record['owner'], 'Feature/Env', 'environment', 'dev'),
                         'deploy-go-feature-env-environment-dev-12345678')
        for owner, unit, purpose in [('short', 'environment', 'dev'),
                                     (self.record['owner'], '../production', 'dev'),
                                     (self.record['owner'], 'environment', 'production')]:
            with self.assertRaises(ValueError):
                module.identity(owner, 'main', unit, purpose)


if __name__ == '__main__':
    unittest.main()
