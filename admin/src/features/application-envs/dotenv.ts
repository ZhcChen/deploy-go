import type { DotenvAssignment, DotenvDocument, DotenvError, DotenvLine } from "../../api/contracts";

const keyPattern = /^[A-Za-z_][A-Za-z0-9_]*$/;
const maxBytes = 1024 * 1024;

export function parseDotenv(content: string): DotenvDocument {
  const errors: DotenvError[] = [];
  if (new TextEncoder().encode(content).byteLength > maxBytes) {
    errors.push({ line: 1, code: "content_too_large", message: "Env 内容超过 1 MiB 上限" });
    return { lines: [], errors };
  }
  if ([...content].some((character) => isControl(character) && character !== "\n")) {
    errors.push({ line: 1, code: "control_character", message: "Env 内容包含不允许的控制字符" });
    return { lines: [], errors };
  }

  const firstLines = new Map<string, number>();
  const lines = content.split("\n").map<DotenvLine>((raw, index) => {
    const line = index + 1;
    const trimmed = raw.trim();
    if (!trimmed) return { kind: "blank", raw };
    if (trimmed.startsWith("#")) return { kind: "comment", raw };
    if (trimmed.startsWith("export ")) {
      errors.push({ line, code: "export_not_supported", message: "不支持 export 语法" });
      return { kind: "invalid", raw };
    }
    const equals = trimmed.indexOf("=");
    if (equals < 0) {
      errors.push({ line, code: "assignment_required", message: "必须使用 KEY=VALUE 语法" });
      return { kind: "invalid", raw };
    }
    const key = trimmed.slice(0, equals);
    const rawValue = trimmed.slice(equals + 1);
    if (!keyPattern.test(key)) {
      errors.push({ line, code: "invalid_key", message: "变量名格式不正确" });
    } else if (firstLines.has(key)) {
      errors.push({ line, code: "duplicate_key", message: `变量名 ${key} 与第 ${firstLines.get(key)} 行重复` });
    } else {
      firstLines.set(key, line);
    }
    const quote = rawValue.startsWith("'") ? "'" : rawValue.startsWith('"') ? '"' : null;
    if (rawValue.includes("$")) errors.push({ line, code: "expansion_not_supported", message: "不支持变量或命令展开" });
    if (quote) {
      if (rawValue.length < 2 || !rawValue.endsWith(quote)) {
        errors.push({ line, code: "unclosed_quote", message: "引号未闭合" });
      } else if (rawValue.slice(1, -1).includes(quote)) {
        errors.push({ line, code: "unsupported_quote", message: "引号值内不支持同类引号" });
      }
    } else if (rawValue.includes("'") || rawValue.includes('"')) {
      errors.push({ line, code: "unsupported_quote", message: "引号只能包裹完整值" });
    }
    const closed = quote && rawValue.length >= 2 && rawValue.endsWith(quote);
    return { kind: "assignment", key, value: closed ? rawValue.slice(1, -1) : rawValue, quote };
  });
  return { lines, errors };
}

export function serializeDotenv(lines: DotenvLine[]) {
  return lines.map((line) => {
    if (line.kind !== "assignment") return line.raw;
    const quote = line.quote ?? "";
    return `${line.key}=${quote}${line.value}${quote}`;
  }).join("\n");
}

export function assignments(document: DotenvDocument): Array<DotenvAssignment & { index: number }> {
  return document.lines.flatMap((line, index) => line.kind === "assignment" ? [{ ...line, index }] : []);
}

export function updateAssignment(lines: DotenvLine[], index: number, patch: Partial<DotenvAssignment>) {
  return lines.map((line, current) => current === index && line.kind === "assignment" ? { ...line, ...patch } : line);
}

export function appendAssignment(lines: DotenvLine[]) {
  const next = [...lines];
  const insertion = next.length > 0 && next.at(-1)?.kind === "blank" ? next.length - 1 : next.length;
  next.splice(insertion, 0, { kind: "assignment", key: "NEW_KEY", value: "", quote: null });
  return next;
}

export function maskedDiff(before: string, after: string) {
  const previous = new Map(assignments(parseDotenv(before)).map((item) => [item.key, item.value]));
  const current = new Map(assignments(parseDotenv(after)).map((item) => [item.key, item.value]));
  const result: string[] = [];
  for (const [key, value] of previous) {
    if (!current.has(key)) result.push(`- ${key}=••••••`);
    else if (current.get(key) !== value) result.push(`~ ${key}=••••••`);
  }
  for (const key of current.keys()) if (!previous.has(key)) result.push(`+ ${key}=••••••`);
  if (result.length === 0 && before !== after) result.push("~ 注释或顺序已调整");
  return result;
}

function isControl(character: string) {
  const code = character.codePointAt(0) ?? 0;
  return code <= 0x1f || (code >= 0x7f && code <= 0x9f);
}
