function none() {}

function flat(value) {
  if (value) {}
}

function boundaries(value) {
  while (value) {}
  switch (value) {}
  try {} catch (error) {}
  value ? 1 : 0;
}

function tryFinally(value) {
  try {
    value;
  } finally {
    value;
  }
}

function nested(value) {
  if (value) {
    while (value) {
      switch (value) {
        default:
          try {} catch (error) {
            return value ? 1 : 0;
          }
      }
    }
  }
}

function outerReset(value) {
  if (value) {
    const nestedReset = () => {
      while (value) {
        if (value) {}
      }
    };
  }
}

function loopBoundary(value) {
  while (value) {}
}

function switchBoundary(value) {
  switch (value) {}
}

function catchBoundary() {
  try {} catch (error) {}
}

function ternaryBoundary(value) {
  return value ? 1 : 0;
}
