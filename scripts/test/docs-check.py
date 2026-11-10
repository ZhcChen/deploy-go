#!/usr/bin/env python3
"""校验仓库 Markdown 文件中的相对链接和文档入口。"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from urllib.parse import unquote


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


# 当前有效文档：做 make 目标、仓库路径、锚点检查。plans/reviews/handoffs 等历史记录保持原样，不在此列。
CURRENT_DOC_PREFIXES = (
    "README.md",
    "AGENTS.md",
    "docs/README.md",
    "docs/runbooks/",
    "docs/standards/",
    "docs/solutions/",
)
# 业务应用自己的 Makefile 目标，文档里作为示例出现，不属于本仓库。
EXTERNAL_MAKE_TARGETS = {"deploy-go-prepare", "deploy-go-release"}
# 仓库内的路径前缀；只检查这些前缀开头的反引号路径。
REPO_PATH_ROOTS = (
    "api", "agent", "agent-protocol", "agent-executor", "agent-updater", "admin", "admin-app",
    "deploy", "scripts", "docs", "specs", "examples", "deploy-go-deployer", "container-template", "skills",
)
# 允许不存在的路径：Obsidian 存储的 specs/001–009、构建与运行产物。
# 契约类文档以被部署业务应用的目录结构作示例，这些路径不在本仓库；按文件显式列出，避免放宽到全局。
BUSINESS_EXAMPLE_PATHS = {
    "docs/standards/agent-control-protocol.md": {"scripts/release.sh"},
    "docs/standards/application-deployment-contract.md": {"api/qfy-voucher-hub-api.tar.gz"},
    "docs/standards/git-branch-deployment-contract.md": {"docs/api"},
}
ALLOWED_MISSING_PATHS = re.compile(
    r"^(specs/00[1-9]-[^/]*(/.*)?|\.data/.*|admin-app/build/.*|admin/dist/.*|admin/node_modules/.*|target/.*)$"
)
PLAN_STATUS_CUTOFF = "2026-11-05"

CODE_FENCE = re.compile(r"```[^\n]*\n(.*?)```", re.S)
INLINE_CODE = re.compile(r"(?<!`)`([^`\n]+)`(?!`)")
MAKE_INVOCATION = re.compile(r"(?:^|[\s;&|(])make\s+((?:-\S+\s+)*)([A-Za-z0-9][A-Za-z0-9_.-]*)")
PATH_IN_CODE = re.compile(r"(?<![\w./-])((?:%s)/[A-Za-z0-9_./-]*[A-Za-z0-9_/-])" % "|".join(re.escape(r) for r in sorted(REPO_PATH_ROOTS, key=len, reverse=True)))
HEADING = re.compile(r"^(#{1,6})\s+(.+?)\s*#*\s*$", re.M)


def is_current_doc(relative: str) -> bool:
    return any(relative == prefix or (prefix.endswith("/") and relative.startswith(prefix)) for prefix in CURRENT_DOC_PREFIXES)


def code_snippets(content: str) -> list[str]:
    fenced = CODE_FENCE.findall(content)
    inline = INLINE_CODE.findall(CODE_FENCE.sub("", content))
    return fenced + inline


def integration_test_targets(root: Path) -> set[str]:
    """工作区里真实存在的集成测试目标：<crate>/tests/<名>.rs 或 <crate>/tests/<名>/main.rs。"""
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-co", "--exclude-standard", "--", "*/tests/*.rs"],
        check=True,
        capture_output=True,
        text=True,
    )
    targets: set[str] = set()
    for line in result.stdout.splitlines():
        parts = line.split("/")
        if len(parts) == 3 and parts[1] == "tests":
            targets.add(parts[2][:-3])
        elif len(parts) == 4 and parts[1] == "tests" and parts[3] == "main.rs":
            targets.add(parts[2])
    return targets


CARGO_TEST_FLAG = re.compile(r"--test\s+([A-Za-z0-9_]+)")


def makefile_targets(root: Path) -> set[str]:
    makefile = root / "Makefile"
    if not makefile.is_file():
        return set()
    return set(re.findall(r"^([A-Za-z0-9][A-Za-z0-9_.-]*):(?!=)", makefile.read_text(encoding="utf-8"), re.M))


def heading_slugs(path: Path, cache: dict[Path, set[str]]) -> set[str]:
    if path in cache:
        return cache[path]
    slugs: set[str] = set()
    counts: dict[str, int] = {}
    try:
        text = CODE_FENCE.sub("", path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError):
        cache[path] = slugs
        return slugs
    for match in HEADING.finditer(text):
        title = re.sub(r"[`*_\[\]]", "", re.sub(r"\]\([^)]*\)", "]", match.group(2)))
        slug = re.sub(r"[^\w\- ]", "", title.strip().lower()).replace(" ", "-")
        seen = counts.get(slug, 0)
        counts[slug] = seen + 1
        slugs.add(slug if seen == 0 else f"{slug}-{seen}")
    cache[path] = slugs
    return slugs


def is_obsolete_doc(content: str) -> bool:
    """文件头 front matter 标注 status: obsolete/superseded 的文档保留为历史说明，不检查其中的命令和路径。"""
    match = re.match(r"^---\n(.*?)\n---\n", content, re.S)
    return bool(match and re.search(r"^status:\s*(obsolete|superseded)\s*$", match.group(1), re.M))


def plan_has_status(content: str) -> bool:
    head = content.splitlines()[:20]
    return any(re.match(r"^(status|状态)\s*[:：]", line) for line in head)


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
    targets = makefile_targets(root)
    test_targets = integration_test_targets(root)
    makefile = root / "Makefile"
    if makefile.is_file() and test_targets:
        for number, line in enumerate(makefile.read_text(encoding="utf-8").splitlines(), start=1):
            for name in CARGO_TEST_FLAG.findall(line) if "cargo" in line else []:
                if name not in test_targets:
                    errors.append(f"Makefile:{number}：引用了不存在的集成测试目标：--test {name}")
    slug_cache: dict[Path, set[str]] = {}
    for required in REQUIRED_FILES:
        if not (root / required).is_file():
            errors.append(f"缺少文档入口：{required}")

    for source in repository_files(root):
        try:
            content = source.read_text(encoding="utf-8")
        except UnicodeDecodeError as error:
            errors.append(f"{source.relative_to(root)}：不是 UTF-8 文本（{error}）")
            continue
        relative = str(source.relative_to(root))
        if is_current_doc(relative) and not is_obsolete_doc(content):
            for snippet in code_snippets(content):
                for flags, target_name in MAKE_INVOCATION.findall(snippet):
                    if targets and target_name not in targets and target_name not in EXTERNAL_MAKE_TARGETS:
                        errors.append(f"{relative}：引用了 Makefile 中不存在的目标：make {target_name}")
                for name in CARGO_TEST_FLAG.findall(snippet) if "cargo" in snippet else []:
                    if test_targets and name not in test_targets:
                        errors.append(f"{relative}：引用了不存在的集成测试目标：--test {name}")
                for path_match in PATH_IN_CODE.finditer(snippet):
                    path_text = path_match.group(1).rstrip("/.")
                    following = snippet[path_match.end():path_match.end() + 1]
                    # 后面紧跟通配符/占位符的是模式而不是具体路径；含 NNN 的是编号占位。
                    if following in ("*", "<", "{", "$", "[") or "NNN" in path_text or not path_text:
                        continue
                    if ALLOWED_MISSING_PATHS.match(path_text) or path_text in BUSINESS_EXAMPLE_PATHS.get(relative, ()):
                        continue
                    if not (root / path_text).exists():
                        errors.append(f"{relative}：引用了不存在的仓库路径：{path_text}")
        plan_name = Path(relative).name
        if relative.startswith("docs/plans/") and re.match(r"^\d{4}-\d{2}-\d{2}-", plan_name) and plan_name[:10] >= PLAN_STATUS_CUTOFF and not plan_has_status(content):
            errors.append(f"{relative}：{PLAN_STATUS_CUTOFF} 起新增的计划必须在文件头标注 status 或“状态：”")
        for match in MARKDOWN_LINK.finditer(content):
            raw_target = match.group(1).strip()
            target = link_path(raw_target)
            if is_current_doc(relative) and "#" in raw_target and not raw_target.startswith(("http://", "https://", "mailto:")):
                fragment = unquote(re.split(r"[#]", raw_target.strip("<>").split()[0], maxsplit=1)[1]) if "#" in raw_target else ""
                anchor_file = source if not target else (source.parent / target).resolve()
                if fragment and anchor_file.suffix == ".md" and anchor_file.exists() and fragment.lower() not in heading_slugs(anchor_file, slug_cache):
                    errors.append(f"{relative}：锚点不存在：{raw_target}")
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


def self_test() -> list[str]:
    """用临时仓库构造正反例，确认每条规则既能发现问题，也不误报合理例外。"""
    failures: list[str] = []
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp).resolve()
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        files = {
            "Makefile": "check:\n\techo ok\n\tcargo test -p demo --test merged -- a::\n\tnode --test scripts/x.cjs\n",
            "demo/tests/merged/main.rs": "mod a;\n",
            "demo/tests/single.rs": "",
            "README.md": "# 说明\n\n运行 `make check`。\n业务示例 `make deploy-go-release`。\n",
            "docs/runbooks/good.md": "# 好文档\n\n## 小节 A\n\n见 [本节](#小节-a)、[另一份](other.md#目标标题)。路径 `docs/runbooks/other.md`，占位 `docs/runbooks/<name>.md`，Obsidian `specs/001-example/spec.md`。\n\n```bash\nmake check\nmake -n check\n```\n",
            "docs/runbooks/other.md": "# 另一份\n\n## 目标标题\n",
            "docs/runbooks/test-refs.md": "# 引用\n\n`cargo test -p demo --test merged`、`cargo test -p demo --test single`。\n",
            "docs/runbooks/bad-test.md": "# 坏引用\n\n`cargo test -p demo --test gone_target`。\n",
            "docs/runbooks/bad.md": "# 坏文档\n\n运行 `make nope`，路径 `docs/runbooks/missing.md`，链接 [坏锚点](other.md#没有这个标题)。\n",
            "docs/plans/2026-11-06-new-plan.md": "# 新计划\n\n没有状态头。\n",
            "docs/plans/2026-11-06-ok-plan.md": "---\nstatus: active\n---\n# 新计划\n",
            "docs/solutions/old.md": "---\nstatus: obsolete\n---\n# 过时\n\n历史命令 `make gone-target` 与 `docs/gone-path.md` 不检查。\n",
            "docs/plans/2026-08-01-old-plan.md": "# 旧计划\n\n历史文件不要求状态头，也不检查 `make legacy` 与 `docs/gone.md`。\n",
        }
        for relative, text in files.items():
            target = root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text, encoding="utf-8")
        for required in REQUIRED_FILES:
            target = root / required
            if not target.exists():
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text("# 占位\n", encoding="utf-8")
        errors = "\n".join(check(root))
        expect_present = ["--test gone_target", "make nope", "docs/runbooks/missing.md", "没有这个标题", "2026-11-06-new-plan.md"]
        expect_absent = ["make check", "deploy-go-release", "小节-a", "目标标题", "other.md\n", "<name>", "specs/001", "ok-plan", "old-plan", "make legacy", "docs/gone.md", "make -n", "gone-target", "gone-path", "--test merged", "--test single", "--test scripts"]
        for text in expect_present:
            if text not in errors:
                failures.append(f"自检失败：应当报告但没有报告：{text}")
        for text in expect_absent:
            if text in errors:
                failures.append(f"自检失败：不应误报：{text}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description="检查仓库 Markdown 文档入口、相对链接、make 目标、仓库路径与锚点")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--self-test", action="store_true", help="运行检查规则自检（不检查仓库内容）")
    args = parser.parse_args()
    if args.self_test:
        failures = self_test()
        if failures:
            for failure in failures:
                print(failure, file=sys.stderr)
            return 1
        print("文档检查规则自检通过")
        return 0
    root = args.root.resolve()
    errors = check(root)
    if errors:
        print(f"文档检查失败：{len(errors)} 个问题", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    count = len(repository_files(root))
    print(f"文档检查通过：已检查 {count} 个 Markdown 文件，入口、链接、make 目标、仓库路径与锚点均有效")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
