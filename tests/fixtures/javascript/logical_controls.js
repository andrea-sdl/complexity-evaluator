function chains(a, b, c, d) {
  const contiguous = a && b && c;
  const splitOr = a && b || c && d;
  const splitNullish = (a && b) ?? (c && d);
  const transparent = a && (b && c);
  const mixed = a && (b || c) && d;
}
