import { cn } from '@/lib/utils'

interface SignalBadgeProps {
  side: string
  status?: string
}

export function SignalBadge({ side, status }: SignalBadgeProps) {
  return (
    <div className="flex items-center gap-1">
      <span
        className={cn(
          'px-1.5 py-0.5 rounded text-xs font-bold',
          side === 'YES' ? 'bg-success/20 text-success' : 'bg-destructive/20 text-destructive',
        )}
      >
        {side}
      </span>
      {status && status !== 'pending' && (
        <span className="px-1.5 py-0.5 rounded text-xs bg-muted text-muted-foreground">
          {status}
        </span>
      )}
    </div>
  )
}
