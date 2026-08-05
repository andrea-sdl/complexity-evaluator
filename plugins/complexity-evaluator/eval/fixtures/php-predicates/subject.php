<?php

function canPublish(
    bool $hasDraft,
    bool $hasTitle,
    bool $hasBody,
    bool $hasEditor,
    bool $hasApproval,
): string {
    if ($hasDraft && $hasTitle && $hasBody && $hasEditor && $hasApproval) {
        return "publish";
    }

    return "draft";
}
