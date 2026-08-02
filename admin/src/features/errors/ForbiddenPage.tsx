import { ShieldX } from "lucide-react";
import { Link } from "react-router-dom";

export function ForbiddenPage() {
  return (
    <main className="standalone-state">
      <ShieldX aria-hidden="true" />
      <h1>没有访问权限</h1>
      <p>当前账号不能访问系统管理功能。</p>
      <Link className="button button--primary" to="/overview">返回概览</Link>
    </main>
  );
}
