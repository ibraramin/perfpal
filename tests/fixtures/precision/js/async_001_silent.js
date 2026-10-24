// Precision corpus — JS await silent control (must stay silent).
async function fetchJson(url) {
  return url.length;
}

export async function loadOne(url) {
  const payload = await fetchJson(url);
  return payload;
}
