use bookbuddy_lib::services::data_directory;
use std::path::PathBuf;

#[test]
fn smoke_location_is_confined_and_release_ignores_overrides() {
    let base = PathBuf::from("local-app-data");
    let id = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        data_directory(base.clone(), false, Some("../../household")).unwrap(),
        base
    );
    assert_eq!(
        data_directory(base.clone(), true, None).unwrap(),
        base.join("development")
    );
    assert_eq!(
        data_directory(base.clone(), true, Some(&id)).unwrap(),
        base.join("development").join("smoke").join(&id)
    );
    assert!(data_directory(base, true, Some("../../household")).is_err());
}
