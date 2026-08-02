import { ArrowLeft } from "lucide-react";
import { Link } from "react-router-dom";

export function NotFoundPage() {
  return (
    <main className="not-found">
      <span className="not-found-code">404</span>
      <h1>页面不存在</h1>
      <p>该地址无对应页面，或资源已经被移除。</p>
      <Link className="button button--primary" to="/overview">
        <ArrowLeft aria-hidden="true" />返回概览
      </Link>
    </main>
  );
}
