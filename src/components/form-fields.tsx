export function FieldLabel({ label, marker }: { label: string; marker: string }) {
  return <span className="field-label">{label}<small aria-hidden="true">{marker}</small></span>
}
