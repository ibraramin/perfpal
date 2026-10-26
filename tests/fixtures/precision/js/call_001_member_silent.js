// Precision corpus — JS member-name collision silent control (must stay silent).
// Both calls have the enclosing function's name as their final segment, but the
// callee text is not a bare identifier (`this.router.handle`,
// `new ListExtensionsCommand().execute`), so PERF-JS-CALL-001 must not fire.
export class Router {
  handle(req) {
    return this.router.handle(req);
  }
}

export function execute(cmd) {
  return new ListExtensionsCommand().execute(cmd);
}
