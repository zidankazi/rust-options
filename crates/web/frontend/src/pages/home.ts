import { fetchBenchmark } from '../api';
import { $ } from '../utils';

let benchTimer: ReturnType<typeof setInterval> | null = null;

export function initHome(): void {
  const benchBtns = ['bench-1m', 'bench-5m', 'bench-10m'];
  const benchNs = [1_000_000, 5_000_000, 10_000_000];
  benchBtns.forEach((id, i) => {
    $(id).addEventListener('click', () => {
      benchBtns.forEach(b => {
        $(b).className = b === id ? 'btn btn-primary' : 'btn btn-secondary';
      });
      runBenchmark(benchNs[i]);
    });
  });

  // Navigate on feature card click
  window.addEventListener('navigate', ((e: CustomEvent) => {
    window.dispatchEvent(new CustomEvent('app-navigate', { detail: e.detail }));
  }) as EventListener);
}

async function runBenchmark(n: number): Promise<void> {
  const el = $('benchmark-result');
  el.style.display = 'block';

  if (benchTimer) clearInterval(benchTimer);

  const pythonEstMs = n * 20 / 1000;
  const pythonEstSec = pythonEstMs / 1000;

  el.innerHTML = `
    <div class="bench-comparison">
      <div class="bench-bar-group">
        <div class="bench-label">
          <span class="bench-lang">Rust</span>
          <span class="bench-time" id="rust-timer">running...</span>
        </div>
        <div class="bench-bar-track">
          <div class="bench-bar rust-bar" id="rust-bar" style="width: 0%; transition: width 300ms ease;"></div>
        </div>
        <div class="bench-detail" id="rust-detail">${(n / 1_000_000).toFixed(0)}M Black-Scholes calls with full Greeks</div>
      </div>
      <div class="bench-bar-group">
        <div class="bench-label">
          <span class="bench-lang">Python <span style="color: var(--text-muted); font-weight: 400;">(NumPy/SciPy)</span></span>
          <span class="bench-time" id="python-timer">waiting...</span>
        </div>
        <div class="bench-bar-track">
          <div class="bench-bar python-bar" id="python-bar" style="width: 0%; transition: width 100ms linear;"></div>
        </div>
        <div class="bench-detail" id="python-detail">~20μs per call · estimated ${pythonEstSec.toFixed(0)}s total</div>
      </div>
      <div class="bench-speedup" id="bench-speedup" style="display: none;">
        <span class="bench-speedup-number" id="speedup-number"></span>
        <span class="bench-speedup-label">faster</span>
      </div>
    </div>
  `;

  const startTime = performance.now();
  let rustDone = false;
  let rustMs = 0;

  const timerInterval = setInterval(() => {
    const elapsed = performance.now() - startTime;
    if (!rustDone) {
      $('rust-timer').textContent = `${(elapsed / 1000).toFixed(1)}s`;
    }
  }, 50);

  try {
    const data = await fetchBenchmark(n);
    rustDone = true;
    rustMs = data.total_ns / 1_000_000;
    clearInterval(timerInterval);

    $('rust-timer').textContent = formatMs(rustMs);
    $('rust-timer').style.color = 'var(--accent)';
    ($('rust-bar') as HTMLElement).style.width = `${Math.max(2, (rustMs / pythonEstMs) * 100)}%`;
    $('rust-detail').textContent = `${data.per_call_ns.toFixed(1)}ns per call · ${formatCps(data.calls_per_second)}/sec`;

    const pythonDuration = Math.min(pythonEstMs, 8000);
    const pythonStart = performance.now();
    $('python-timer').textContent = '0.0s';

    benchTimer = setInterval(() => {
      const elapsed = performance.now() - pythonStart;
      const progress = Math.min(elapsed / pythonDuration, 1);
      const simulated = progress * pythonEstSec;

      $('python-timer').textContent = `${simulated.toFixed(1)}s`;
      ($('python-bar') as HTMLElement).style.width = `${progress * 100}%`;

      if (simulated > 0) {
        const currentSpeedup = Math.round((simulated * 1000) / rustMs);
        $('bench-speedup').style.display = 'flex';
        $('speedup-number').textContent = `${currentSpeedup.toLocaleString()}x`;
      }

      if (progress >= 1) {
        clearInterval(benchTimer!);
        benchTimer = null;
        $('python-timer').textContent = formatMs(pythonEstMs);
        const finalSpeedup = Math.round(pythonEstMs / rustMs);
        $('speedup-number').textContent = `${finalSpeedup.toLocaleString()}x`;
        $('python-detail').textContent = `~20μs per call · ~50K/sec`;
      }
    }, 50);

  } catch (err) {
    clearInterval(timerInterval);
    el.innerHTML = `<p style="color: var(--red);">Benchmark failed: ${(err as Error).message}</p>`;
  }
}

function formatMs(ms: number): string {
  if (ms < 1) return `${(ms * 1000).toFixed(0)}μs`;
  if (ms < 1000) return `${ms.toFixed(1)}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

function formatCps(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(0)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return n.toFixed(0);
}
