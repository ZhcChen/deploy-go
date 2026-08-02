import { Check, Copy } from "lucide-react";
import { useState } from "react";
import { Button } from "../../components/Button";

export interface ErrorNoticeValue {
  message: string;
  requestId?: string;
}

export function ApiErrorNotice({ error }: { error: ErrorNoticeValue }) {
  const [copied, setCopied] = useState(false);
  async function copyRequestId() {
    if (!error.requestId) return;
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(error.requestId);
      } else {
        const input = document.createElement("textarea");
        input.value = error.requestId;
        input.style.position = "fixed";
        input.style.opacity = "0";
        document.body.append(input);
        input.select();
        document.execCommand("copy");
        input.remove();
      }
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }
  return (
    <div className="notice notice--danger" role="alert">
      <strong>{error.message}</strong>
      {error.requestId ? <div className="request-id-row"><small>Request ID: {error.requestId}</small><Button aria-label="复制 Request ID" onClick={() => void copyRequestId()}>{copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}{copied ? "已复制" : "复制"}</Button></div> : null}
    </div>
  );
}
