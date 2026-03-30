import { fetchPrice } from '../api';
import type { PricingResult } from '../types';
import { $, debounce } from '../utils';

interface CalcParams {
  s: number;
  k: number;
  t: number;
  r: number;
  sigma: number;
  type: string;
}

// Store current chart state for hover interaction
let chartState: {
  points: { x: number; y: number }[];
  low: number;
  high: number;
  minY: number;
  yRange: number;
  yPad: number;
  padding: { top: number; right: number; bottom: number; left: number };
  params: CalcParams;
  result: PricingResult;
  imageData: ImageData | null;
} | null = null;

export function initCalculator(): void {
  const debouncedCalc = debounce(calculatePrice, 150);
  const inputs = ['calc-s', 'calc-k', 'calc-t', 'calc-r', 'calc-sigma', 'calc-type'];
  inputs.forEach(id => {
    $(id).addEventListener('input', debouncedCalc);
    $(id).addEventListener('change', debouncedCalc);
  });

  // Hover crosshair on payoff chart
  const canvas = $('payoff-chart') as HTMLCanvasElement;
  canvas.addEventListener('mousemove', onChartHover);
  canvas.addEventListener('mouseleave', onChartLeave);

  calculatePrice();
}

function getParams(): CalcParams | null {
  const params = {
    s: parseFloat(($('calc-s') as HTMLInputElement).value),
    k: parseFloat(($('calc-k') as HTMLInputElement).value),
    t: parseFloat(($('calc-t') as HTMLInputElement).value),
    r: parseFloat(($('calc-r') as HTMLInputElement).value),
    sigma: parseFloat(($('calc-sigma') as HTMLInputElement).value),
    type: ($('calc-type') as HTMLSelectElement).value,
  };
  if (Object.values(params).some(v => typeof v === 'number' && isNaN(v))) return null;
  if (params.s <= 0 || params.k <= 0 || params.t <= 0 || params.sigma <= 0) return null;
  return params;
}

async function calculatePrice(): Promise<void> {
  const params = getParams();
  if (!params) return;

  try {
    const data = await fetchPrice(params);
    $('calc-result').style.display = 'block';
    $('calc-price').textContent = `$${data.price.toFixed(4)}`;

    // Greeks
    $('calc-greeks').innerHTML = [
      { label: 'Delta', value: data.delta.toFixed(4) },
      { label: 'Gamma', value: data.gamma.toFixed(4) },
      { label: 'Theta/day', value: (data.theta / 365).toFixed(4) },
      { label: 'Vega', value: data.vega.toFixed(4) },
      { label: 'Rho', value: data.rho.toFixed(4) },
    ].map(g => `
      <div class="greek-card">
        <div class="greek-label">${g.label}</div>
        <div class="greek-value">${g.value}</div>
      </div>
    `).join('');

    // Payoff stats
    const premium = data.price;
    const maxLoss = -premium;
    let maxProfit: number;
    let breakeven: number;

    if (params.type === 'call') {
      maxProfit = Infinity;
      breakeven = params.k + premium;
    } else {
      maxProfit = params.k - premium;
      breakeven = params.k - premium;
    }

    $('payoff-stats').style.display = 'grid';
    $('payoff-stats').innerHTML = `
      <div class="stat-card">
        <div class="stat-label">Max Profit</div>
        <div class="stat-value positive">${maxProfit === Infinity ? 'Unlimited' : '$' + maxProfit.toFixed(2)}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">Max Loss</div>
        <div class="stat-value negative">$${maxLoss.toFixed(2)}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">Breakeven</div>
        <div class="stat-value">$${breakeven.toFixed(2)}</div>
      </div>
    `;

    renderPayoff(params, data);
  } catch (_) {
    // Silent during typing
  }
}

