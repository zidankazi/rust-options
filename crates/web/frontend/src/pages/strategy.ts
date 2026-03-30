import { fetchPrice } from '../api';
import type { PricingResult } from '../types';
import { $ } from '../utils';

interface Leg {
  type: 'call' | 'put';
  strike: number;
  premium: number;
  direction: 1 | -1;
  quantity: number;
  greeks: PricingResult | null;
}

let legs: Leg[] = [];
let spotPrice = 100;
let vol = 0.20;
let riskFreeRate = 0.05;
let timeToExpiry = 0.25;

// --- Presets ---

interface Preset {
  name: string;
  description: string;
  build: (s: number) => Leg[];
}

const PRESETS: Preset[] = [
  {
    name: 'Long Call',
    description: 'Bullish, unlimited upside',
    build: (s) => [
      { type: 'call', strike: round5(s), premium: 0, direction: 1, quantity: 1, greeks: null },
    ],
  },
  {
    name: 'Long Put',
    description: 'Bearish, profit if stock drops',
    build: (s) => [
      { type: 'put', strike: round5(s), premium: 0, direction: 1, quantity: 1, greeks: null },
    ],
  },
  {
    name: 'Bull Call Spread',
    description: 'Moderately bullish, capped risk',
    build: (s) => [
      { type: 'call', strike: round5(s), premium: 0, direction: 1, quantity: 1, greeks: null },
      { type: 'call', strike: round5(s) + 10, premium: 0, direction: -1, quantity: 1, greeks: null },
    ],
  },
  {
    name: 'Bear Put Spread',
    description: 'Moderately bearish, capped risk',
    build: (s) => [
      { type: 'put', strike: round5(s), premium: 0, direction: 1, quantity: 1, greeks: null },
      { type: 'put', strike: round5(s) - 10, premium: 0, direction: -1, quantity: 1, greeks: null },
    ],
  },
  {
    name: 'Straddle',
    description: 'Bet on big move, either direction',
    build: (s) => [
      { type: 'call', strike: round5(s), premium: 0, direction: 1, quantity: 1, greeks: null },
      { type: 'put', strike: round5(s), premium: 0, direction: 1, quantity: 1, greeks: null },
    ],
  },
  {
    name: 'Strangle',
    description: 'Cheaper straddle, wider strikes',
    build: (s) => [
      { type: 'call', strike: round5(s) + 5, premium: 0, direction: 1, quantity: 1, greeks: null },
      { type: 'put', strike: round5(s) - 5, premium: 0, direction: 1, quantity: 1, greeks: null },
    ],
  },
  {
    name: 'Iron Condor',
    description: 'Bet stock stays flat, defined risk',
    build: (s) => {
      const atm = round5(s);
      return [
        { type: 'put', strike: atm - 15, premium: 0, direction: 1, quantity: 1, greeks: null },
        { type: 'put', strike: atm - 5, premium: 0, direction: -1, quantity: 1, greeks: null },
        { type: 'call', strike: atm + 5, premium: 0, direction: -1, quantity: 1, greeks: null },
        { type: 'call', strike: atm + 15, premium: 0, direction: 1, quantity: 1, greeks: null },
      ];
    },
  },
  {
    name: 'Butterfly',
    description: 'Bet stock stays near a price',
    build: (s) => {
      const atm = round5(s);
      return [
        { type: 'call', strike: atm - 10, premium: 0, direction: 1, quantity: 1, greeks: null },
        { type: 'call', strike: atm, premium: 0, direction: -1, quantity: 2, greeks: null },
        { type: 'call', strike: atm + 10, premium: 0, direction: 1, quantity: 1, greeks: null },
      ];
    },
  },
];

function round5(n: number): number {
  return Math.round(n / 5) * 5;
}

// --- Init ---

export function initStrategy(): void {
  renderPage();
}

