import { Plus, X } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { Button } from "../../components/Button";
import { TextInput } from "../../components/form";

function normalizeTag(value: string) {
  return value.trim().replace(/\s+/g, " ");
}

function tagError(value: string): string | null {
  if (!value) return "标签不能为空";
  if (value.length > 40) return "标签最多 40 个字符";
  if (
    Array.from(value).some((char) => {
      const code = char.codePointAt(0) ?? 0;
      return code < 32 || code === 127;
    })
  ) {
    return "标签不能包含控制字符";
  }
  return null;
}

export function TagPicker({
  availableTags,
  value,
  onChange,
  disabled = false,
}: {
  availableTags: string[];
  value: string[];
  onChange(next: string[]): void;
  disabled?: boolean;
}) {
  const [creating, setCreating] = useState(false);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);

  function add(raw: string) {
    const tag = normalizeTag(raw);
    const problem = tagError(tag);
    if (problem) {
      setError(problem);
      return;
    }
    if (value.some((existing) => existing.toLocaleLowerCase() === tag.toLocaleLowerCase())) {
      setDraft("");
      setCreating(false);
      setError(null);
      return;
    }
    if (value.length >= 10) {
      setError("一个应用最多关联 10 个标签");
      return;
    }
    onChange([...value, tag]);
    setDraft("");
    setCreating(false);
    setError(null);
  }

  const options = availableTags.filter(
    (tag) => !value.some((existing) => existing.toLocaleLowerCase() === tag.toLocaleLowerCase()),
  );

  return (
    <div className={`tag-picker${disabled ? " is-disabled" : ""}`}>
      {value.length > 0 ? (
        <div className="tag-picker-value">
          {value.map((tag) => (
            <button
              type="button"
              className="tag-badge tag-badge--remove"
              key={tag}
              disabled={disabled}
              title={`移除标签 ${tag}`}
              onClick={() => onChange(value.filter((existing) => existing !== tag))}
            >
              <span>{tag}</span>
              <X aria-hidden="true" />
            </button>
          ))}
        </div>
      ) : (
        <p className="tag-picker-empty">暂未选择标签</p>
      )}
      <div className="tag-picker-options">
        {options.map((tag) => (
          <button
            type="button"
            className="tag-option"
            key={tag}
            disabled={disabled}
            onClick={() => add(tag)}
          >
            {tag}
          </button>
        ))}
        {creating ? (
          <div className="tag-create-form">
            <TextInput
              autoFocus
              maxLength={40}
              placeholder="输入新标签名称"
              aria-label="新标签名称"
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={(event: KeyboardEvent<HTMLInputElement>) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  add(draft);
                }
              }}
            />
            <Button
              type="button"
              tone="primary"
              disabled={!draft.trim()}
              onClick={() => add(draft)}
            >
              添加
            </Button>
            <Button
              type="button"
              onClick={() => {
                setCreating(false);
                setDraft("");
                setError(null);
              }}
            >
              取消
            </Button>
          </div>
        ) : (
          <button
            type="button"
            className="tag-create"
            disabled={disabled}
            onClick={() => {
              setCreating(true);
              setError(null);
            }}
          >
            <Plus aria-hidden="true" />
            新建标签
          </button>
        )}
      </div>
      {error ? <p className="tag-picker-error" role="alert">{error}</p> : null}
    </div>
  );
}

export function TagPickerField({
  availableTags,
  value,
  onChange,
  hint,
  disabled = false,
}: {
  availableTags: string[];
  value: string[];
  onChange(next: string[]): void;
  hint?: string;
  disabled?: boolean;
}) {
  return (
    <div className="form-field form-span">
      <span>标签</span>
      <TagPicker availableTags={availableTags} value={value} onChange={onChange} disabled={disabled} />
      {hint ? <small>{hint}</small> : null}
    </div>
  );
}