function renderPayoff(params: CalcParams, result: PricingResult): void {
  $('payoff-card').style.display = 'block';

  const canvas = $('payoff-chart') as HTMLCanvasElement;
  const ctx = canvas.getContext('2d')!;

  const rect = canvas.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  canvas.width = rect.width * dpr;
  canvas.height = rect.height * dpr;
  ctx.scale(dpr, dpr);

  const w = rect.width;
  const h = rect.height;
  const padding = { top: 24, right: 30, bottom: 44, left: 60 };
  const plotW = w - padding.left - padding.right;
  const plotH = h - padding.top - padding.bottom;

  const premium = result.price;
  const low = params.k * 0.7;
  const high = params.k * 1.3;
  const steps = 200;
  const points: { x: number; y: number }[] = [];

  for (let i = 0; i <= steps; i++) {
    const stockPrice = low + (high - low) * (i / steps);
    const payoff = params.type === 'call'
      ? Math.max(stockPrice - params.k, 0) - premium
      : Math.max(params.k - stockPrice, 0) - premium;
    points.push({ x: stockPrice, y: payoff });
  }

  const minY = Math.min(...points.map(p => p.y));
  const maxY = Math.max(...points.map(p => p.y));
  const yRange = maxY - minY || 1;
  const yPad = yRange * 0.15;

  // Save state for hover (imageData set after drawing)
  chartState = { points, low, high, minY, yRange, yPad, padding, params, result, imageData: null };

  const scaleX = (x: number) => padding.left + ((x - low) / (high - low)) * plotW;
  const scaleY = (y: number) => padding.top + plotH - ((y - (minY - yPad)) / (yRange + 2 * yPad)) * plotH;

  ctx.clearRect(0, 0, w, h);

  // Subtle horizontal grid lines
  ctx.strokeStyle = '#EDE5D8';
  ctx.lineWidth = 0.5;
  const yTicks = 5;
  for (let i = 0; i <= yTicks; i++) {
    const py = padding.top + (plotH * i) / yTicks;
    ctx.beginPath();
    ctx.moveTo(padding.left, py);
    ctx.lineTo(w - padding.right, py);
    ctx.stroke();
  }

  // Zero line (thicker)
  const zeroY = scaleY(0);
  ctx.strokeStyle = '#D5C9B8';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(padding.left, zeroY);
  ctx.lineTo(w - padding.right, zeroY);
  ctx.stroke();

  // Profit fill
  ctx.beginPath();
  let inProfit = false;
  for (let i = 0; i <= steps; i++) {
    const px = scaleX(points[i].x);
    const py = scaleY(points[i].y);
    if (points[i].y > 0) {
      if (!inProfit) { ctx.moveTo(px, zeroY); inProfit = true; }
      ctx.lineTo(px, py);
    } else if (inProfit) {
      ctx.lineTo(px, zeroY);
      inProfit = false;
    }
  }
  if (inProfit) ctx.lineTo(scaleX(points[steps].x), zeroY);
  ctx.closePath();
  ctx.fillStyle = 'rgba(139, 168, 138, 0.18)';
  ctx.fill();

  // Loss fill
  ctx.beginPath();
  let inLoss = false;
  for (let i = 0; i <= steps; i++) {
    const px = scaleX(points[i].x);
    const py = scaleY(points[i].y);
    if (points[i].y < 0) {
      if (!inLoss) { ctx.moveTo(px, zeroY); inLoss = true; }
      ctx.lineTo(px, py);
    } else if (inLoss) {
      ctx.lineTo(px, zeroY);
      inLoss = false;
    }
  }
  if (inLoss) ctx.lineTo(scaleX(points[steps].x), zeroY);
  ctx.closePath();
  ctx.fillStyle = 'rgba(196, 91, 91, 0.12)';
  ctx.fill();

  // Strike line
  const strikeX = scaleX(params.k);
  ctx.setLineDash([5, 5]);
  ctx.strokeStyle = '#AAA';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(strikeX, padding.top);
  ctx.lineTo(strikeX, h - padding.bottom);
  ctx.stroke();
  ctx.setLineDash([]);

  // Spot line
  const spotX = scaleX(params.s);
  ctx.setLineDash([3, 3]);
  ctx.strokeStyle = '#D97757';
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(spotX, padding.top);
  ctx.lineTo(spotX, h - padding.bottom);
  ctx.stroke();
  ctx.setLineDash([]);

  // Payoff line
  ctx.beginPath();
  ctx.strokeStyle = '#1F1F1F';
  ctx.lineWidth = 2.5;
  ctx.lineJoin = 'round';
  for (let i = 0; i <= steps; i++) {
    const px = scaleX(points[i].x);
    const py = scaleY(points[i].y);
    if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
  }
  ctx.stroke();

  // Axis labels
  ctx.fillStyle = '#888';
  ctx.font = '11px Inter, sans-serif';

  // X axis
  ctx.textAlign = 'center';
  const xTicks = 6;
  for (let i = 0; i <= xTicks; i++) {
    const val = low + (high - low) * (i / xTicks);
    ctx.fillText(`$${val.toFixed(0)}`, scaleX(val), h - padding.bottom + 18);
  }

  // Y axis
  ctx.textAlign = 'right';
  for (let i = 0; i <= yTicks; i++) {
    const val = (minY - yPad) + (yRange + 2 * yPad) * (i / yTicks);
    const py = padding.top + plotH - (plotH * i) / yTicks;
    ctx.fillText(`$${val.toFixed(0)}`, padding.left - 8, py + 4);
  }

  // Strike label
  ctx.fillStyle = '#AAA';
  ctx.textAlign = 'center';
  ctx.font = '10px Inter, sans-serif';
  ctx.fillText(`Strike $${params.k}`, strikeX, h - padding.bottom + 34);

  // Spot label
  ctx.fillStyle = '#D97757';
  ctx.fillText(`Spot $${params.s}`, spotX, padding.top - 8);

  // Save rendered chart for hover overlay
  if (chartState) {
    chartState.imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
  }
}

