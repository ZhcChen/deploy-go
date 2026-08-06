import {
  cpSync,
  existsSync,
  lstatSync,
  mkdtempSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve, sep } from "node:path";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";

const root = realpathSync(resolve(dirname(fileURLToPath(import.meta.url)), ".."));
const checkOnly = process.argv.includes("--check");
const generatorVersion = "7.24.0";
const generatorSha256 =
  "4b83ccc6fd43056c8c631cd0195e5100bd0550912502527bab09ac76152dab0c";
const unknown = process.argv.slice(2).filter((argument) => argument !== "--check");

if (unknown.length > 0) {
  throw new Error(`未知参数: ${unknown.join(" ")}`);
}

const targets = [
  {
    name: "Web",
    generator: "typescript-fetch",
    destination: "admin/src/api/generated",
    additionalProperties: [
      "supportsES6=true",
      "typescriptThreePlus=true",
      "useSingleRequestParameter=true",
      "withoutRuntimeChecks=false",
    ],
  },
  {
    name: "Flutter",
    generator: "dart-dio",
    destination: "admin-app/lib/api/generated",
    additionalProperties: [
      "pubName=deploy_go_api_client",
      "pubLibrary=deploy_go_api_client.api",
      "pubVersion=0.1.0",
      "pubDescription=deploy-go自动生成API客户端",
      "pubHomepage=https://github.com/ZhcChen/deploy-go",
      "pubRepository=https://github.com/ZhcChen/deploy-go",
      "serializationLibrary=built_value",
    ],
    buildDart: true,
  },
];

verifyGeneratorIntegrity();
const workRoot = mkdtempSync(join(tmpdir(), "deploy-go-api-clients-"));

try {
  for (const target of targets) {
    target.output = join(workRoot, target.generator);
    generate(target, target.output);
    if (target.generator === "typescript-fetch") {
      removeUnusedWebRuntimeImports(target.output);
      attachWebRequestTransformers(target.output);
      attachWebResponseTransformers(target.output);
    }
    if (target.buildDart) {
      alignDartSdkConstraint(target.output);
      const committedLock = join(insideRoot(target.destination), "pubspec.lock");
      const hasCommittedLock = existsSync(committedLock);
      if (hasCommittedLock) {
        cpSync(committedLock, join(target.output, "pubspec.lock"));
      }
      const analysisOptions = join(target.output, "analysis_options.yaml");
      writeFileSync(
        analysisOptions,
        `${readFileSync(analysisOptions, "utf8")}    unused_import: ignore\n`,
        "utf8",
      );
      run(
        "dart",
        ["pub", "get", ...(hasCommittedLock ? ["--enforce-lockfile"] : [])],
        target.output,
      );
      run(
        "dart",
        ["run", "build_runner", "build", "--delete-conflicting-outputs"],
        target.output,
      );
      removeUnsafeDartCookieAuth(target.output);
      rmSync(join(target.output, ".dart_tool"), { recursive: true, force: true });
      rmSync(join(target.output, ".gitignore"), { force: true });
      rmSync(join(target.output, "README.md"), { force: true });
    }
    rmSync(join(target.output, ".openapi-generator-ignore"), { force: true });
    rmSync(join(target.output, ".openapi-generator", "FILES"), { force: true });
    normalizeGeneratedText(target.output);
  }
  verifyGeneratedCoverage(targets);

  if (checkOnly) {
    for (const target of targets) {
      const destination = insideRoot(target.destination);
      const differences = compareTrees(target.output, destination);
      if (differences.length > 0) {
        console.error(`${target.name} API client 已漂移：`);
        for (const difference of differences.slice(0, 30)) {
          console.error(`  ${difference}`);
        }
        if (differences.length > 30) {
          console.error(`  另有 ${differences.length - 30} 项差异`);
        }
        process.exitCode = 1;
      }
    }
    verifyInvalidSpec(workRoot);
  } else {
    installGeneratedTargets(targets);
  }
} finally {
  rmSync(workRoot, { recursive: true, force: true });
}

