import type { PropsWithChildren } from "react";

export function AuthLayout({ children, context }: PropsWithChildren<{ context: string }>) {
  return (
    <main className="public-page">
      <section className="auth-panel">
        <div className="auth-brand">
          <span className="brand-mark">DG</span>
          <span><strong>Deploy Go</strong><small>{context}</small></span>
        </div>
        {children}
      </section>
    </main>
  );
}
