macro_rules! generated {
    () => {
        fn hidden() {
            if true {}
        }
    };
}

generated!();

fn visible() {
    generated!();
}
