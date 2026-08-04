fn outer() {
    if true {}
    let closure = || if true {};
    fn nested() {
        if true {}
    }
    closure();
}

fn café() { if true {} }
