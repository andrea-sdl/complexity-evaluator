function statementContainers(source, left, right, scope, fallback, body, end) {
  for ({ [left && right]: target } in source) {}
  with (scope && fallback) {
    body && end;
  }
}
