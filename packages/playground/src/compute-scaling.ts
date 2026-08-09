const dpr = window.devicePixelRatio;

const unitScale = computeScaleDprToInteger();
const globalScale = Math.ceil(dpr) / (dpr * unitScale);
document.body.style.setProperty("--us", String(unitScale));
document.body.style.setProperty("--gs", String(globalScale));

function computeScaleDprToInteger() {
  for (let u = 1; u < 32; ++u) {
    if (floatEq(u * dpr, Math.round(u * dpr))) {
      return u;
    }
  }
  return 1;
}

function floatEq(a: number, b: number) {
  return Math.abs(a - b) < 1e-6;
}
