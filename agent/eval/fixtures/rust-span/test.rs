#[path = "subject.rs"]
mod subject;

#[test]
fn returns_a_zone_only_for_expedited_contract_shipments() {
    assert_eq!(subject::shipping_zone(0, true, true), "zone-01");
    assert_eq!(subject::shipping_zone(41, true, true), "zone-42");
    assert_eq!(subject::shipping_zone(42, true, true), "unknown");
    assert_eq!(subject::shipping_zone(0, true, false), "standard");
    assert_eq!(subject::shipping_zone(0, false, true), "standard");
}
