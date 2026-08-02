import type { ButtonHTMLAttributes } from "react";

type Tone = "default" | "primary" | "danger";

export function Button({ tone = "default", className = "", ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { tone?: Tone }) {
  return <button className={`button button--${tone} ${className}`.trim()} {...props} />;
}
