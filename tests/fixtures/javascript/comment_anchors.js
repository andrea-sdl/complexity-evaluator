function commentAnchors(value, left, right, fallback) {
  return value /* ? */ ? left /* && */ && right : fallback;
}
