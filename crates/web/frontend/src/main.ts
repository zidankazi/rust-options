import './tooltip';
import { initHome } from './pages/home';
import { initChain } from './pages/chain';
import { initCalculator } from './pages/calculator';
import { initStrategy } from './pages/strategy';

// --- Route mapping ---

const ROUTES: Record<string, string> = {
  '/': 'home',
  '/options': 'chain',
  '/pricer': 'calculator',
  '/strategies': 'strategy',
  '/portfolio': 'portfolio',
  '/backtest': 'backtest',
};

function navigateTo(path: string): void {
  const page = ROUTES[path];
  if (!page) return;
  history.pushState(null, '', path);
  showPage(page);
}

function showPage(page: string): void {
  document.querySelectorAll('.nav-item').forEach(n => {
    n.classList.toggle('active', (n as HTMLElement).dataset.page === page);
  });
  document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
  document.getElementById(`page-${page}`)?.classList.add('active');
}

// --- Navigation ---

document.querySelectorAll('.nav-item').forEach(item => {
  item.addEventListener('click', (e) => {
    e.preventDefault();
    if (item.classList.contains('disabled')) return;
    const page = (item as HTMLElement).dataset.page!;
    const path = Object.entries(ROUTES).find(([_, v]) => v === page)?.[0] || '/';
    navigateTo(path);
  });
});

window.addEventListener('popstate', () => {
  const page = ROUTES[window.location.pathname] || 'home';
  showPage(page);
});

// Listen for navigation from home feature cards
window.addEventListener('app-navigate', ((e: CustomEvent) => {
  navigateTo(e.detail);
}) as EventListener);

// --- Init pages ---

initHome();
initChain();
initCalculator();
initStrategy();

// Show correct page on initial load
const initialPage = ROUTES[window.location.pathname] || 'home';
showPage(initialPage);
