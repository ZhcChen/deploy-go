import { Unplug } from "lucide-react";
import { Button } from "../../components/Button";
import { useAuth } from "../auth/AuthContext";

export function ServiceUnavailablePage() {
  const { retry, error } = useAuth();
  return (
    <main className="standalone-state" role="alert">
      <Unplug aria-hidden="true" />
      <h1>服务暂时不可用</h1>
      <p>控制服务没有响应，当前无法读取部署状态。</p>
      {error?.requestId ? <small className="request-id">Request ID: {error.requestId}</small> : null}
      <Button tone="primary" onClick={() => void retry()}>重新连接</Button>
    </main>
  );
}
