import { Info } from 'lucide-react'
import { useId } from 'react'

/** The small ⓘ affordance that explains what an option means. */
export function Tooltip({ label, text }: { label: string; text: string }) {
  const id = useId()
  return <span className="tooltip">
    <button type="button" className="info-button" aria-label={label} aria-describedby={id}
      onClick={event => event.stopPropagation()}><Info size={10} aria-hidden="true" /></button>
    <span className="tooltip-bubble" role="tooltip" id={id}>{text}</span>
  </span>
}
