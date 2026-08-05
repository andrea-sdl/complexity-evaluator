<?php

require __DIR__ . "/subject.php";

for ($flags = 0; $flags < 32; $flags++) {
    $hasDraft = (bool) ($flags & 1);
    $hasTitle = (bool) ($flags & 2);
    $hasBody = (bool) ($flags & 4);
    $hasEditor = (bool) ($flags & 8);
    $hasApproval = (bool) ($flags & 16);
    $complete = $hasDraft && $hasTitle && $hasBody && $hasEditor && $hasApproval;
    $expected = $complete ? "publish" : "draft";

    if (canPublish($hasDraft, $hasTitle, $hasBody, $hasEditor, $hasApproval) !== $expected) {
        throw new RuntimeException("canPublish changed behavior for flag set {$flags}");
    }
}
