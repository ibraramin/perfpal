// Precision corpus — JS truthiness-guard silent control (must stay silent).
// The `if` condition is a visible base case but contributes no comparison
// event, so only the conditional-guard gate keeps PERF-JS-CALL-001 silent.
export function getDefaults(definition) {
  if (definition.properties) return getDefaults(definition.properties);
  return {};
}
