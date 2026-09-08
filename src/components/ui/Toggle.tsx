interface ToggleProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  describedBy?: string;
}

export function Toggle({ label, checked, onChange, describedBy = "preview-note" }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      className="switch"
      aria-label={label}
      aria-checked={checked}
      aria-describedby={describedBy}
      onClick={() => onChange(!checked)}
    >
      <span className="switch-thumb" aria-hidden="true" />
    </button>
  );
}
