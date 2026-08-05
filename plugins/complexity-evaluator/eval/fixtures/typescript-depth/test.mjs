import assert from "node:assert/strict";

import { canReview } from "./subject.ts";

for (let flags = 0; flags < 16; flags += 1) {
  const hasDraft = Boolean(flags & 1);
  const hasTitle = Boolean(flags & 2);
  const hasBody = Boolean(flags & 4);
  const hasApproval = Boolean(flags & 8);
  const expected = hasDraft && hasTitle && hasBody && hasApproval;

  assert.equal(canReview(hasDraft, hasTitle, hasBody, hasApproval), expected);
}
