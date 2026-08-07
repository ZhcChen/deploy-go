import { ArrowLeft } from "lucide-react";
import type { Ref } from "react";
import { Link } from "react-router-dom";

export function BackLink({ to, parentLabel, linkRef }: { to: string; parentLabel: string; linkRef?: Ref<HTMLAnchorElement> }) {
  return <Link ref={linkRef} className="back-link" to={to} aria-label={`返回${parentLabel}`}>
    <span className="back-link__icon"><ArrowLeft aria-hidden="true" /></span>
    <span>返回{parentLabel}</span>
  </Link>;
}
