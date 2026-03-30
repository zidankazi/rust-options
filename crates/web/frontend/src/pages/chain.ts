import { fetchExpirations, fetchChain, fetchQuotes, fetchSparklines, fetchVolSurface } from '../api';
import type { OptionChainEntry, StockQuote, SparklineData } from '../types';
import { $, formatNum, showLoading } from '../utils';

declare const Plotly: any;

let currentSymbol = '';
let currentSpot = 0;
let currentEntries: OptionChainEntry[] = [];
let showAllStrikes = false;

const INDICES = ['SPY', 'QQQ', 'DIA', 'IWM'];
const SECTORS: Record<string, string[]> = {
  'Technology': ['AAPL', 'MSFT', 'GOOGL', 'AMZN', 'NVDA', 'META', 'TSLA', 'AMD', 'NFLX', 'CRM', 'ORCL', 'INTC'],
  'Finance': ['JPM', 'BAC', 'GS', 'V', 'MA', 'BLK', 'MS', 'AXP'],
  'Healthcare': ['JNJ', 'UNH', 'PFE', 'LLY', 'MRK', 'ABBV', 'TMO', 'ABT'],
  'ETFs': ['SPY', 'QQQ', 'IWM', 'DIA', 'XLF', 'XLK', 'VTI', 'ARKK'],
};

const ALL_SYMBOLS = [...new Set([...INDICES, ...Object.values(SECTORS).flat()])];

export function initChain(): void {
  $('search-btn').addEventListener('click', () => searchSymbol());
  $('symbol-input').addEventListener('keydown', (e) => {
    if ((e as KeyboardEvent).key === 'Enter') searchSymbol();
  });
  $('back-btn').addEventListener('click', () => goHome());

  showLanding();
}

function goHome(): void {
  currentSymbol = '';
  ($('symbol-input') as HTMLInputElement).value = '';
  $('back-btn').style.display = 'none';
  $('vol-surface-section').style.display = 'none';
  showLanding();
}

// --- Landing: Market Overview ---

async function showLanding(): Promise<void> {
  $('chain-stats').style.display = 'none';
  $('expiry-tabs').style.display = 'none';

  const content = $('chain-content');
  content.innerHTML = `
    <div class="market-overview">
      <div class="indices-bar" id="indices-bar">
        ${INDICES.map(s => `
          <div class="index-card loading-shimmer" data-symbol="${s}">
            <div class="index-header">
              <span class="index-symbol">${s}</span>
              <span class="index-change">—</span>
            </div>
            <div class="index-price">—</div>
            <canvas class="index-sparkline" data-symbol="${s}" width="200" height="50"></canvas>
          </div>
        `).join('')}
      </div>

      ${Object.entries(SECTORS).map(([sector, symbols]) => `
        <div class="sector-section">
          <div class="section-title">${sector}</div>
          <div class="sector-table">
            <table>
              <thead>
                <tr>
                  <th>Symbol</th>
                  <th>Name</th>
                  <th>Price</th>
                  <th>Change</th>
                  <th>%</th>
                </tr>
              </thead>
              <tbody>
                ${symbols.map(s => `
                  <tr class="stock-row" data-symbol="${s}">
                    <td class="stock-row-symbol">${s}</td>
                    <td class="stock-row-name">—</td>
                    <td class="stock-row-price">—</td>
                    <td class="stock-row-change">—</td>
                    <td class="stock-row-pct">—</td>
                  </tr>
                `).join('')}
              </tbody>
            </table>
          </div>
        </div>
      `).join('')}
    </div>
  `;

  // Click handlers for stock rows
  document.querySelectorAll('.stock-row').forEach(row => {
    row.addEventListener('click', () => {
      const symbol = (row as HTMLElement).dataset.symbol!;
      ($('symbol-input') as HTMLInputElement).value = symbol;
      searchSymbol();
    });
  });

  document.querySelectorAll('.index-card').forEach(card => {
    card.addEventListener('click', () => {
      const symbol = (card as HTMLElement).dataset.symbol!;
      ($('symbol-input') as HTMLInputElement).value = symbol;
      searchSymbol();
    });
  });

  // Load quotes
  try {
    const quotes = await fetchQuotes(ALL_SYMBOLS);
    const quoteMap = new Map(quotes.map(q => [q.symbol, q]));
    applyQuotes(quoteMap);
  } catch (_) { /* landing degrades gracefully */ }

  // Load sparklines for indices
  try {
    const sparklines = await fetchSparklines(INDICES);
    for (const sp of sparklines) {
      drawSparkline(sp);
    }
  } catch (_) { /* sparklines are optional */ }
}

