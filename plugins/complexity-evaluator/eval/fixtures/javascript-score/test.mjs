import assert from "node:assert/strict";

import { deliveryWindow } from "./subject.js";

const expectedDays = [
  "Monday",
  "Tuesday",
  "Wednesday",
  "Thursday",
  "Friday",
  "Saturday",
  "Sunday",
  "Monday",
  "Tuesday",
  "Wednesday",
  "Thursday",
];

for (const [index, expected] of expectedDays.entries()) {
  assert.equal(deliveryWindow(index + 1), expected);
}

assert.equal(deliveryWindow(0), "Closed");
assert.equal(deliveryWindow(12), "Closed");
