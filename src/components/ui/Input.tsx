interface InputProps {
  id?: string
  value: string
  placeholder?: string
  type?: 'text' | 'number'
  onChange: (value: string) => void
}

const base = 'input input-bordered w-full'

/** Replaces b-form-input. */
export default function Input({ id, value, placeholder, type = 'text', onChange }: InputProps) {
  return (
    <input
      id={id}
      type={type}
      className={base}
      value={value}
      placeholder={placeholder}
      onChange={(e) => onChange(e.target.value)}
    />
  )
}