function applyQuotes(quoteMap: Map<string, StockQuote>): void {
  // Index cards
  document.querySelectorAll('.index-card').forEach(card => {
    const symbol = (card as HTMLElement).dataset.symbol!;
    const q = quoteMap.get(symbol);
    if (!q) return;
    card.classList.remove('loading-shimmer');

    const isUp = q.change >= 0;
    const sign = isUp ? '+' : '';
    const cls = isUp ? 'positive' : 'negative';

    card.querySelector('.index-price')!.textContent = `$${q.price.toFixed(2)}`;
    const changeEl = card.querySelector('.index-change')!;
    changeEl.textContent = `${sign}${q.change_percent.toFixed(2)}%`;
    changeEl.className = `index-change ${cls}`;
  });

  // Table rows
  document.querySelectorAll('.stock-row').forEach(row => {
    const symbol = (row as HTMLElement).dataset.symbol!;
    const q = quoteMap.get(symbol);
    if (!q) return;

    const isUp = q.change >= 0;
    const sign = isUp ? '+' : '';
    const cls = isUp ? 'positive' : 'negative';

    row.querySelector('.stock-row-name')!.textContent = q.name;
    row.querySelector('.stock-row-price')!.textContent = `$${q.price.toFixed(2)}`;

    const changeEl = row.querySelector('.stock-row-change')!;
    changeEl.textContent = `${sign}${q.change.toFixed(2)}`;
    changeEl.className = `stock-row-change ${cls}`;

    const pctEl = row.querySelector('.stock-row-pct')!;
    pctEl.textContent = `${sign}${q.change_percent.toFixed(2)}%`;
    pctEl.className = `stock-row-pct ${cls}`;
  });
}

