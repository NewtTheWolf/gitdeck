interface SkeletonProps {
  className?: string;
}

/**
 * A subtle loading placeholder: a surface-2 block at low opacity with a slow
 * opacity pulse (no shimmer / rainbow). Respects prefers-reduced-motion.
 */
export function Skeleton({ className = "" }: SkeletonProps) {
  return (
    <div
      aria-hidden
      className={
        "animate-skeleton-pulse rounded-[--radius-sm] bg-surface-2 " +
        "motion-reduce:animate-none " +
        className
      }
    />
  );
}

export default Skeleton;
