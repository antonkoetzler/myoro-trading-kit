import { cn } from '@/lib/utils'

interface Column<T> {
  key: keyof T | string
  header: string
  render?: (row: T) => React.ReactNode
  className?: string
}

interface DataTableProps<T> {
  columns: Column<T>[]
  data: T[]
  selectedIdx?: number
  onSelect?: (idx: number) => void
  className?: string
  emptyMessage?: string
}

export function DataTable<T extends object>({
  columns,
  data,
  selectedIdx,
  onSelect,
  className,
  emptyMessage = 'No data',
}: DataTableProps<T>) {
  return (
    <div className={cn('overflow-auto', className)}>
      <table className="w-full text-sm">
        <thead className="sticky top-0 bg-card border-b border-border">
          <tr>
            {columns.map((col) => (
              <th
                key={col.key as string}
                className={cn('px-3 py-2 text-left text-xs font-medium text-muted-foreground', col.className)}
              >
                {col.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.length === 0 ? (
            <tr>
              <td colSpan={columns.length} className="px-3 py-8 text-center text-muted-foreground">
                {emptyMessage}
              </td>
            </tr>
          ) : (
            data.map((row, i) => (
              <tr
                key={i}
                onClick={() => onSelect?.(i)}
                className={cn(
                  'border-b border-border/50 hover:bg-muted/30 cursor-pointer transition-colors',
                  selectedIdx === i && 'bg-muted/50',
                )}
              >
                {columns.map((col) => (
                  <td key={col.key as string} className={cn('px-3 py-2', col.className)}>
                    {col.render
                      ? col.render(row)
                      : String((row as Record<string, unknown>)[col.key as string] ?? '')}
                  </td>
                ))}
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  )
}