function renderPage(): void {
  const page = $('page-strategy');
  page.innerHTML = `
    <h1 class="page-title">Strategy Builder</h1>
    <p class="page-subtitle">Build multi-leg options strategies and visualize combined payoff</p>

    <div class="card" style="margin-bottom: 20px;">
      <div class="card-title" style="margin-bottom: 16px;">Parameters</div>
      <div class="calculator-grid">
        <div class="input-group">
          <label>Spot Price</label>
          <input type="number" id="strat-spot" value="${spotPrice}" step="1">
        </div>
        <div class="input-group">
          <label>Volatility (σ)</label>
          <input type="number" id="strat-vol" value="${vol}" step="0.01">
        </div>
        <div class="input-group">
          <label>Time to Expiry (years)</label>
          <input type="number" id="strat-time" value="${timeToExpiry}" step="0.01">
        </div>
        <div class="input-group">
          <label>Risk-Free Rate</label>
          <input type="number" id="strat-rate" value="${riskFreeRate}" step="0.01">
        </div>
      </div>
    </div>

    <div class="card" style="margin-bottom: 20px;">
      <div class="card-title" style="margin-bottom: 16px;">Presets</div>
      <div class="preset-grid">
        ${PRESETS.map((p, i) => `
          <button class="preset-btn" data-idx="${i}">
            <span class="preset-name">${p.name}</span>
            <span class="preset-desc">${p.description}</span>
          </button>
        `).join('')}
      </div>
    </div>

    <div class="card" style="margin-bottom: 20px;">
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;">
        <div class="card-title">Legs</div>
        <button class="btn btn-secondary" id="add-leg-btn" style="font-size: 13px; padding: 6px 14px;">+ Add Leg</button>
      </div>
      <div id="legs-container"></div>
    </div>

    <div id="strat-stats" class="stats-row" style="display: none;"></div>

    <div class="card" id="strat-chart-card" style="display: none;">
      <div class="card-header">
        <div class="card-title">Combined Payoff at Expiry</div>
      </div>
      <canvas id="strat-chart" height="320"></canvas>
    </div>
  `;

  // Event listeners
  document.querySelectorAll('.preset-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const idx = parseInt((btn as HTMLElement).dataset.idx!);
      readParams();
      legs = PRESETS[idx].build(spotPrice);
      priceAllLegs();
    });
  });

  $('add-leg-btn').addEventListener('click', () => {
    readParams();
    legs.push({ type: 'call', strike: round5(spotPrice), premium: 0, direction: 1, quantity: 1, greeks: null });
    priceAllLegs();
  });

  ['strat-spot', 'strat-vol', 'strat-time', 'strat-rate'].forEach(id => {
    $(id).addEventListener('change', () => {
      readParams();
      if (legs.length > 0) priceAllLegs();
    });
  });
}

function readParams(): void {
  spotPrice = parseFloat(($('strat-spot') as HTMLInputElement).value) || 100;
  vol = parseFloat(($('strat-vol') as HTMLInputElement).value) || 0.20;
  timeToExpiry = parseFloat(($('strat-time') as HTMLInputElement).value) || 0.25;
  riskFreeRate = parseFloat(($('strat-rate') as HTMLInputElement).value) || 0.05;
}

// --- Price all legs via the API ---

async function priceAllLegs(): Promise<void> {
  const promises = legs.map(async (leg) => {
    try {
      const result = await fetchPrice({
        s: spotPrice,
        k: leg.strike,
        t: timeToExpiry,
        r: riskFreeRate,
        sigma: vol,
        type: leg.type,
      });
      leg.premium = result.price;
      leg.greeks = result;
    } catch (_) {
      leg.premium = 0;
      leg.greeks = null;
    }
  });

  await Promise.all(promises);
  renderLegs();
  renderChart();
}

// --- Render legs table ---

