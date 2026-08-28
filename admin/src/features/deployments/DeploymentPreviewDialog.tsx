import { Server, X } from "lucide-react";
import { useEffect, useId, useRef } from "react";
import { Button } from "../../components/Button";
import type { ApplicationDeploymentPreviewResponse } from "../../api/generated";

interface DeploymentPreviewDialogProps {
  preview: ApplicationDeploymentPreviewResponse;
  onClose(): void;
}

export function DeploymentPreviewDialog({
  preview,
  onClose,
}: DeploymentPreviewDialogProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const restoreFocus = document.activeElement as HTMLElement | null;
    closeRef.current?.focus();
    return () => {
      if (restoreFocus?.isConnected) restoreFocus.focus();
    };
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
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
  }, [onClose]);

  return (
    <div className="modal-backdrop">
      <div
        ref={panelRef}
        className="deployment-preview-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
      >
        <header className="deployment-preview-dialog__header">
          <div>
            <h2 id={titleId}>部署预览</h2>
            <p>配置或目标变化会使当前 snapshot 失效，请核对后回到部署页发起部署。</p>
          </div>
          <Button
            ref={closeRef}
            className="deployment-preview-dialog__close"
            aria-label="关闭部署预览"
            onClick={onClose}
          >
            <X aria-hidden="true" />
          </Button>
        </header>
        <div className="deployment-preview-dialog__body">
          <div className="deployment-preview-dialog__content" aria-label="部署详情">
            <dl className="definition-grid">
              <div><dt>应用</dt><dd>{preview.applicationName}</dd></div>
              <div><dt>目标数量</dt><dd>{preview.targets.length}</dd></div>
              <div><dt>执行模式</dt><dd>{executionModeLabel(preview.executionMode)}</dd></div>
              {preview.executionMode === "two_stage" ? <>
                <div><dt>固定分支</dt><dd><code>{preview.deploymentBranch}</code></dd></div>
                <div><dt>Commit</dt><dd><code>{preview.resolvedCommitSha}</code></dd></div>
                <div><dt>发布版本</dt><dd><code>{preview.releaseVersion}</code></dd></div>
                <div><dt>模块</dt><dd>{preview.modules?.join(", ")}</dd></div>
              </> : preview.executionMode === "image" && preview.imageSpec ? <>
                <div><dt>模板</dt><dd><code>{preview.imageSpec.template}</code></dd></div>
                <div><dt>镜像</dt><dd><code>{preview.imageSpec.image}</code></dd></div>
                <div><dt>宿主端口</dt><dd><code>{preview.imageSpec.host_port}</code></dd></div>
                <div><dt>Env 文件</dt><dd>{preview.imageSpec.env_files.join(", ")}</dd></div>
              </> : null}
              <div><dt>Snapshot</dt><dd><code>{preview.snapshotHash}</code></dd></div>
              {preview.executionMode === "image" ? null : <div><dt>参数</dt><dd><code>{JSON.stringify(preview.parameters)}</code></dd></div>}
            </dl>
            <ul className="deployment-target-preview" aria-label="目标节点预览">
              {preview.targets.map((target) => (
                <li key={target.targetId}>
                  <div className="target-preview__identity">
                    <Server aria-hidden="true" />
                    <span><strong>{target.nodeName}</strong><code>{target.nodeId}</code></span>
                  </div>
                  <div className="target-preview__states">
                    <span className={`status-badge status-badge--${target.agentOnline ? "online" : "pending"}`}>{target.agentOnline ? "在线" : "离线，部署将等待节点恢复"}</span>
                    <span className={`status-badge status-badge--${target.envGateStatus === "failed" ? "disabled" : target.envGateStatus === "ready" || target.envGateStatus === "not_required" ? "online" : "pending"}`}>{envGateLabel(target.envGateStatus)}</span>
                  </div>
                  <code>{target.imageSpec?.image ?? target.scriptPath}</code>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}

function executionModeLabel(mode: string) {
  if (mode === "two_stage") return "两阶段（prepare + release）";
  if (mode === "image") return "镜像直连（固定 Make target）";
  return "单脚本";
}

function envGateLabel(status: string) {
  if (status === "ready") return "Env 已就绪";
  if (status === "failed") return "Env 同步失败";
  if (status === "not_required") return "无需 Env";
  return "Env 等待同步";
}
