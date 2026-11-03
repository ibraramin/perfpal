function tsNested(n: number): number {
  let s = 0;
  for (let i = 0; i < n; i++) {
    while (s < i) {
      s++;
    }
  }
  return s;
}

function tsOf(xs: number[]): number {
  let s = 0;
  for (const x of xs) {
    s += x;
  }
  return s;
}
