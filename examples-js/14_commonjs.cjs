// CommonJS entry: `require` and `module.exports` must produce import edges and
// top-level globals, never a false positive. No loop, no accumulation.
const path = require("node:path");

function basename(file) {
  return path.basename(file);
}

module.exports = { basename };
