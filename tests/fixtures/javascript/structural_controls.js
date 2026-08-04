function loops(items) {
  for (;;) {
    while (items.length) {}
  }
  for (const item of items) {}
  for (const key in items) {}
  do {} while (items.length);
}

function switched(value) {
  switch (value) {
    case 1:
      if (value) {}
      break;
    default:
      if (value) {}
  }
}

function recovered(value) {
  try {
    if (value) {}
  } catch (error) {
    if (value) {}
  } finally {
    if (value) {}
  }
}

function ternary(value) {
  return value ? (value > 1 ? 2 : 1) : (value < 0 ? -1 : 0);
}

function jumps() {
  outer: for (;;) {
    if (true) {
      continue outer;
    }
    break outer;
  }
}