function renderLegs(): void {
  const container = $('legs-container');

  if (legs.length === 0) {
    container.innerHTML = '<p style="color: var(--text-muted); font-size: 13px;">No legs yet. Pick a preset or add one manually.</p>';
    $('strat-stats').style.display = 'none';
    $('strat-chart-card').style.display = 'none';
    return;
  }

  container.innerHTML = `
    <table style="width: 100%;">
      <thead>
        <tr>
          <th>Direction</th>
          <th>Type</th>
          <th>Strike</th>
          <th>Premium</th>
          <th>Qty</th>
          <th>Delta</th>
          <th>Theta/day</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        ${legs.map((leg, i) => {
          const dir = leg.direction === 1 ? 'BUY' : 'SELL';
          const dirClass = leg.direction === 1 ? 'positive' : 'negative';
          const delta = leg.greeks ? (leg.direction * leg.quantity * leg.greeks.delta).toFixed(3) : '—';
          const theta = leg.greeks ? (leg.direction * leg.quantity * leg.greeks.theta / 365).toFixed(3) : '—';
          return `
            <tr>
              <td><button class="dir-toggle ${dirClass}" data-idx="${i}">${dir}</button></td>
              <td><button class="type-toggle" data-idx="${i}">${leg.type.toUpperCase()}</button></td>
              <td><input type="number" class="leg-strike" data-idx="${i}" value="${leg.strike}" step="5" style="width: 80px;"></td>
              <td>$${leg.premium.toFixed(2)}</td>
              <td><input type="number" class="leg-qty" data-idx="${i}" value="${leg.quantity}" min="1" step="1" style="width: 50px;"></td>
              <td>${delta}</td>
              <td style="color: var(--red);">${theta}</td>
              <td><button class="remove-leg" data-idx="${i}">✕</button></td>
            </tr>
          `;
        }).join('')}
      </tbody>
    </table>
    <div class="net-cost" style="margin-top: 12px; font-size: 14px; font-weight: 600;">
      Net cost: $${netCost().toFixed(2)} per share
      <span style="color: var(--text-muted); font-weight: 400; margin-left: 8px;">
        (${netCost() > 0 ? 'you pay' : 'you receive'})
      </span>
    </div>
  `;

  // Wire up leg controls
  container.querySelectorAll('.dir-toggle').forEach(btn => {
    btn.addEventListener('click', () => {
      const i = parseInt((btn as HTMLElement).dataset.idx!);
      legs[i].direction = legs[i].direction === 1 ? -1 : 1;
      renderLegs();
      renderChart();
    });
  });

  container.querySelectorAll('.type-toggle').forEach(btn => {
    btn.addEventListener('click', () => {
      const i = parseInt((btn as HTMLElement).dataset.idx!);
      legs[i].type = legs[i].type === 'call' ? 'put' : 'call';
      priceAllLegs();
    });
  });

  container.querySelectorAll('.leg-strike').forEach(input => {
    input.addEventListener('change', () => {
      const i = parseInt((input as HTMLElement).dataset.idx!);
      legs[i].strike = parseFloat((input as HTMLInputElement).value);
      priceAllLegs();
    });
  });

  container.querySelectorAll('.leg-qty').forEach(input => {
    input.addEventListener('change', () => {
      const i = parseInt((input as HTMLElement).dataset.idx!);
      legs[i].quantity = parseInt((input as HTMLInputElement).value) || 1;
      renderLegs();
      renderChart();
    });
  });

  container.querySelectorAll('.remove-leg').forEach(btn => {
    btn.addEventListener('click', () => {
      const i = parseInt((btn as HTMLElement).dataset.idx!);
      legs.splice(i, 1);
      if (legs.length === 0) {
        renderLegs();
      } else {
        renderLegs();
        renderChart();
      }
    });
  });
}

// --- Payoff math ---

function legPayoff(leg: Leg, price: number): number {
  const intrinsic = leg.type === 'call'
    ? Math.max(price - leg.strike, 0)
    : Math.max(leg.strike - price, 0);
  return leg.direction * leg.quantity * (intrinsic - leg.premium);
}

function strategyPayoff(price: number): number {
  return legs.reduce((sum, leg) => sum + legPayoff(leg, price), 0);
}

function netCost(): number {
  return legs.reduce((sum, leg) => sum + leg.direction * leg.quantity * leg.premium, 0);
}

// --- Chart ---

