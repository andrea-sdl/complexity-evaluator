function wrappers(
  a: boolean,
  b: boolean,
  c: boolean,
  predicate: <T>() => boolean,
) {
  if (((a && b && c) as boolean)) {}
  if ((a && b && c) satisfies boolean) {}
  if ((a && b && c)!) {}
  if (<boolean>(a && b && c)) {}
  if (predicate<boolean>) {}
}
