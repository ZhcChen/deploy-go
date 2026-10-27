"""只初始化本 Compose project 的空卷，不读取宿主配置或输出密钥。"""
import base64
import os
import secrets
import stat
from pathlib import Path

os.chown('/data', 1000, 1000)
os.chmod('/data', 0o750)
os.chown('/keys', 0, 1000)
os.chmod('/keys', 0o750)
for name, uid, mode in [('master', 1000, 0o600), ('terminal', 0, 0o440), ('release', 0, 0o440)]:
    path = Path('/keys') / name
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
    except FileExistsError:
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != uid or stat.S_IMODE(info.st_mode) != mode:
            raise SystemExit('development_key_metadata_invalid')
        with path.open('rb') as source:
            if len(base64.b64decode(source.read(), validate=True)) != 32:
                raise SystemExit('development_key_length_invalid')
    else:
        with os.fdopen(fd, 'wb') as target:
            target.write(base64.b64encode(secrets.token_bytes(32)))
            target.flush()
            os.fsync(target.fileno())
            os.fchown(target.fileno(), uid, 1000)
            os.fchmod(target.fileno(), mode)
print('development_volumes_ready')
