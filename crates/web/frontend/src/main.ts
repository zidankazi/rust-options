import './tooltip';
import { initChain } from './pages/chain';
import { initCalculator } from './pages/calculator';

// --- Navigation ---

document.querySelectorAll('.nav-item').forEach(item => {
  item.addEventListener('click', () => {
    if (item.classList.contains('disabled')) return;

    document.querySelectorAll('.nav-item').forEach(n => n.classList.remove('active'));
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));

    item.classList.add('active');
    const page = (item as HTMLElement).dataset.page;
    document.getElementById(`page-${page}`)!.classList.add('active');
  });
});

// --- Init pages ---

initChain();
initCalculator();
