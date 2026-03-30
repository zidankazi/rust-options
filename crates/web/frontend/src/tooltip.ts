const tooltip = document.createElement('div');
tooltip.className = 'tooltip';
document.body.appendChild(tooltip);

document.addEventListener('mouseover', (e) => {
  const th = (e.target as HTMLElement).closest('th[data-tip]') as HTMLElement | null;
  if (!th) return;
  tooltip.textContent = th.dataset.tip!;
  const rect = th.getBoundingClientRect();
  tooltip.style.left = rect.left + rect.width / 2 - tooltip.offsetWidth / 2 + 'px';
  tooltip.style.top = rect.bottom + 8 + 'px';
  tooltip.classList.add('visible');
});

document.addEventListener('mouseout', (e) => {
  const th = (e.target as HTMLElement).closest('th[data-tip]') as HTMLElement | null;
  if (!th) return;
  tooltip.classList.remove('visible');
});
