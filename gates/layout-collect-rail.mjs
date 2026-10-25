// AppShellRail.tsx:151-360. Measure every visible control, including the bottom group.
export function collectRail() {
  const controls = [...document.querySelectorAll('.app-shell-rail button, .app-shell-rail a')].filter(element => {
    const box = element.getBoundingClientRect();
    if (box.width <= 0 || box.height <= 0) return false;
    for (let parent = element; parent; parent = parent.parentElement) {
      const style = getComputedStyle(parent);
      if (style.display === 'none' || style.visibility === 'hidden' || Number(style.opacity) === 0) return false;
    }
    return true;
  });
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 1;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  const hex = color => {
    context.clearRect(0, 0, 1, 1);
    context.fillStyle = color;
    context.fillRect(0, 0, 1, 1);
    const [r, g, b, a] = context.getImageData(0, 0, 1, 1).data;
    return a === 0 ? null : '#' + [r, g, b].map(value => value.toString(16).padStart(2, '0')).join('');
  };
  return controls.map((element, index) => {
    const box = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return {
      key: `rail.icon.${index}`, x: box.x, y: box.y, w: box.width, h: box.height,
      color: hex(style.color), bg: hex(style.backgroundColor), fontSize: parseFloat(style.fontSize),
      fontWeight: parseFloat(style.fontWeight), lineHeight: parseFloat(style.lineHeight),
      radius: parseFloat(style.borderTopLeftRadius), borderColor: hex(style.borderTopColor),
    };
  });
}
