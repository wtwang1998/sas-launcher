/**
 * A Windows-style on/off switch. A real button keeps it keyboard operable and
 * lets the accessible name come from the row label through `aria-labelledby`.
 */
export function Switch({
  labelledBy,
  checked,
  disabled = false,
  onChange,
}: {
  labelledBy: string
  checked: boolean
  disabled?: boolean
  onChange: (next: boolean) => void
}) {
  return (
    <button
      type="button"
      role="switch"
      className="switch"
      aria-checked={checked}
      aria-labelledby={labelledBy}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    >
      <span className="switch-thumb" aria-hidden="true" />
    </button>
  )
}
