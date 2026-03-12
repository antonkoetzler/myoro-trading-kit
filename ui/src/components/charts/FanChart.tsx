import ReactECharts from 'echarts-for-react'

interface FanChartProps {
  paths: number[][] | null
  height?: number
}

export function FanChart({ paths, height = 200 }: FanChartProps) {
  if (!paths || paths.length === 0) {
    return (
      <div style={{ height }} className="flex items-center justify-center text-muted-foreground text-sm">
        No Monte Carlo paths
      </div>
    )
  }

  const len = paths[0].length
  const median = Array.from({ length: len }, (_, i) => {
    const vals = paths.map((p) => p[i]).sort((a, b) => a - b)
    return vals[Math.floor(vals.length / 2)]
  })

  const series = [
    ...paths.slice(0, Math.min(paths.length, 100)).map((path) => ({
      type: 'line',
      data: path,
      symbol: 'none',
      lineStyle: { color: 'rgba(59,130,246,0.08)', width: 1 },
      silent: true,
    })),
    {
      type: 'line',
      data: median,
      symbol: 'none',
      lineStyle: { color: '#3b82f6', width: 2 },
      name: 'Median',
    },
  ]

  const option = {
    backgroundColor: 'transparent',
    grid: { left: 60, right: 20, top: 20, bottom: 40 },
    xAxis: { type: 'category', data: median.map((_, i) => i), axisLabel: { color: '#64748b' }, axisLine: { lineStyle: { color: '#334155' } } },
    yAxis: { type: 'value', axisLabel: { color: '#64748b' }, splitLine: { lineStyle: { color: '#1e293b' } } },
    series,
    legend: { show: false },
  }
  return <ReactECharts option={option} style={{ height }} />
}
