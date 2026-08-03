import { useEffect, useId, useRef, type RefObject } from "react";
import { Button } from "./Button";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  pending?: boolean;
  tone?: "primary" | "danger";
  fallbackFocusRef?: RefObject<HTMLElement | null>;
  onConfirm(): void;
  onClose(): void;
}

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel,
  pending = false,
  tone = "danger",
  fallbackFocusRef,
  onConfirm,
  onClose,
}: ConfirmDialogProps) {
  const titleId = useId();
  const messageId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    const restoreFocus = document.activeElement as HTMLElement | null;
    const fallbackFocus = fallbackFocusRef?.current;
    cancelRef.current?.focus();
    return () => {
      if (restoreFocus?.isConnected) restoreFocus.focus();
      else fallbackFocus?.focus();
    };
  }, [fallbackFocusRef, open]);

  useEffect(() => {
    if (open && pending) panelRef.current?.focus();
  }, [open, pending]);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !pending) {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = panelRef.current?.querySelectorAll<HTMLElement>(
        "button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex='-1'])",
      );
      if (!focusable?.length) {
        event.preventDefault();
        panelRef.current?.focus();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose, open, pending]);

  if (!open) return null;
  return (
    <div className="modal-backdrop">
      <div
        ref={panelRef}
        className="confirm-dialog"
        role="dialog"
        aria-modal="true"
        aria-busy={pending}
        aria-labelledby={titleId}
        aria-describedby={messageId}
        tabIndex={-1}
      >
        <h2 id={titleId}>{title}</h2>
        <p id={messageId}>{message}</p>
        <div className="confirm-dialog__actions">
          <Button ref={cancelRef} disabled={pending} onClick={onClose}>返回</Button>
          <Button tone={tone} disabled={pending} onClick={onConfirm}>
            {pending ? "正在处理..." : confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
