import { Component, type ErrorInfo, type ReactNode } from "react";
import { AlertTriangle } from "lucide-react";

interface Props {
  children: ReactNode;
}

interface State {
  failed: boolean;
}

export class AppErrorBoundary extends Component<Props, State> {
  state: State = { failed: false };

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Web 管理端渲染失败", error, info.componentStack);
  }

  render() {
    if (this.state.failed) {
      return (
        <main className="fatal-state" role="alert">
          <AlertTriangle aria-hidden="true" />
          <h1>页面暂时无法显示</h1>
          <p>刷新页面后重试。若问题持续，请记录发生时间。</p>
          <button type="button" onClick={() => window.location.reload()}>
            刷新页面
          </button>
        </main>
      );
    }
    return this.props.children;
  }
}
