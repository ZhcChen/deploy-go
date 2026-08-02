import { forwardRef, type ButtonHTMLAttributes } from "react";

type Tone = "default" | "primary" | "danger";

export const Button = forwardRef<
  HTMLButtonElement,
  ButtonHTMLAttributes<HTMLButtonElement> & { tone?: Tone }
>(function Button({ tone = "default", className = "", ...props }, ref) {
  return <button ref={ref} className={`button button--${tone} ${className}`.trim()} {...props} />;
});
