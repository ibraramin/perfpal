// Precision corpus — JS snapshot silent control (must stay silent).
export function snapshot(rows) {
  const copies = [];
  for (const row of rows) {
    const view = rows.slice();
    copies.push(view);
  }
  return copies;
}
