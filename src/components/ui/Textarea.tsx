interface TextareaProps {
  id?: string
  value: string
  placeholder?: string
  rows?: number
  onChange: (value: string) => void
}

const base = 'textarea textarea-bordered w-full font-mono'

/** Replaces b-form-textarea. */
export default function Textarea({ id, value, placeholder, rows = 2, onChange }: TextareaProps) {
  return (
    <textarea
      id={id}
      rows={rows}
      className={base}
      value={value}
      placeholder={placeholder}
      onChange={(e) => onChange(e.target.value)}
    />
  )
}