function drawSparkline(data: SparklineData): void {
  const canvas = document.querySelector(
    `.index-sparkline[data-symbol="${data.symbol}"]`
  ) as HTMLCanvasElement | null;
  if (!canvas || data.prices.length < 2) return;

  // Wait for layout if canvas isn't sized yet
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  if (w === 0 || h === 0) {
    requestAnimationFrame(() => drawSparkline(data));
    return;
  }

  const ctx = canvas.getContext('2d')!;
  const dpr = window.devicePixelRatio || 1;
  canvas.width = w * dpr;
  canvas.height = h * dpr;
  ctx.scale(dpr, dpr);

  const prices = data.prices;
  const min = Math.min(...prices);
  const max = Math.max(...prices);
  const range = max - min || 1;

  const isUp = prices[prices.length - 1] >= prices[0];
  const color = isUp ? '#8BA88A' : '#C45B5B';

  // Line
  ctx.beginPath();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.lineJoin = 'round';

  for (let i = 0; i < prices.length; i++) {
    const x = (i / (prices.length - 1)) * w;
    const y = h - ((prices[i] - min) / range) * (h - 4) - 2;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();

  // Gradient fill
  ctx.lineTo(w, h);
  ctx.lineTo(0, h);
  ctx.closePath();
  const grad = ctx.createLinearGradient(0, 0, 0, h);
  grad.addColorStop(0, isUp ? 'rgba(139,168,138,0.2)' : 'rgba(196,91,91,0.15)');
  grad.addColorStop(1, 'rgba(255,255,255,0)');
  ctx.fillStyle = grad;
  ctx.fill();
}

// --- Chain View ---

function searchSymbol(): void {
  const symbol = ($('symbol-input') as HTMLInputElement).value.trim().toUpperCase();
  if (!symbol) return;
  currentSymbol = symbol;
  loadChain();
}

async function loadChain(): Promise<void> {
  $('chain-stats').style.display = 'none';
  $('expiry-tabs').style.display = 'none';
  $('vol-surface-section').style.display = 'none';
  showLoading('chain-content');

  try {
    const data = await fetchExpirations(currentSymbol);
    renderStats(data.spot_price, data.expirations.length);
    renderExpiryDropdown(data.expirations);

    // Show vol surface section and wire button
    $('vol-surface-section').style.display = 'block';
    $('vol-surface-container').innerHTML = '<div class="empty-state" style="padding: 40px;"><p>Click "Load Surface" to compute implied volatility across all strikes and expirations.</p></div>';
    $('load-vol-surface').onclick = () => loadVolSurface();

    const now = Date.now() / 1000;
    const nextExpiry = data.expirations.find(e => e > now + 86400) || data.expirations[0];
    await loadExpiry(nextExpiry);
  } catch (err) {
    $('chain-content').innerHTML =
      `<div class="empty-state"><h3>Error</h3><p>${(err as Error).message}</p></div>`;
  }
}

async function loadVolSurface(): Promise<void> {
  const container = $('vol-surface-container');
  container.innerHTML = '<div class="loading"><div class="spinner"></div>Computing IV surface across all strikes and expirations...</div>';

  try {
    const data = await fetchVolSurface(currentSymbol);
    if (data.points.length === 0) {
      container.innerHTML = '<div class="empty-state"><p>Not enough data to build surface.</p></div>';
      return;
    }

    // Build grid data for Plotly
    // Group by expiry, then by strike
    const expirySet = [...new Set(data.points.map(p => p.expiry_days))].sort((a, b) => a - b);
    const strikeSet = [...new Set(data.points.map(p => p.strike))].sort((a, b) => a - b);

    const ivMap = new Map<string, number>();
    for (const p of data.points) {
      ivMap.set(`${p.strike}_${p.expiry_days}`, p.iv);
    }

    // Build Z matrix (IV values) with interpolation for missing points
    const z: (number | null)[][] = [];
    for (const exp of expirySet) {
      const row: (number | null)[] = [];
      for (const strike of strikeSet) {
        const val = ivMap.get(`${strike}_${exp}`);
        row.push(val !== undefined ? val : null);
      }
      z.push(row);
    }

    const trace = {
      type: 'surface',
      x: strikeSet,
      y: expirySet,
      z: z,
      colorscale: [
        [0, '#8BA88A'],
        [0.25, '#B8C9A3'],
        [0.5, '#F0E9DF'],
        [0.75, '#E8A882'],
        [1, '#D97757'],
      ],
      colorbar: {
        title: { text: 'IV %', font: { size: 12, color: '#555' } },
        tickfont: { size: 11, color: '#888' },
        thickness: 15,
        len: 0.6,
      },
      hovertemplate:
        'Strike: $%{x:.0f}<br>' +
        'Expiry: %{y:.0f} days<br>' +
        'IV: %{z:.1f}%<extra></extra>',
      contours: {
        z: { show: true, usecolormap: true, highlightcolor: '#fff', project: { z: false } }
      },
      lighting: { ambient: 0.7, diffuse: 0.5, specular: 0.2, roughness: 0.5 },
      opacity: 0.95,
    };

    const layout = {
      scene: {
        xaxis: {
          title: { text: 'Strike Price', font: { size: 12, color: '#555' } },
          tickfont: { size: 10, color: '#888' },
          gridcolor: '#E5D9C8',
          zerolinecolor: '#E5D9C8',
        },
        yaxis: {
          title: { text: 'Days to Expiry', font: { size: 12, color: '#555' } },
          tickfont: { size: 10, color: '#888' },
          gridcolor: '#E5D9C8',
          zerolinecolor: '#E5D9C8',
        },
        zaxis: {
          title: { text: 'IV %', font: { size: 12, color: '#555' } },
          tickfont: { size: 10, color: '#888' },
          gridcolor: '#E5D9C8',
          zerolinecolor: '#E5D9C8',
        },
        bgcolor: '#FAF6F0',
        camera: { eye: { x: 1.8, y: -1.8, z: 1.2 } },
      },
      paper_bgcolor: '#FAF6F0',
      margin: { l: 0, r: 0, t: 0, b: 0 },
      font: { family: 'Inter, sans-serif' },
    };

    const config = {
      responsive: true,
      displaylogo: false,
      modeBarButtonsToRemove: ['toImage', 'sendDataToCloud'],
    };

    Plotly.newPlot(container, [trace], layout, config);
  } catch (err) {
    container.innerHTML = `<div class="empty-state"><p>Error: ${(err as Error).message}</p></div>`;
  }
}

function renderStats(spotPrice: number, numExpirations: number): void {
  $('back-btn').style.display = 'block';

  const el = $('chain-stats');
  el.style.display = 'grid';
  el.innerHTML = `
    <div class="stat-card">
      <div class="stat-label">Symbol</div>
      <div class="stat-value">${currentSymbol}</div>
    </div>
    <div class="stat-card">
      <div class="stat-label">Spot Price</div>
      <div class="stat-value">$${spotPrice.toFixed(2)}</div>
    </div>
    <div class="stat-card">
      <div class="stat-label">Expirations</div>
      <div class="stat-value">${numExpirations}</div>
    </div>
  `;
}

function renderExpiryDropdown(expirations: number[]): void {
  const el = $('expiry-tabs');
  el.style.display = 'block';
  const now = Date.now() / 1000;
  const future = expirations.filter(e => e > now + 86400);

  const options = future.map(exp => {
    const date = new Date(exp * 1000);
    const days = Math.round((exp - now) / 86400);
    const dateStr = date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
    return { exp, dateStr, days };
  });

  const first = options[0];

  el.innerHTML = `
    <div class="expiry-dropdown">
      <label class="expiry-label">Expiration</label>
      <div class="dropdown" id="expiry-dd">
        <button class="dropdown-trigger">
          <span class="dropdown-value">
            <span class="dropdown-date">${first.dateStr}</span>
            <span class="dropdown-days">${first.days} days</span>
          </span>
          <svg class="dropdown-chevron" width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <div class="dropdown-menu">
          ${options.map(o => `
            <button class="dropdown-item${o.exp === first.exp ? ' active' : ''}" data-expiry="${o.exp}">
              <span class="dropdown-item-date">${o.dateStr}</span>
              <span class="dropdown-item-days">${o.days} days</span>
            </button>
          `).join('')}
        </div>
      </div>
    </div>
  `;

  const dd = $('expiry-dd');
  dd.querySelector('.dropdown-trigger')!.addEventListener('click', () => dd.classList.toggle('open'));
  dd.querySelectorAll('.dropdown-item').forEach(item => {
    item.addEventListener('click', () => {
      const exp = parseInt((item as HTMLElement).dataset.expiry!);
      dd.classList.remove('open');
      dd.querySelectorAll('.dropdown-item').forEach(i => i.classList.remove('active'));
      item.classList.add('active');
      dd.querySelector('.dropdown-date')!.textContent = item.querySelector('.dropdown-item-date')!.textContent;
      dd.querySelector('.dropdown-days')!.textContent = item.querySelector('.dropdown-item-days')!.textContent;
      loadExpiry(exp);
    });
  });
  document.addEventListener('click', (e) => {
    if (!dd.contains(e.target as Node)) dd.classList.remove('open');
  });
}

async function loadExpiry(expiry: number): Promise<void> {
  showLoading('chain-content');
  showAllStrikes = false;
  try {
    const data = await fetchChain(currentSymbol, expiry);
    currentSpot = data.spot_price;
    currentEntries = data.entries;
    renderChainTable();
  } catch (err) {
    $('chain-content').innerHTML =
      `<div class="empty-state"><h3>Error</h3><p>${(err as Error).message}</p></div>`;
  }
}

const TABLE_HEADERS = `
  <tr>
    <th data-tip="The price at which you can buy (call) or sell (put) the underlying stock">Strike</th>
    <th data-tip="Highest price a buyer is willing to pay right now">Bid</th>
    <th data-tip="Lowest price a seller is willing to accept right now">Ask</th>
    <th data-tip="Implied Volatility — the market's forecast of future price movement, extracted from the option's price">IV</th>
    <th data-tip="How much the option price moves per $1 move in the stock. 0.50 means the option gains $0.50 when the stock rises $1">Delta</th>
    <th data-tip="How fast delta itself changes per $1 stock move. High gamma means delta shifts quickly">Gamma</th>
    <th data-tip="How much value the option loses per day from time passing. Almost always negative">Θ/day</th>
    <th data-tip="How much the option price changes per 1% move in volatility">Vega</th>
    <th data-tip="Number of contracts traded today">Vol</th>
    <th data-tip="Open Interest — total outstanding contracts not yet closed or exercised">OI</th>
  </tr>
`;

function renderChainTable(): void {
  const content = $('chain-content');

  if (currentEntries.length === 0) {
    content.innerHTML = '<div class="empty-state"><h3>No contracts</h3><p>No option contracts found for this expiration.</p></div>';
    return;
  }

  const filtered = showAllStrikes
    ? currentEntries
    : currentEntries.filter(e => {
        const pct = Math.abs(e.strike - currentSpot) / currentSpot;
        return pct <= 0.20;
      });

  const hiddenCount = currentEntries.length - filtered.length;
  const calls = filtered.filter(e => e.option_type === 'Call');
  const puts = filtered.filter(e => e.option_type === 'Put');

  const toggleText = showAllStrikes
    ? `Show near-the-money only`
    : `Show all strikes (+${hiddenCount} hidden)`;

  content.innerHTML = `
    <div style="margin-bottom: 10px;">
      <button class="strike-filter-btn" id="toggle-strikes">${toggleText}</button>
    </div>
    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px;">
      <div>
        <div class="section-title">Calls</div>
        <div class="table-container">
          <table>
            <thead>${TABLE_HEADERS}</thead>
            <tbody>${calls.map(contractRow).join('')}</tbody>
          </table>
        </div>
      </div>
      <div>
        <div class="section-title">Puts</div>
        <div class="table-container">
          <table>
            <thead>${TABLE_HEADERS}</thead>
            <tbody>${puts.map(contractRow).join('')}</tbody>
          </table>
        </div>
      </div>
    </div>
  `;

  $('toggle-strikes').addEventListener('click', () => {
    showAllStrikes = !showAllStrikes;
    renderChainTable();
  });
}

function contractRow(c: OptionChainEntry): string {
  const hasGreeks = c.greeks && c.implied_volatility;
  const muted = hasGreeks ? '' : ' class="muted"';
  const iv = c.implied_volatility ? (c.implied_volatility * 100).toFixed(1) + '%' : '—';
  const delta = c.greeks ? c.greeks.delta.toFixed(3) : '—';
  const gamma = c.greeks ? c.greeks.gamma.toFixed(4) : '—';
  const theta = c.greeks ? (c.greeks.theta / 365).toFixed(3) : '—';
  const vega = c.greeks ? (c.greeks.vega / 100).toFixed(3) : '—';
  return `
    <tr>
      <td>${c.strike.toFixed(2)}</td>
      <td>${c.bid.toFixed(2)}</td>
      <td>${c.ask.toFixed(2)}</td>
      <td${muted}>${iv}</td>
      <td${muted}>${delta}</td>
      <td${muted}>${gamma}</td>
      <td${muted} style="${hasGreeks ? 'color: var(--red)' : ''}">${theta}</td>
      <td${muted}>${vega}</td>
      <td>${formatNum(c.volume)}</td>
      <td>${formatNum(c.open_interest)}</td>
    </tr>
  `;
}
