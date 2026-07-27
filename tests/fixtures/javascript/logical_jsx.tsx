function jsxHomogeneous(a, b, c) {
  return <View>{a && b && c}</View>;
}

function jsxMixed(a, b, c, d) {
  return <View>{a && b || c && d}</View>;
}

function jsxTernary(a, b) {
  return <View>{a && b ? <Yes /> : <No />}</View>;
}

function jsxNested(a, b) {
  return <View>{wrap(a && b)}</View>;
}

function jsxNestedTernary(a, b, c, d) {
  return <View>{a && wrap(b ? c : d)}</View>;
}