function alignDartSdkConstraint(output) {
  const pubspec = join(output, "pubspec.yaml");
  const source = readFileSync(pubspec, "utf8");
  const aligned = source.replace(
    /sdk: ['"]>=2\.18\.0 <4\.0\.0['"]/,
    "sdk: '>=3.11.0 <4.0.0'",
  );
  if (aligned === source) {
    throw new Error("无法固定 Flutter 生成客户端的 Dart SDK 版本");
  }
  writeFileSync(pubspec, aligned, "utf8");
}

function generate(target, output) {
  run(
    process.execPath,
    [
      generatorEntrypoint(),
      "generate",
      "--input-spec",
      join(root, "api", "openapi", "openapi.json"),
      "--generator-name",
      target.generator,
      "--output",
      output,
      "--global-property",
      "apiDocs=false,modelDocs=false,apiTests=false,modelTests=false",
      "--additional-properties",
      target.additionalProperties.join(","),
    ],
    root,
  );
}

function generatorEntrypoint() {
  return join(
    root,
    "node_modules",
    "@openapitools",
    "openapi-generator-cli",
    "main.js",
  );
}

function verifyGeneratorIntegrity() {
  const jar = join(
    root,
    "node_modules",
    "@openapitools",
    "openapi-generator-cli",
    "versions",
    `${generatorVersion}.jar`,
  );
  if (!existsSync(jar)) {
    throw new Error(`缺少 OpenAPI Generator ${generatorVersion}，请先运行 npm ci`);
  }
  const actual = createHash("sha256").update(readFileSync(jar)).digest("hex");
  if (actual !== generatorSha256) {
    throw new Error(
      `OpenAPI Generator ${generatorVersion} 完整性校验失败: ${actual}`,
    );
  }
}

function removeUnsafeDartCookieAuth(output) {
  const api = join(output, "lib", "src", "api.dart");
  const barrel = join(output, "lib", "deploy_go_api_client.dart");
  let apiSource = readFileSync(api, "utf8");
  const original = apiSource;
  apiSource = apiSource.replace(
    "import 'package:deploy_go_api_client/src/auth/api_key_auth.dart';\n",
    "",
  );
  apiSource = apiSource.replace("        ApiKeyAuthInterceptor(),\n", "");
  apiSource = apiSource.replace(
    /\n  void setApiKey\(String name, String apiKey\) \{[\s\S]*?\n  \}\n\n  \/\/\/ Removes the API key[\s\S]*?\n  \}\n/,
    "",
  );
  if (apiSource === original || apiSource.includes("ApiKeyAuthInterceptor")) {
    throw new Error("无法移除 Dart 客户端不安全的 cookieAuth API-key 入口");
  }
  writeFileSync(api, apiSource, "utf8");

  const barrelSource = readFileSync(barrel, "utf8").replace(
    "export 'package:deploy_go_api_client/src/auth/api_key_auth.dart';\n",
    "",
  );
  if (barrelSource.includes("api_key_auth.dart")) {
    throw new Error("Dart 客户端仍导出不安全的 API-key interceptor");
  }
  writeFileSync(barrel, barrelSource, "utf8");
  rmSync(join(output, "lib", "src", "auth", "api_key_auth.dart"));
}

function removeUnusedWebRuntimeImports(output) {
  let removed = 0;
  for (const source of sourceFiles(output, ".ts")) {
    const content = readFileSync(source, "utf8");
    const cleaned = content.replace(
      "import { mapValues } from '../runtime';\n",
      "",
    );
    if (cleaned !== content) {
      removed += 1;
      writeFileSync(source, cleaned, "utf8");
    }
  }
  if (removed === 0) {
    throw new Error("Web 客户端未发现预期的未使用 mapValues import");
  }
  const remaining = sourceFiles(output, ".ts").filter((source) =>
    readFileSync(source, "utf8").includes("mapValues"),
  );
  if (remaining.length > 0) {
    throw new Error("Web 客户端仍包含不可用的 mapValues 引用");
  }
}

function sourceFiles(directory, extension) {
  const files = [];
  visit(directory);
  return files;

  function visit(current) {
    for (const entry of readdirSync(current).sort()) {
      const absolute = join(current, entry);
      if (statSync(absolute).isDirectory()) visit(absolute);
      else if (absolute.endsWith(extension)) files.push(absolute);
    }
  }
}

