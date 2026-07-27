<?php

function nested_if(bool $a, bool $b): void {
    if ($a) {
        if ($b) {}
    }
}

function elseif_case(bool $a, bool $b): void {
    if ($a) {
    } elseif ($b) {
    }
}

function else_if_case(bool $a, bool $b, bool $c): void {
    if ($a) {
    } else if ($b) {
        if ($c) {}
    }
}

function switch_case(int $value, bool $flag): void {
    switch ($value) {
        case 1:
            if ($flag) {}
    }
}

function switch_expression_case(bool $a, bool $b): void {
    switch ($a ? 1 : 0) {
        case $b ? 1 : 0:
            break;
    }
}

function loop_case(array $items, bool $flag): void {
    for ($i = 0; $i < 1; $i++) {}
    foreach ($items as $item) {}
    while ($flag) {}
    do {} while ($flag);
}

function catch_case(): void {
    try {
    } catch (RuntimeException) {
    } finally {
    }
}

function ternary_case(bool $a, bool $b): int {
    return $a ? ($b ? 1 : 2) : 3;
}

function logical_case(bool $a, bool $b, bool $c): bool {
    return $a && $b || $c;
}

function pipe_case(string $value): int {
    return $value |> trim(...) |> strlen(...);
}

function mixed_operator_case(bool $a, bool $b): void {
    while ($a) {
        $first = $a && $b |> trim(...);
        $second = (/* lead */ ($a ? 1 : 0) && $b) |> trim(...);
    }
}

function break_case(bool $flag): void {
    while ($flag) {
        break /* a comment is not a level */;
        break 2;
    }
}

function continue_case(bool $flag): void {
    while ($flag) {
        continue /* a comment is not a level */;
        continue 2;
    }
}

function goto_case(): void {
    goto done;
    done:
}

function callable_parent(bool $flag): void {
    if ($flag) {
        function nested_named(bool $value): void {
            if ($value) {}
        }
        $nested = function (bool $value): void {
            if ($value) {}
        };
        $arrow = fn(bool $value): int => $value ? 1 : 0;
    }
}

function recursive_case(int $value): int {
    if ($value <= 0) {
        return 0;
    }
    return recursive_case($value - 1);
}

function match_case(int $value, bool $flag): int {
    return match ($value) {
        1 => $flag ? 1 : 0,
        default => 0,
    };
}

function zero_flow(mixed $value): Generator {
    yield $value;
    throw new RuntimeException();
}

#[Deprecated]
function modern_zero(?object $object): mixed {
    return $object?->call(value: 1) ?? Fiber::getCurrent();
}

function alternative_case(bool $flag): void {
    if ($flag):
        while ($flag):
            break;
        endwhile;
    else:
    endif;
}

class Hooked {
    public string $name {
        get => $this->name;
        set {
            $this->name = $value;
        }
    }
}
