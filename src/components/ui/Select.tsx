import Dropdown from './Dropdown';
import {t} from '@/i18n';
export interface Option {
  name: string
  value: string | boolean
}

interface SelectProps {
  id?: string
  value: string
  options: Option[]
  onChange: (value: string) => void
}

const base = 'select select-bordered w-full'

/** Replaces b-form-select. */
export default function Select({ id, value, options, onChange }: SelectProps) {
  return (
    <Dropdown id={id} className={base} value={value} onChange={(e) => onChange(e.target.value)}>
      {/* Keyed by name as well as value: the size list legitimately repeats
          widths (1440p and 1600p are both 2560 wide), and keying on value alone
          makes React drop one of the pair. */}
      {options.map((o) => (
        <option key={`${t(o.name)}-${o.value}`} value={String(o.value)}>
          {t(o.name)}
        </option>
      ))}
    </Dropdown>
  )
}



