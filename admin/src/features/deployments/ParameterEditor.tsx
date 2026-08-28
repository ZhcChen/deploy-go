import { Field, Select, TextInput } from "../../components/form";

interface JsonSchemaProperty {
  type?: string;
  title?: string;
  description?: string;
  enum?: unknown[];
  default?: unknown;
  minimum?: number;
  maximum?: number;
  "x-options"?: unknown[];
  "x-default-selected"?: unknown[];
}

interface JsonSchema {
  properties?: Record<string, JsonSchemaProperty>;
  required?: string[];
}

export function schemaDefaults(schema: unknown) {
  const result: Record<string, unknown> = {};
  for (const [name, property] of Object.entries(asSchema(schema).properties ?? {})) {
    if (name === "modules" && moduleOptions(schema).length > 0) result[name] = moduleDefaults(schema).join(",");
    else if (property.default !== undefined) result[name] = property.default;
    else if (property.type === "boolean") result[name] = false;
  }
  return result;
}

export function ParameterEditor({ schema, value, disabled, hiddenNames = [], showEmpty = true, onChange }: { schema: unknown; value: Record<string, unknown>; disabled?: boolean; hiddenNames?: string[]; showEmpty?: boolean; onChange(value: Record<string, unknown>): void }) {
  const parsed = asSchema(schema);
  const entries = Object.entries(parsed.properties ?? {}).filter(([name]) => !hiddenNames.includes(name));
  if (entries.length === 0) return showEmpty ? <p className="notice">该目标不需要额外参数。</p> : null;
  return <div className="parameter-grid">{entries.map(([name, property]) => {
    const label = property.title || name;
    const required = parsed.required?.includes(name) ?? false;
    const current = value[name];
    if (property.type === "boolean") return <label className="toggle-row" key={name}><span>{label}<small>{property.description}</small></span><input type="checkbox" disabled={disabled} checked={current === true} onChange={(event) => onChange({ ...value, [name]: event.target.checked })} /></label>;
    if (property.enum) return <Field label={label} hint={property.description} key={name}><Select required={required} disabled={disabled} value={String(current ?? "")} onChange={(event) => onChange({ ...value, [name]: event.target.value })}><option value="">请选择</option>{property.enum.map((option) => <option key={String(option)} value={String(option)}>{String(option)}</option>)}</Select></Field>;
    const numeric = property.type === "integer" || property.type === "number";
    return <Field label={label} hint={property.description} key={name}><TextInput type={numeric ? "number" : "text"} required={required} disabled={disabled} min={property.minimum} max={property.maximum} value={String(current ?? "")} onChange={(event) => {
      const next = { ...value };
      if (numeric && event.target.value === "") delete next[name];
      else next[name] = numeric ? Number(event.target.value) : event.target.value;
      onChange(next);
    }} /></Field>;
  })}</div>;
}

export function ModuleSelector({ schema, value, disabled, onChange }: { schema: unknown; value: unknown; disabled?: boolean; onChange(value: string): void }) {
  const options = moduleOptions(schema);
  if (options.length === 0) return <p className="notice notice--warning">目标尚未配置模块选项，请先在目标参数 schema 的 <code>modules.x-options</code> 中声明模块。</p>;
  const selected = new Set(String(value ?? "").split(",").map((item) => item.trim()).filter(Boolean));
  const allSelected = options.every((option) => selected.has(option));
  function toggle(option: string) {
    const next = new Set(selected);
    if (next.has(option)) next.delete(option);
    else next.add(option);
    onChange(options.filter((item) => next.has(item)).join(","));
  }
  return <fieldset className="module-selector" disabled={disabled}>
    <legend>部署模块</legend>
    <div className="module-selector__toolbar"><span>已选择 {selected.size} / {options.length}</span><button type="button" onClick={() => onChange(allSelected ? "" : options.join(","))}>{allSelected ? "取消全选" : "全选"}</button></div>
    <div className="module-selector__options">{options.map((option) => <label key={option}><input type="checkbox" checked={selected.has(option)} onChange={() => toggle(option)} /><span>{option}</span></label>)}</div>
  </fieldset>;
}

export function moduleOptions(schema: unknown) {
  const options = asSchema(schema).properties?.modules?.["x-options"];
  return Array.isArray(options) ? options.filter((item): item is string => typeof item === "string" && item.length > 0) : [];
}

export function moduleDefaults(schema: unknown) {
  const options = moduleOptions(schema);
  const configured = asSchema(schema).properties?.modules?.["x-default-selected"];
  if (!Array.isArray(configured)) return options;
  const selected = new Set(configured.filter((item): item is string => typeof item === "string" && options.includes(item)));
  return options.filter((option) => selected.has(option));
}

function asSchema(value: unknown): JsonSchema {
  return value && typeof value === "object" && !Array.isArray(value) ? value as JsonSchema : {};
}
