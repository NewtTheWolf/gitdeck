import type { ReactNode } from "react";

interface BadgeProps {
  icon?: ReactNode;
  children: ReactNode;
  className?: string;
}

/**
 * A small monochrome pill — hairline border, surface fill, no colored
 * background. Reserved for metadata chips (private / fork).
 */
export function Badge({ icon, children, className = "" }: BadgeProps) {
  return (
    <span
      className={
        "inline-flex items-center gap-1 rounded-full border border-border " +
        "bg-surface-2 px-2 py-0.5 text-[11px] font-medium uppercase tracking-wide " +
        "text-text-faint " +
        className
      }
    >
      {icon}
      {children}
    </span>
  );
}

export default Badge;