// --- Hover crosshair ---

function onChartHover(e: MouseEvent): void {
  if (!chartState || !chartState.imageData) return;

  const canvas = e.target as HTMLCanvasElement;
  const ctx = canvas.getContext('2d')!;
  const rect = canvas.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const mx = e.clientX - rect.left;
  const my = e.clientY - rect.top;

  const { low, high, minY, yRange, yPad, padding, params, result } = chartState;
  const w = rect.width;
  const h = rect.height;
  const plotW = w - padding.left - padding.right;
  const plotH = h - padding.top - padding.bottom;

  // Check bounds
  if (mx < padding.left || mx > w - padding.right || my < padding.top || my > h - padding.bottom) {
    onChartLeave();
    return;
  }

  // Map mouse X to stock price
  const stockPrice = low + ((mx - padding.left) / plotW) * (high - low);
  const payoff = params.type === 'call'
    ? Math.max(stockPrice - params.k, 0) - result.price
    : Math.max(params.k - stockPrice, 0) - result.price;

  const scaleY = (y: number) => padding.top + plotH - ((y - (minY - yPad)) / (yRange + 2 * yPad)) * plotH;
  const py = scaleY(payoff);

  // Restore clean chart, then draw crosshair on top
  ctx.putImageData(chartState.imageData, 0, 0);
  ctx.save();
  ctx.scale(dpr, dpr);

  // Vertical crosshair line
  ctx.setLineDash([2, 2]);
  ctx.strokeStyle = '#1F1F1F';
  ctx.lineWidth = 0.75;
  ctx.beginPath();
  ctx.moveTo(mx, padding.top);
  ctx.lineTo(mx, h - padding.bottom);
  ctx.stroke();
  ctx.setLineDash([]);

  // Dot on the payoff line
  ctx.beginPath();
  ctx.arc(mx, py, 4, 0, Math.PI * 2);
  ctx.fillStyle = payoff >= 0 ? '#8BA88A' : '#C45B5B';
  ctx.fill();
  ctx.strokeStyle = '#fff';
  ctx.lineWidth = 2;
  ctx.stroke();

  // Tooltip pill
  const label = `$${stockPrice.toFixed(1)}  →  ${payoff >= 0 ? '+' : ''}$${payoff.toFixed(2)}`;
  ctx.font = '600 12px Inter, sans-serif';
  const textW = ctx.measureText(label).width;
  const pillW = textW + 20;
  const pillH = 28;
  let pillX = mx - pillW / 2;
  const pillY = py - pillH - 12;

  pillX = Math.max(padding.left, Math.min(pillX, w - padding.right - pillW));

  ctx.fillStyle = '#1F1F1F';
  ctx.beginPath();
  ctx.roundRect(pillX, pillY, pillW, pillH, 6);
  ctx.fill();

  ctx.fillStyle = '#FAF6F0';
  ctx.textAlign = 'center';
  ctx.fillText(label, pillX + pillW / 2, pillY + pillH / 2 + 4);

  ctx.restore();
}

function onChartLeave(): void {
  if (!chartState || !chartState.imageData) return;
  const canvas = $('payoff-chart') as HTMLCanvasElement;
  const ctx = canvas.getContext('2d')!;
  ctx.putImageData(chartState.imageData, 0, 0);
}
