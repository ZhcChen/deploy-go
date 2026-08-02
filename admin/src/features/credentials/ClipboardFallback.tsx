import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { Button } from "../../components/Button";

export function ClipboardFallback({ value }: { value: string }) {
  const [state, setState] = useState<"idle" | "copied" | "failed">("idle");

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setState("copied");
    } catch {
      setState("failed");
    }
  }

  return (
    <div className="copy-block">
      <code>{value}</code>
      <Button type="button" onClick={() => void copy()}>
        {state === "copied" ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
        {state === "copied" ? "已复制" : "复制公钥"}
      </Button>
      {state === "failed" ? (
        <p role="alert">自动复制失败，请选中上方完整公钥后手动复制。</p>
      ) : null}
    </div>
  );
}