function attachWebResponseTransformers(output) {
  const apiDirectory = join(output, "apis");
  let transformed = 0;
  for (const source of sourceFiles(apiDirectory, ".ts")) {
    let content = readFileSync(source, "utf8");
    const converterModels = new Set();
    content = content.replace(
      /(async \w+Raw\([\s\S]*?\): Promise<runtime\.ApiResponse<([A-Za-z]\w*)>> \{)([\s\S]*?)(\n    \})/g,
      (method, signature, model, body, ending) => {
        if (model === "void") return method;
        const rawResponse = "new runtime.JSONApiResponse(response);";
        if (!body.includes(rawResponse)) {
          throw new Error(`Web 客户端 ${source} 的 ${model} response 结构不符合预期`);
        }
        const converter = `${model}FromJSON`;
        converterModels.add(model);
        transformed += 1;
        return `${signature}${body.replace(
          rawResponse,
          `new runtime.JSONApiResponse(response, (jsonValue) => ${converter}(jsonValue));`,
        )}${ending}`;
      },
    );
    if (converterModels.size > 0) {
      const converterImport = [...converterModels]
        .sort()
        .map((model) => `import { ${model}FromJSON } from '../models/${model}';`)
        .join("\n");
      content = content.replace(
        "import * as runtime from '../runtime';\n",
        `import * as runtime from '../runtime';\n${converterImport}\n`,
      );
    }
    writeFileSync(source, content, "utf8");
  }
  if (transformed === 0) {
    throw new Error("Web 客户端未发现可绑定的 JSON response transformer");
  }
  const unsafe = sourceFiles(apiDirectory, ".ts").filter((source) =>
    readFileSync(source, "utf8").includes("new runtime.JSONApiResponse(response);"),
  );
  if (unsafe.length > 0) {
    throw new Error("Web 客户端仍包含未转换的 JSON response");
  }
}

function attachWebRequestTransformers(output) {
  const apiDirectory = join(output, "apis");
  let transformed = 0;
  for (const source of sourceFiles(apiDirectory, ".ts")) {
    let content = readFileSync(source, "utf8");
    const converterModels = new Set();
    const parameterModels = new Map(
      [...content.matchAll(/^    ([a-zA-Z]\w*Request): ([A-Z]\w*Request);$/gm)]
        .map((match) => [match[1], match[2]]),
    );
    content = content.replace(
      /body: requestParameters\['([a-zA-Z]\w*Request)'\],/g,
      (body, parameter) => {
        const model = parameterModels.get(parameter);
        if (!model) {
          throw new Error(`Web 客户端 ${source} 无法确定 ${parameter} 的请求模型`);
        }
        converterModels.add(model);
        transformed += 1;
        return `body: ${model}ToJSON(requestParameters['${parameter}']),`;
      },
    );
    if (converterModels.size > 0) {
      const converterImport = [...converterModels]
        .sort()
        .map((model) => `import { ${model}ToJSON } from '../models/${model}';`)
        .join("\n");
      content = content.replace(
        "import * as runtime from '../runtime';\n",
        `import * as runtime from '../runtime';\n${converterImport}\n`,
      );
    }
    writeFileSync(source, content, "utf8");
  }
  if (transformed === 0) {
    throw new Error("Web 客户端未发现可绑定的 JSON request transformer");
  }
  const unsafe = sourceFiles(apiDirectory, ".ts").filter((source) =>
    /body: requestParameters\['[a-zA-Z]\w*Request'\],/.test(
      readFileSync(source, "utf8"),
    ),
  );
  if (unsafe.length > 0) {
    throw new Error("Web 客户端仍包含未转换的 JSON request body");
  }
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.status !== 0) {
    process.stderr.write(result.stdout ?? "");
    process.stderr.write(result.stderr ?? "");
    throw new Error(`${command} 执行失败，退出码 ${result.status ?? "unknown"}`);
  }
}

