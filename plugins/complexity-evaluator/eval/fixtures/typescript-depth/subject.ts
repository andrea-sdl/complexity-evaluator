export function canReview(
  hasDraft: boolean,
  hasTitle: boolean,
  hasBody: boolean,
  hasApproval: boolean,
): boolean {
  if (hasDraft) {
    if (hasTitle) {
      if (hasBody) {
        if (hasApproval) {
          return true;
        }
      }
    }
  }

  return false;
}
