<?php

function flat_condition(bool $a, bool $b, bool $c): void
{
    if ($a && $b && $c) {
    }
}

function nested_condition(bool $a, bool $b, bool $c): void
{
    if ($a && ($b || !$c)) {
    }
}

function php_operator_conditions(mixed $a, mixed $b, mixed $c): void
{
    if ($a and $b) {}
    if ($a or $b) {}
    if ($a xor $b) {}
    if ($a ?? $b) {}
    if (($a && $b) |> trim(...)) {}
    if (($a | $b) === $c) {}
}

function branch_conditions(bool $a, bool $b, bool $c): void
{
    if ($a) {
    } elseif ($b) {
    } else if ($c) {
    }
}

function loop_and_ternary_conditions(bool $a, bool $b, array $items): int
{
    while ($a) {}
    do {} while ($b);
    for ($index = 0; $a && $b; $index++) {}
    for (;;) {}
    foreach ($items as $item) {}
    $free = $a && $b;
    return $a ? ($b ? 1 : 0) : 0;
}

function ternary_condition_barrier(bool $a, bool $b, bool $c, bool $d): void
{
    if ($a ? ($b && $c) : $d) {}
}

function grouped_condition_locations(bool $a, bool $b): int
{
    for ($index = 0; ($a && $b); $index++) {}
    return ($a && $b) ? 1 : 0;
}
