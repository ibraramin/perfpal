// Precision corpus — JS delete property (PERF-JS-TYPE-001): planted TP.
export function dropField(record) {
  delete record.prop;
}
