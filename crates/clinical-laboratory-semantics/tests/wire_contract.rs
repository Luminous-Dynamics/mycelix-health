use mycelix_clinical_laboratory_semantics::{
    LaboratoryResultStatusV1, MissingReasonV1,
};

#[test]
fn laboratory_result_status_tags_are_frozen_for_v1() {
    assert_eq!(LaboratoryResultStatusV1::Registered as u8, 0);
    assert_eq!(LaboratoryResultStatusV1::Partial as u8, 1);
    assert_eq!(LaboratoryResultStatusV1::Preliminary as u8, 2);
    assert_eq!(LaboratoryResultStatusV1::Final as u8, 3);
    assert_eq!(LaboratoryResultStatusV1::Amended as u8, 4);
    assert_eq!(LaboratoryResultStatusV1::Corrected as u8, 5);
    assert_eq!(LaboratoryResultStatusV1::Appended as u8, 6);
    assert_eq!(LaboratoryResultStatusV1::Cancelled as u8, 7);
    assert_eq!(LaboratoryResultStatusV1::EnteredInError as u8, 8);
    assert_eq!(LaboratoryResultStatusV1::Unknown as u8, 9);
}

#[test]
fn missing_reason_tags_are_frozen_for_v1() {
    assert_eq!(MissingReasonV1::NotProvided as u8, 0);
    assert_eq!(MissingReasonV1::Unknown as u8, 1);
    assert_eq!(MissingReasonV1::NotApplicable as u8, 2);
    assert_eq!(MissingReasonV1::Redacted as u8, 3);
}