function verifyInvalidSpec(directory) {
  const invalidSpec = join(directory, "invalid-openapi.json");
  const output = join(directory, "invalid-output");
  writeFileSync(invalidSpec, "{\n", "utf8");
  const result = spawnSync(
    process.execPath,
    [
      generatorEntrypoint(),
      "generate",
      "--input-spec",
      invalidSpec,
      "--generator-name",
      "typescript-fetch",
      "--output",
      output,
    ],
    { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  );
  if (result.status === 0) {
    throw new Error("OpenAPI Generator 未拒绝非法契约");
  }
}

function verifyGeneratedCoverage(generatedTargets) {
  const document = JSON.parse(
    readFileSync(join(root, "api", "openapi", "openapi.json"), "utf8"),
  );
  const operations = Object.values(document.paths).flatMap((pathItem) =>
    Object.values(pathItem)
      .map((operation) => operation.operationId)
      .filter(Boolean)
      .map(toLowerCamelCase),
  );
  const models = Object.keys(document.components?.schemas ?? {});

  for (const target of generatedTargets) {
    const sources = collectSources(target.output).join("\n");
    const missingOperations = operations.filter(
      (operation) => !sources.includes(operation),
    );
    const missingModels = models.filter((model) => !sources.includes(model));
    if (missingOperations.length > 0 || missingModels.length > 0) {
      throw new Error(
        `${target.name} 生成结果缺少契约成员: ${[
          ...missingOperations,
          ...missingModels,
        ].join(", ")}`,
      );
    }
  }
}

function collectSources(directory) {
  const sources = [];
  collect(directory, sources);
  return sources;
}

function collect(current, sources) {
  for (const entry of readdirSync(current).sort()) {
    const absolute = join(current, entry);
    if (statSync(absolute).isDirectory()) {
      collect(absolute, sources);
    } else if (absolute.endsWith(".ts") || absolute.endsWith(".dart")) {
      sources.push(readFileSync(absolute, "utf8"));
    }
  }
}

function normalizeGeneratedText(directory) {
  normalize(directory);
}

function normalize(current) {
  for (const entry of readdirSync(current).sort()) {
    const absolute = join(current, entry);
    if (statSync(absolute).isDirectory()) {
      normalize(absolute);
      continue;
    }
    const content = readFileSync(absolute, "utf8");
    const normalized = `${content.replace(/[ \t]+$/gm, "").replace(/\n+$/, "")}\n`;
    writeFileSync(absolute, normalized, "utf8");
  }
}

function toLowerCamelCase(value) {
  return value.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
}

function installGeneratedTargets(generatedTargets) {
  const prepared = generatedTargets.map((target) => {
    const destination = insideRoot(target.destination);
    const parent = dirname(destination);
    mkdirSync(parent, { recursive: true });
    const suffix = `${target.generator}-${process.pid}`;
    const staging = insideRoot(join(relative(root, parent), `.generated-stage-${suffix}`));
    const backup = insideRoot(join(relative(root, parent), `.generated-backup-${suffix}`));
    rmSync(staging, { recursive: true, force: true });
    rmSync(backup, { recursive: true, force: true });
    cpSync(target.output, staging, { recursive: true });
    return { target, destination, staging, backup, hadDestination: false };
  });
  const installed = [];

  try {
    for (const item of prepared) {
      if (existsSync(item.destination)) {
        renameSync(item.destination, item.backup);
        item.hadDestination = true;
      }
      try {
        renameSync(item.staging, item.destination);
      } catch (error) {
        if (item.hadDestination) {
          renameSync(item.backup, item.destination);
        }
        throw error;
      }
      installed.push(item);
    }
  } catch (error) {
    for (const item of installed.reverse()) {
      rmSync(item.destination, { recursive: true, force: true });
      if (item.hadDestination && existsSync(item.backup)) {
        renameSync(item.backup, item.destination);
      }
    }
    throw error;
  } finally {
    for (const item of prepared) {
      rmSync(item.staging, { recursive: true, force: true });
    }
  }

  for (const item of prepared) {
    rmSync(item.backup, { recursive: true, force: true });
    console.log(`${item.target.name} API client 已生成到 ${item.target.destination}`);
  }
}

function insideRoot(path) {
  const absolute = resolve(root, path);
  if (!absolute.startsWith(`${root}${sep}`)) {
    throw new Error(`生成目录越出仓库: ${path}`);
  }
  let current = root;
  for (const segment of relative(root, absolute).split(sep)) {
    current = join(current, segment);
    if (!existsSync(current)) continue;
    if (lstatSync(current).isSymbolicLink()) {
      throw new Error(`生成目录包含符号链接: ${relative(root, current)}`);
    }
    const resolved = realpathSync(current);
    if (resolved !== root && !resolved.startsWith(`${root}${sep}`)) {
      throw new Error(`生成目录解析到仓库外: ${relative(root, current)}`);
    }
  }
  return absolute;
}

function compareTrees(expectedRoot, actualRoot) {
  const expected = snapshot(expectedRoot);
  const actual = snapshot(actualRoot);
  const paths = new Set([...expected.keys(), ...actual.keys()]);
  return [...paths]
    .sort()
    .flatMap((path) => {
      if (!actual.has(path)) return [`缺少 ${path}`];
      if (!expected.has(path)) return [`多余 ${path}`];
      if (!expected.get(path).equals(actual.get(path))) return [`内容不同 ${path}`];
      return [];
    });
}

function snapshot(directory) {
  const files = new Map();
  if (!existsSync(directory)) return files;
  walk(directory, directory, files);
  return files;
}

function walk(base, current, files) {
  for (const entry of readdirSync(current).sort()) {
    const absolute = join(current, entry);
    if (statSync(absolute).isDirectory()) {
      walk(base, absolute, files);
    } else {
      files.set(relative(base, absolute), readFileSync(absolute));
    }
  }
}
