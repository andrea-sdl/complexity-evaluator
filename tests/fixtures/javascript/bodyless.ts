declare function ambient(value: boolean): boolean;

function overloaded(value: string): string;
function overloaded(value: number): number;
function overloaded(value: string | number) {
  if (value) {}
}

abstract class AbstractWorker {
  abstract run(value: boolean): void;

  complete() {
    if (true) {}
  }
}
