<?php

function flat_depth(bool $flag): void
{
    if ($flag) {
    }
}

function nested_depth(bool $flag, int $value): void
{
    if ($flag) {
        while ($flag) {
            switch ($value) {
                case 1:
                    if ($flag) {
                    }
            }
        }
    }
}

function catch_depth(bool $flag): void
{
    try {
        if ($flag) {
        }
    } catch (RuntimeException) {
        while ($flag) {
        }
    } finally {
        if ($flag) {
        }
    }
}

function ternary_depth(bool $first, bool $second): int
{
    return $first ? ($second ? 1 : 2) : 0;
}

function match_depth(int $value, bool $flag): int
{
    return match ($value) {
        1 => $flag ? 1 : 0,
        default => 0,
    };
}

function switch_selector_depth(bool $flag): void
{
    switch ($flag ? 1 : 0) {
        default:
            break;
    }
}

function switch_content_depth(int $value, bool $flag): void
{
    switch ($value) {
        case $flag ? 1 : 0:
            break;
    }
}

function match_selector_depth(bool $flag): int
{
    return match ($flag ? 1 : 0) {
        default => 0,
    };
}

function match_content_depth(int $value, bool $flag): int
{
    return match ($value) {
        1 => $flag ? 1 : 0,
        default => 0,
    };
}

function try_finally_depth(bool $flag): void
{
    try {
        if ($flag) {
        }
    } finally {
        if ($flag) {
        }
    }
}
