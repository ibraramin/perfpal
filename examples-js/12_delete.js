// PERF-JS-TYPE-001 — `delete obj.prop` (Info). True positive: the hidden-class
// → dictionary-mode shape. Silent control: the subscript form, which is a
// different (keyed) operation and is excluded by the frozen disposition.

export function dropField(record) {
  delete record.prop; // TP PERF-JS-TYPE-001
}

export function dropKey(record, key) {
  delete record[key]; // silent: subscript form is excluded
}
