#!/usr/bin/env python3
"""校验仓库 Markdown 文件中的相对链接和文档入口。"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


MARKDOWN_LINK = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")
REQUIRED_FILES = (
    "README.md",
    "AGENTS.md",
    "docs/README.md",
    "docs/standards/README.md",
    "docs/runbooks/README.md",
    "docs/plans/README.md",
    "specs/README.md",
    "docs/standards/document-authority.md",
    "docs/runbooks/development-guide.md",
    "docs/runbooks/spec-kit-workflow.md",
)


def repository_files(root: Path) -> list[Path]:
    result = subprocess.run(
        [
            "git",
            "-C",
            str(root),
            "ls-files",
            "-co",
            "--exclude-standard",
            "--",
            "*.md",
            "*.markdown",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return [root / line for line in result.stdout.splitlines() if line]


def link_path(raw_target: str) -> str:
    target = raw_target.strip()
    if target.startswith("<") and ">" in target:
        target = target[1 : target.index(">")]
    else:
        target = target.split(maxsplit=1)[0]
    return target.split("#", maxsplit=1)[0]


def check(root: Path) -> list[str]:
    errors: list[str] = []
    for required in REQUIRED_FILES:
        if not (root / required).is_file():
            errors.append(f"缺少文档入口：{required}")

    for source in repository_files(root):
        try:
            content = source.read_text(encoding="utf-8")
        except UnicodeDecodeError as error:
            errors.append(f"{source.relative_to(root)}：不是 UTF-8 文本（{error}）")
            continue
        for match in MARKDOWN_LINK.finditer(content):
            raw_target = match.group(1).strip()
            target = link_path(raw_target)
            if not target or target.startswith(("#", "http://", "https://", "mailto:")):
                continue
            candidate = (source.parent / target).resolve()
            try:
                candidate.relative_to(root)
            except ValueError:
                errors.append(f"{source.relative_to(root)}：链接越界：{raw_target}")
                continue
            if not candidate.exists():
                errors.append(f"{source.relative_to(root)}：链接目标不存在：{raw_target}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description="检查仓库 Markdown 文档入口和相对链接")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    root = args.root.resolve()
    errors = check(root)
    if errors:
        print(f"文档检查失败：{len(errors)} 个问题", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    count = len(repository_files(root))
    print(f"文档检查通过：已检查 {count} 个 Markdown 文件，入口和相对链接均有效")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