function renderChart(): void {
  if (legs.length === 0) return;

  $('strat-chart-card').style.display = 'block';

  // Compute stats
  const allStrikes = legs.map(l => l.strike);
  const minStrike = Math.min(...allStrikes);
  const maxStrike = Math.max(...allStrikes);
  const range = Math.max(maxStrike - minStrike, 20);
  const low = minStrike - range * 0.8;
  const high = maxStrike + range * 0.8;

  const steps = 300;
  const points: { x: number; y: number }[] = [];
  for (let i = 0; i <= steps; i++) {
    const price = low + (high - low) * (i / steps);
    points.push({ x: price, y: strategyPayoff(price) });
  }

  const payoffs = points.map(p => p.y);
  const maxProfit = Math.max(...payoffs);
  const maxLoss = Math.min(...payoffs);

  // Breakevens
  const breakevens: number[] = [];
  for (let i = 1; i < points.length; i++) {
    if ((points[i - 1].y < 0 && points[i].y >= 0) || (points[i - 1].y >= 0 && points[i].y < 0)) {
      // Linear interpolation
      const x0 = points[i - 1].x, y0 = points[i - 1].y;
      const x1 = points[i].x, y1 = points[i].y;
      breakevens.push(x0 + (0 - y0) * (x1 - x0) / (y1 - y0));
    }
  }

  // Net Greeks
  const netDelta = legs.reduce((s, l) => s + (l.greeks ? l.direction * l.quantity * l.greeks.delta : 0), 0);
  const netGamma = legs.reduce((s, l) => s + (l.greeks ? l.direction * l.quantity * l.greeks.gamma : 0), 0);
  const netTheta = legs.reduce((s, l) => s + (l.greeks ? l.direction * l.quantity * l.greeks.theta / 365 : 0), 0);
  const netVega = legs.reduce((s, l) => s + (l.greeks ? l.direction * l.quantity * l.greeks.vega : 0), 0);

  // Stats cards
  const statsEl = $('strat-stats');
  statsEl.style.display = 'grid';

  const profitEdge = maxProfit > 1000000;
  const lossEdge = maxLoss < -1000000;

  statsEl.innerHTML = `
    <div class="stat-card">
      <div class="stat-label">Max Profit</div>
      <div class="stat-value positive">${profitEdge ? 'Unlimited' : '$' + maxProfit.toFixed(2)}</div>
    </div>
    <div class="stat-card">
      <div class="stat-label">Max Loss</div>
      <div class="stat-value negative">${lossEdge ? 'Unlimited' : '$' + maxLoss.toFixed(2)}</div>
    </div>
    <div class="stat-card">
      <div class="stat-label">Breakeven${breakevens.length > 1 ? 's' : ''}</div>
      <div class="stat-value">${breakevens.length > 0 ? breakevens.map(b => '$' + b.toFixed(1)).join(', ') : '—'}</div>
    </div>
    <div class="stat-card">
      <div class="stat-label">Net Greeks</div>
      <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.6;">
        Δ ${netDelta.toFixed(3)} &nbsp; Γ ${netGamma.toFixed(4)} &nbsp; Θ ${netTheta.toFixed(3)} &nbsp; V ${netVega.toFixed(2)}
      </div>
    </div>
  `;

  // Draw chart
  const canvas = $('strat-chart') as HTMLCanvasElement;
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

  const minY = Math.min(maxLoss, 0);
  const maxY = Math.max(maxProfit, 0);
  const yRange = (maxY - minY) || 1;
  const yPad = yRange * 0.15;

  const scaleX = (x: number) => padding.left + ((x - low) / (high - low)) * plotW;
  const scaleY = (y: number) => padding.top + plotH - ((y - (minY - yPad)) / (yRange + 2 * yPad)) * plotH;

  ctx.clearRect(0, 0, w, h);

  // Grid
  ctx.strokeStyle = '#EDE5D8';
  ctx.lineWidth = 0.5;
  for (let i = 0; i <= 5; i++) {
    const py = padding.top + (plotH * i) / 5;
    ctx.beginPath();
    ctx.moveTo(padding.left, py);
    ctx.lineTo(w - padding.right, py);
    ctx.stroke();
  }

  // Zero line
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

  // Strike lines
  ctx.setLineDash([5, 5]);
  ctx.strokeStyle = '#AAA';
  ctx.lineWidth = 0.75;
  for (const leg of legs) {
    const sx = scaleX(leg.strike);
    ctx.beginPath();
    ctx.moveTo(sx, padding.top);
    ctx.lineTo(sx, h - padding.bottom);
    ctx.stroke();
  }
  ctx.setLineDash([]);

  // Spot line
  const spotX = scaleX(spotPrice);
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

  // Breakeven dots
  for (const be of breakevens) {
    const bx = scaleX(be);
    ctx.beginPath();
    ctx.arc(bx, zeroY, 4, 0, Math.PI * 2);
    ctx.fillStyle = '#D97757';
    ctx.fill();
  }

  // Axis labels
  ctx.fillStyle = '#888';
  ctx.font = '11px Inter, sans-serif';
  ctx.textAlign = 'center';
  for (let i = 0; i <= 6; i++) {
    const val = low + (high - low) * (i / 6);
    ctx.fillText(`$${val.toFixed(0)}`, scaleX(val), h - padding.bottom + 18);
  }

  ctx.textAlign = 'right';
  for (let i = 0; i <= 5; i++) {
    const val = (minY - yPad) + (yRange + 2 * yPad) * (i / 5);
    const py = padding.top + plotH - (plotH * i) / 5;
    ctx.fillText(`$${val.toFixed(0)}`, padding.left - 8, py + 4);
  }

  // Spot label
  ctx.fillStyle = '#D97757';
  ctx.textAlign = 'center';
  ctx.font = '10px Inter, sans-serif';
  ctx.fillText(`Spot $${spotPrice}`, spotX, padding.top - 8);
}
