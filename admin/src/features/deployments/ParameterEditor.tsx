interface JsonSchemaProperty {
  type?: string;
  title?: string;
  description?: string;
  enum?: unknown[];
  default?: unknown;
  minimum?: number;
  maximum?: number;
}

interface JsonSchema {
  properties?: Record<string, JsonSchemaProperty>;
  required?: string[];
}

export function schemaDefaults(schema: unknown) {
  const result: Record<string, unknown> = {};
  for (const [name, property] of Object.entries(asSchema(schema).properties ?? {})) {
    if (property.default !== undefined) result[name] = property.default;
    else if (property.type === "boolean") result[name] = false;
  }
  return result;
}

export function ParameterEditor({ schema, value, disabled, onChange }: { schema: unknown; value: Record<string, unknown>; disabled?: boolean; onChange(value: Record<string, unknown>): void }) {
  const parsed = asSchema(schema);
  const entries = Object.entries(parsed.properties ?? {});
  if (entries.length === 0) return <p className="notice">该目标不需要额外参数。</p>;
  return <div className="parameter-grid">{entries.map(([name, property]) => {
    const label = property.title || name;
    const required = parsed.required?.includes(name) ?? false;
    const current = value[name];
    if (property.type === "boolean") return <label className="toggle-row" key={name}><span>{label}<small>{property.description}</small></span><input type="checkbox" disabled={disabled} checked={current === true} onChange={(event) => onChange({ ...value, [name]: event.target.checked })} /></label>;
    if (property.enum) return <label key={name}>{label}<select required={required} disabled={disabled} value={String(current ?? "")} onChange={(event) => onChange({ ...value, [name]: event.target.value })}><option value="">请选择</option>{property.enum.map((option) => <option key={String(option)} value={String(option)}>{String(option)}</option>)}</select><small>{property.description}</small></label>;
    const numeric = property.type === "integer" || property.type === "number";
    return <label key={name}>{label}<input type={numeric ? "number" : "text"} required={required} disabled={disabled} min={property.minimum} max={property.maximum} value={String(current ?? "")} onChange={(event) => {
      const next = { ...value };
      if (numeric && event.target.value === "") delete next[name];
      else next[name] = numeric ? Number(event.target.value) : event.target.value;
      onChange(next);
    }} /><small>{property.description}</small></label>;
  })}</div>;
}

function asSchema(value: unknown): JsonSchema {
  return value && typeof value === "object" && !Array.isArray(value) ? value as JsonSchema : {};
}
