function flat(a: boolean, b: boolean, c: boolean) {
  if (a && b && c) {}
}

function mixed(a: boolean, b: boolean, c: boolean) {
  while (a && (b || !c)) {}
}

function containers(a: boolean, b: boolean) {
  if ((a && b) === true) {}
  if (Boolean(a || b)) {}
  if (() => a && b) {}
}

function nullish(a: boolean | null, b: boolean, c: boolean) {
  if ((a ?? b) && c) {}
}
