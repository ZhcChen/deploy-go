import { AlertCircle, Inbox, LoaderCircle } from "lucide-react";

type PageStateKind = "loading" | "empty" | "error";

const content = {
  loading: { title: "正在加载", detail: "正在获取最新数据。", icon: LoaderCircle },
  empty: { title: "暂无数据", detail: "这里还没有可显示的记录。", icon: Inbox },
  error: { title: "加载失败", detail: "数据暂时无法获取，请稍后重试。", icon: AlertCircle },
} satisfies Record<PageStateKind, { title: string; detail: string; icon: typeof Inbox }>;

export function PageState({ kind }: { kind: PageStateKind }) {
  const state = content[kind];
  const Icon = state.icon;
  return (
    <section className={`page-state page-state--${kind}`} aria-live="polite">
      <Icon aria-hidden="true" />
      <h2>{state.title}</h2>
      <p>{state.detail}</p>
    </section>
  );
}
