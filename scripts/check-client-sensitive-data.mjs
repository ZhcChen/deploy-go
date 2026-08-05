import { readFile, readdir, stat } from "node:fs/promises";
import { relative, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const patterns = [
  { name: "SSH/private key", regex: /-----BEGIN (?:OPENSSH|RSA|EC|DSA|ENCRYPTED)? ?PRIVATE KEY-----/g },
  { name: "OpenSSH private key payload", regex: /b3BlbnNzaC1rZXktdjEAAAAA[A-Za-z0-9+/=]{24,}/g },
  { name: "session cookie", regex: /(?:^|[;\s])deploy_go_session=[A-Za-z0-9._~-]{16,}/g },
  { name: "credential master key", regex: /DEPLOY_GO_(?:PREVIOUS_)?MASTER_KEY\s*=\s*["']?[A-Za-z0-9+/=_-]{32,}/g },
  { name: "CSRF token", regex: /(?:csrf_token|csrfToken|X-CSRF-Token)["']?\s*[:=]\s*["'][A-Za-z0-9._~-]{16,}/g },
  { name: "script secret", regex: /(?:SCRIPT_SECRET|DEPLOY_TOKEN|API_TOKEN|API_SECRET)\s*=\s*["']?[A-Za-z0-9+/._=~-]{16,}/g },
  { name: "JWT-like token", regex: /eyJ[A-Za-z0-9_-]{16,}\.[A-Za-z0-9_-]{16,}\.[A-Za-z0-9_-]{16,}/g },
];

function matches(pattern, value) {
  pattern.regex.lastIndex = 0;
  return pattern.regex.test(value);
}

function selfTest() {
  const canaries = new Map([
    ["credential master key", "DEPLOY_GO_MASTER_KEY=QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE="],
    ["CSRF token", '"csrf_token":"csrf-canary-123456789"'],
    ["session cookie", "deploy_go_session=session-canary-123456789"],
    ["script secret", "DEPLOY_TOKEN=deploy-canary-123456789"],
  ]);
  for (const [name, canary] of canaries) {
    const pattern = patterns.find((candidate) => candidate.name === name);
    if (!pattern || !matches(pattern, canary)) throw new Error(`敏感模式自检失败：${name}`);
  }
  for (const pattern of patterns) {
    if (matches(pattern, "DEPLOY_TOKEN_FILE=/srv/secrets/app/token")) {
      throw new Error(`敏感模式误报路径引用：${pattern.name}`);
    }
  }
  console.log(`客户端敏感模式自检通过（${canaries.size} 个 canary）`);
}

async function collect(path) {
  const info = await stat(path);
  if (info.isFile()) return [path];
  if (!info.isDirectory()) return [];
  const entries = await readdir(path, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    if (["node_modules", ".dart_tool", "generated"].includes(entry.name)) continue;
    files.push(...await collect(resolve(path, entry.name)));
  }
  return files;
}

async function defaultFiles() {
  const paths = [
    "admin/src",
    "admin/e2e",
    "admin-app/lib",
    "admin-app/test",
    "admin-app/integration_test",
    "test-fixtures",
  ];
  return (await Promise.all(paths.map((path) => collect(resolve(root, path))))).flat();
}

const inputs = process.argv.slice(2).filter((input) => input !== "--self-test");
if (process.argv.includes("--self-test")) selfTest();
const files = inputs.length
  ? (await Promise.all(inputs.map((path) => collect(resolve(root, path))))).flat()
  : await defaultFiles();
const findings = [];
for (const file of files) {
  let content;
  try {
    content = await readFile(file, "utf8");
  } catch {
    continue;
  }
  for (const pattern of patterns) {
    pattern.regex.lastIndex = 0;
    for (const match of content.matchAll(pattern.regex)) {
      const line = content.slice(0, match.index).split("\n").length;
      findings.push(`${relative(root, file)}:${line} ${pattern.name}`);
    }
  }
}

if (findings.length) {
  console.error("客户端敏感模式扫描失败：");
  for (const finding of findings) console.error(`- ${finding}`);
  process.exitCode = 1;
} else {
  console.log(`客户端敏感模式扫描通过（${files.length} 个文件）`);
}
