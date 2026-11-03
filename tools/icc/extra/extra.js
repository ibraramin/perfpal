function jsNested(n) {
  let s = 0;
  for (let i = 0; i < n; i++) {
    for (const x of [i]) {
      s += x;
    }
  }
  return s;
}

function jsCallback(n) {
  let s = 0;
  const f = () => {
    while (s < n) {
      s++;
    }
  };
  f();
  return s;
}
