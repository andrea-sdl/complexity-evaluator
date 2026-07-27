<?php

function callable_condition(bool $flag): void
{
    if ((function (bool $a, bool $b): bool {
        if ($a && $b) {
            return true;
        }
        return false;
    })(true, false) && $flag) {
    }
}
