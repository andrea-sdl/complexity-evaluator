<?php

class ReservedConstants
{
    private const ARRAY = 1;
    private const BOOL = 1;
    private const CALLABLE = 1;
    private const FALSE = 1;
    private const FLOAT = 1;
    private const INT = 1;
    private const ITERABLE = 1;
    private const MIXED = 1;
    private const string NAMESPACE /* shared prefix */ = 'metrics';
    private const NULL = 1;
    private const OBJECT = 1;
    private const STRING = 1;
    private const TRUE = 1;
    private const VOID = 1;

    public function record(bool $enabled): void
    {
        if ($enabled) {
        }
    }
}
