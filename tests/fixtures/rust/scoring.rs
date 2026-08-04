fn scoring(a: bool, b: bool, c: bool, option: Option<u8>) {
    if a {
        while b && c || a {
            continue;
        }
    } else if let Some(value) = option {
        match value {
            0 if a && !b => {
                'retry: loop {
                    break 'retry;
                }
            }
            _ => {}
        }
    } else {
        for _ in 0..1 {}
    }
    let Some(_) = option else { return; };
}
