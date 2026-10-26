// Precision corpus — JS same-name delegation silent control (must stay silent).
// Object-property arrows and class methods derive their record name from a
// property key, not a lexical binding: the bare same-name call targets an
// outer function, never the enclosing method/property.
export const resolve = (value) => value;

export const handlers = {
  resolve: (value) => resolve(value),
};

export class Dispatcher {
  resolve(value) {
    return resolve(value);
  }
}
