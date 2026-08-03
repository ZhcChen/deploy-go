import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { Button } from "../../components/Button";

export function ClipboardFallback({ value, label = "复制", failure = "自动复制失败，请选中上方完整内容后手动复制。" }: { value: string; label?: string; failure?: string }) {
  const [state, setState] = useState<"idle" | "copied" | "failed">("idle");

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setState("copied");
    } catch {
      setState("failed");
    }
  }

  return <div className="copy-block">
    <code>{value}</code>
    <Button type="button" onClick={() => void copy()}>
      {state === "copied" ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
      {state === "copied" ? "已复制" : label}
    </Button>
    {state === "failed" ? <p role="alert">{failure}</p> : null}
  </div>;
}
