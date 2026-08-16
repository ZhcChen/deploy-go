import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { NodeTelemetrySection } from "../features/nodes/NodeTelemetrySection";
import { server } from "./server";

function renderTelemetry() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={client}><NodeTelemetrySection nodeId="node-1" /></QueryClientProvider>);
}

const latest = {
  cpu_usage_ratio: { status: "available", value: 0.25 },
  memory_total_bytes: { status: "available", value: 1073741824 },
  memory_used_bytes: { status: "available", value: 536870912 },
  work_root_total_bytes: { status: "available", value: 2147483648 },
  work_root_used_bytes: { status: "available", value: 1073741824 },
  disk_read_bytes_per_second: { status: "available", value: 1024 },
  disk_write_bytes_per_second: { status: "available", value: 2048 },
  disk_busy_ratio: { status: "warming_up" },
  network_receive_bytes_per_second: { status: "available", value: 4096 },
  network_transmit_bytes_per_second: { status: "available", value: 1024 },
  gpu_status: "unsupported", gpu_reason: "hardware_not_present", gpus: [],
};

describe("节点遥测", () => {
  it("显示当前资源、预热状态和 24 小时趋势", async () => {
    server.use(http.get("/api/v1/nodes/node-1/telemetry", () => HttpResponse.json({ node_id:"node-1",connectivity:"online",capability:"supported",freshness:"fresh",captured_at:"2026-08-16T00:00:00Z",received_at:"2026-08-16T00:00:01Z",latest,history:[{received_at:"2026-08-16T00:00:00Z",cpu_usage_ratio:0.25,memory_used_bytes:536870912,work_root_used_bytes:1073741824,disk_read_bytes_per_second:1024,disk_write_bytes_per_second:2048,disk_busy_ratio:null,network_receive_bytes_per_second:4096,network_transmit_bytes_per_second:1024}]})));
    renderTelemetry();
    expect(await screen.findByText("25.0%")).toBeInTheDocument();
    expect(screen.getByText("采集预热中")).toBeInTheDocument();
    expect(screen.getByText(/当前 25.0%/)).toBeInTheDocument();
    expect(screen.getByText("数据正常")).toBeInTheDocument();
    expect(screen.getByText("未检测到 NVIDIA GPU")).toBeInTheDocument();
  });

  it("分别显示 stale 和旧协议不支持状态", async () => {
    server.use(http.get("/api/v1/nodes/node-1/telemetry", () => HttpResponse.json({node_id:"node-1",connectivity:"offline",capability:"unsupported",capability_reason:"protocol_v11",freshness:"empty",captured_at:null,received_at:null,latest:null,history:[]})));
    renderTelemetry();
    expect(await screen.findByText("当前 Agent 版本不支持遥测，请升级 Agent。")).toBeInTheDocument();
    expect(screen.getByText("离线")).toBeInTheDocument();
    expect(screen.getByText("暂无数据")).toBeInTheDocument();
  });

  it("请求失败后可重试并恢复", async () => {
    let calls=0;
    server.use(http.get("/api/v1/nodes/node-1/telemetry", () => { calls+=1; return calls===1 ? HttpResponse.json({code:"internal",message:"失败"},{status:500}) : HttpResponse.json({node_id:"node-1",connectivity:"online",capability:"supported",freshness:"empty",captured_at:null,received_at:null,latest:null,history:[]}); }));
    const user=userEvent.setup(); renderTelemetry();
    await user.click(await screen.findByRole("button",{name:/重试/}));
    expect(await screen.findByText("等待首个遥测样本。")).toBeInTheDocument();
    expect(calls).toBe(2);
  });
});
