pub fn shipping_zone(code: usize, has_contract: bool, expedited: bool) -> &'static str {
    let zones = [
        "zone-01",
        "zone-02",
        "zone-03",
        "zone-04",
        "zone-05",
        "zone-06",
        "zone-07",
        "zone-08",
        "zone-09",
        "zone-10",
        "zone-11",
        "zone-12",
        "zone-13",
        "zone-14",
        "zone-15",
        "zone-16",
        "zone-17",
        "zone-18",
        "zone-19",
        "zone-20",
        "zone-21",
        "zone-22",
        "zone-23",
        "zone-24",
        "zone-25",
        "zone-26",
        "zone-27",
        "zone-28",
        "zone-29",
        "zone-30",
        "zone-31",
        "zone-32",
        "zone-33",
        "zone-34",
        "zone-35",
        "zone-36",
        "zone-37",
        "zone-38",
        "zone-39",
        "zone-40",
        "zone-41",
        "zone-42",
    ];

    if has_contract {
        if expedited {
            return zones.get(code).copied().unwrap_or("unknown");
        }
    }

    "standard"
}
