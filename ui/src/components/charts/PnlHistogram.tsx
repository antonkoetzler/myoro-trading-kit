import ReactECharts from 'echarts-for-react'

interface PnlHistogramProps {
  buckets: [number, number][]
  height?: number
}

export function PnlHistogram({ buckets, height = 200 }: PnlHistogramProps) {
  const option = {
    backgroundColor: 'transparent',
    grid: { left: 60, right: 20, top: 20, bottom: 40 },
    xAxis: { type: 'category', data: buckets.map(([mid]) => mid.toFixed(2)), axisLabel: { color: '#64748b', rotate: 30 }, axisLine: { lineStyle: { color: '#334155' } } },
    yAxis: { type: 'value', axisLabel: { color: '#64748b' }, splitLine: { lineStyle: { color: '#1e293b' } } },
    series: [{
      type: 'bar',
      data: buckets.map(([mid, count]) => ({
        value: count,
        itemStyle: { color: mid >= 0 ? '#22c55e' : '#ef4444' },
      })),
    }],
    tooltip: { trigger: 'axis', backgroundColor: '#1e293b', borderColor: '#334155', textStyle: { color: '#e2e8f0' } },
  }
  return <ReactECharts option={option} style={{ height }} />
}
