import { cn } from '@/lib/utils'

interface MetricCardProps {
  label: string
  value: string | number
  className?: string
  positive?: boolean
}

export function MetricCard({ label, value, className, positive }: MetricCardProps) {
  const numVal = typeof value === 'number' ? value : parseFloat(String(value))
  const colorClass =
    positive !== undefined
      ? positive
        ? 'text-success'
        : 'text-destructive'
      : !isNaN(numVal)
      ? numVal >= 0
        ? 'text-success'
        : 'text-destructive'
      : 'text-foreground'

  return (
    <div className={cn('bg-card rounded border border-border p-3', className)}>
      <div className="text-xs text-muted-foreground mb-1">{label}</div>
      <div className={cn('text-lg font-semibold tabular-nums', colorClass)}>
        {typeof value === 'number' ? value.toFixed(4) : value}
      </div>
    </div>
  )
}
