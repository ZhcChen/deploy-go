import { Check, ChevronDown } from "lucide-react";
import {
  Children,
  isValidElement,
  useEffect,
  forwardRef,
  useId,
  useMemo,
  useRef,
  useState,
  type InputHTMLAttributes,
  type ReactNode,
  type TextareaHTMLAttributes,
} from "react";

export function Field({
  label,
  hint,
  className = "",
  children,
}: {
  label: string;
  hint?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <label className={`form-field ${className}`.trim()}>
      <span>{label}</span>
      {children}
      {hint ? <small>{hint}</small> : null}
    </label>
  );
}

export const TextInput = forwardRef<
  HTMLInputElement,
  InputHTMLAttributes<HTMLInputElement>
>(function TextInput({ className = "", ...props }, ref) {
  return <input ref={ref} className={`form-control ${className}`.trim()} {...props} />;
});

interface SelectOption {
  value: string;
  label: ReactNode;
}

export interface SelectValueEvent {
  target: { value: string };
}

interface SelectProps {
  value: string;
  onChange(event: SelectValueEvent): void;
  children: ReactNode;
  disabled?: boolean;
  required?: boolean;
  className?: string;
  searchable?: boolean;
  searchPlaceholder?: string;
  ariaLabel?: string;
}

function collectOptions(children: ReactNode): SelectOption[] {
  return Children.toArray(children).flatMap((child) => {
    if (!isValidElement<{ value?: unknown; children?: ReactNode }>(child) || child.type !== "option") return [];
    return [{ value: String(child.props.value ?? ""), label: child.props.children }];
  });
}

export function Select({ value, onChange, children, disabled = false, required, className = "", searchable = false, searchPlaceholder = "搜索选项", ariaLabel }: SelectProps) {
  const options = useMemo(() => collectOptions(children), [children]);
  const [search, setSearch] = useState("");
  const filteredOptions = searchable ? options.filter((option) => String(option.label).toLocaleLowerCase().includes(search.trim().toLocaleLowerCase())) : options;
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const listId = useId();
  const selected = options.find((option) => option.value === value);
  const visibleLabel = selected?.label ?? options[0]?.label ?? "";

  useEffect(() => {
    if (!open) return;
    requestAnimationFrame(() => {
      if (searchable && activeIndex < 0) { searchRef.current?.focus(); return; }
      menuRef.current?.querySelectorAll<HTMLElement>("[role='option']")[activeIndex]?.focus();
    });
  }, [open, activeIndex, searchable]);

  useEffect(() => {
    if (!open) return;
    function handlePointerDown(event: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(event.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", handlePointerDown);
    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, [open]);

  function choose(index: number) {
    const option = filteredOptions[index];
    if (!option) return;
    onChange({ target: { value: option.value } });
    setOpen(false);
    buttonRef.current?.focus();
  }

  function openMenu() {
    setSearch("");
    const selectedIndex = options.findIndex((option) => option.value === value);
    setActiveIndex(searchable ? -1 : selectedIndex >= 0 ? selectedIndex : 0);
    setOpen(true);
  }

  function handleButtonKeyDown(event: React.KeyboardEvent<HTMLButtonElement>) {
    if (disabled) return;
    if (event.key === "ArrowDown" || event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      openMenu();
    }
  }

  function handleMenuKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229) return;
    const inSearch = event.target === searchRef.current;
    if (inSearch && !["ArrowDown", "Escape", "Enter", "Tab"].includes(event.key)) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((index) => Math.min(index + 1, filteredOptions.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((index) => Math.max(index - 1, searchable ? -1 : 0));
    } else if (event.key === "Home") {
      event.preventDefault();
      setActiveIndex(0);
    } else if (event.key === "End") {
      event.preventDefault();
      setActiveIndex(filteredOptions.length - 1);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      choose(inSearch ? 0 : activeIndex);
    } else if (event.key === "Escape") {
      event.preventDefault();
      setOpen(false);
      buttonRef.current?.focus();
    } else if (event.key === "Tab") {
      setOpen(false);
    }
  }

  return (
    <div className="select-root" ref={rootRef}>
      <button
        ref={buttonRef}
        type="button"
        className={`form-control select-control ${className}`.trim()}
        disabled={disabled}
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        aria-required={required || undefined}
        onClick={() => {
          if (open) setOpen(false);
          else openMenu();
        }}
        onKeyDown={handleButtonKeyDown}
      >
        <span className="select-value">{visibleLabel}</span>
        <ChevronDown className="select-chevron" aria-hidden="true" />
      </button>
      {open ? (
        <div ref={menuRef} className="select-menu" onKeyDown={handleMenuKeyDown}>
          {searchable ? <input ref={searchRef} className="form-control select-search" aria-label={searchPlaceholder} placeholder={searchPlaceholder} value={search} onChange={(event) => { setSearch(event.target.value); setActiveIndex(-1); }} /> : null}
          <div id={listId} role="listbox" aria-label={ariaLabel ?? (searchable ? searchPlaceholder : String(visibleLabel))}>
          {filteredOptions.map((option, index) => (
            <button
              key={option.value}
              type="button"
              role="option"
              tabIndex={-1}
              aria-selected={option.value === value}
              className={`select-option${index === activeIndex ? " is-highlighted" : ""}${option.value === value ? " is-selected" : ""}`}
              onClick={() => choose(index)}
              onMouseEnter={() => { if (!searchable) setActiveIndex(index); }}
            >
              <Check aria-hidden="true" />
              <span>{option.label}</span>
            </button>
          ))}
          </div>
          {filteredOptions.length === 0 ? <div className="select-empty" role="status">没有匹配的选项</div> : null}
        </div>
      ) : null}
    </div>
  );
}

export const TextArea = forwardRef<
  HTMLTextAreaElement,
  TextareaHTMLAttributes<HTMLTextAreaElement>
>(function TextArea({ className = "", ...props }, ref) {
  return <textarea ref={ref} className={`form-control ${className}`.trim()} {...props} />;
});
