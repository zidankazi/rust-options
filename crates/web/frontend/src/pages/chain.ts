import { fetchExpirations, fetchChain, fetchQuotes, fetchSparklines } from '../api';
import type { OptionChainEntry, StockQuote, SparklineData } from '../types';
import { $, formatNum, showLoading } from '../utils';

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

  const ctx = canvas.getContext('2d')!;
  const dpr = window.devicePixelRatio || 1;
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
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
  showLoading('chain-content');

  try {
    const data = await fetchExpirations(currentSymbol);
    renderStats(data.spot_price, data.expirations.length);
    renderExpiryDropdown(data.expirations);

    const now = Date.now() / 1000;
    const nextExpiry = data.expirations.find(e => e > now + 86400) || data.expirations[0];
    await loadExpiry(nextExpiry);
  } catch (err) {
    $('chain-content').innerHTML =
      `<div class="empty-state"><h3>Error</h3><p>${(err as Error).message}</p></div>`;
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
