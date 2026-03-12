import ReactECharts from 'echarts-for-react'

interface EquityCurveProps {
  data: number[]
  height?: number
}

export function EquityCurve({ data, height = 200 }: EquityCurveProps) {
  const option = {
    backgroundColor: 'transparent',
    grid: { left: 60, right: 20, top: 20, bottom: 40 },
    xAxis: { type: 'category', data: data.map((_, i) => i), axisLine: { lineStyle: { color: '#334155' } }, axisLabel: { color: '#64748b' } },
    yAxis: { type: 'value', axisLine: { lineStyle: { color: '#334155' } }, axisLabel: { color: '#64748b' }, splitLine: { lineStyle: { color: '#1e293b' } } },
    dataZoom: [{ type: 'inside' }, { type: 'slider', height: 20 }],
    series: [{
      type: 'line',
      data,
      smooth: true,
      symbol: 'none',
      lineStyle: { color: '#3b82f6', width: 2 },
      areaStyle: { color: { type: 'linear', x: 0, y: 0, x2: 0, y2: 1, colorStops: [{ offset: 0, color: 'rgba(59,130,246,0.3)' }, { offset: 1, color: 'rgba(59,130,246,0)' }] } },
    }],
    tooltip: { trigger: 'axis', backgroundColor: '#1e293b', borderColor: '#334155', textStyle: { color: '#e2e8f0' } },
  }
  return <ReactECharts option={option} style={{ height }} />
}
