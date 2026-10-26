// Precision corpus — JS delete-subscript silent control (must stay silent).
export function dropKey(record, key) {
  delete record[key];
}
