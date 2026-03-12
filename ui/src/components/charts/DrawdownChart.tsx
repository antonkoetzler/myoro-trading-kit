import ReactECharts from 'echarts-for-react'

interface DrawdownChartProps {
  data: number[]
  height?: number
}

export function DrawdownChart({ data, height = 200 }: DrawdownChartProps) {
  const option = {
    backgroundColor: 'transparent',
    grid: { left: 60, right: 20, top: 20, bottom: 40 },
    xAxis: { type: 'category', data: data.map((_, i) => i), axisLine: { lineStyle: { color: '#334155' } }, axisLabel: { color: '#64748b' } },
    yAxis: { type: 'value', inverse: true, axisLabel: { formatter: (v: number) => `-${v.toFixed(1)}%`, color: '#64748b' }, splitLine: { lineStyle: { color: '#1e293b' } } },
    dataZoom: [{ type: 'inside' }],
    series: [{
      type: 'line',
      data,
      smooth: true,
      symbol: 'none',
      lineStyle: { color: '#ef4444', width: 2 },
      areaStyle: { color: 'rgba(239,68,68,0.4)' },
    }],
    tooltip: { trigger: 'axis', formatter: (params: { value: number }[]) => `-${params[0]?.value?.toFixed(2)}%`, backgroundColor: '#1e293b', borderColor: '#334155', textStyle: { color: '#e2e8f0' } },
  }
  return <ReactECharts option={option} style={{ height }} />
}
