// Precision corpus — JS tiny-loop silent control (must stay silent).
export function tiny(items) {
  let out = "";
  for (let i = 0; i < 4; i++) {
    out += `item:${items[i]}`;
  }
  return out;
}
