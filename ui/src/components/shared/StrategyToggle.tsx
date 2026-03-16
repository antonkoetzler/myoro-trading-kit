interface Props {
  enabled: boolean
  onToggle: (next: boolean) => void
}

export function StrategyToggle({ enabled, onToggle }: Props) {
  return (
    <button
      role="switch"
      aria-checked={enabled}
      onClick={() => onToggle(!enabled)}
      className={[
        'relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer items-center rounded-full',
        'border-2 border-transparent outline-none transition-colors duration-200',
        'focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1',
        'focus-visible:ring-offset-background',
        enabled ? 'bg-success' : 'bg-muted hover:bg-muted-foreground/20',
      ].join(' ')}
    >
      <span
        className={[
          'pointer-events-none inline-block h-3.5 w-3.5 rounded-full bg-white shadow-sm',
          'transition-transform duration-200',
          enabled ? 'translate-x-4' : 'translate-x-0.5',
        ].join(' ')}
      />
    </button>
  )
}
