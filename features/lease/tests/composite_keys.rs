use miresidian_lease::{
    LeaseError, LeaseParameter, LeaseRequest, MAX_RESOURCE_KEY_BYTES, ResourceKey,
};
#[test]
fn composite_boundaries_and_unicode_are_unambiguous() {
    assert_ne!(
        ResourceKey::from_parts(&["a:b", "c"]).unwrap(),
        ResourceKey::from_parts(&["a", "b:c"]).unwrap()
    );
    assert_ne!(
        ResourceKey::from_parts(&["a", "b"]).unwrap(),
        ResourceKey::from_parts(&["b", "a"]).unwrap()
    );
    let composite = ResourceKey::from_parts(&["a", "b"]).unwrap();
    assert_eq!(
        ResourceKey::from_parts(&[composite.as_str()]),
        Err(LeaseError::ReservedResourcePrefix)
    );
    assert!(ResourceKey::from_parts(&["串口", "设备"]).is_ok());
    assert_eq!(
        ResourceKey::from_parts(&["COM3"]).unwrap(),
        ResourceKey::new("COM3").unwrap()
    );
    for parts in [vec![], vec![""], vec!["a", " "]] {
        assert_eq!(
            ResourceKey::from_parts(&parts),
            Err(LeaseError::EmptyResource)
        );
    }
    assert_eq!(
        ResourceKey::from_parts(&[&"a".repeat(MAX_RESOURCE_KEY_BYTES), "b"]),
        Err(LeaseError::ResourceTooLong)
    );
}
#[test]
fn legacy_and_new_single_field_requests_keep_identical_bytes() {
    let parameters = vec![
        LeaseParameter::new("resource", "COM3"),
        LeaseParameter::new("baud", "9600"),
    ];
    let old = LeaseRequest::from_parameters(parameters.clone(), "resource", &["baud"]).unwrap();
    let new = LeaseRequest::from_parameter_keys(parameters, &["resource"], &["baud"]).unwrap();
    assert_eq!(old, new);
    assert_eq!(new.value("baud"), Some("9600"));
    assert_eq!(
        LeaseRequest::from_parameter_keys(vec![], &["missing"], &[]),
        Err(LeaseError::UnknownParameter("missing".into()))
    );
}
