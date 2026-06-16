import { Check } from "lucide-react";

interface CheckboxProps {
  checked: boolean;
  onChange: () => void;
  "aria-label"?: string;
}

/**
 * Custom checkbox: a hairline-bordered square that fills with the accent and
 * shows a Lucide check when checked. Keyboard-accessible via a real button.
 */
export function Checkbox({
  checked,
  onChange,
  "aria-label": ariaLabel,
}: CheckboxProps) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      aria-label={ariaLabel}
      onClick={onChange}
      className={
        "flex size-[18px] shrink-0 items-center justify-center rounded-[--radius-sm] border " +
        "transition-[opacity,transform,color,background-color,border-color] duration-150 ease-out " +
        "outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 " +
        "focus-visible:ring-offset-canvas motion-reduce:transition-none " +
        (checked
          ? "border-accent bg-accent text-accent-fg"
          : "border-border-strong bg-surface-2 text-transparent hover:border-text-faint")
      }
    >
      <Check size={12} strokeWidth={2.5} aria-hidden />
    </button>
  );
}

export default Checkbox;
