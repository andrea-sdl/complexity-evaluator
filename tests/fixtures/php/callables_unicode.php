<?php
/* é */ function outer(bool $flag): void {
    if ($flag) {
        $closure = function (bool $inner): void {
            if ($inner) {
            }
        };
    }
    $arrow = fn(bool $value): int => $value ? 1 : 0;
}

class Example {
    public function method(bool $flag): void {
        if ($flag) {
        }
    }
}
