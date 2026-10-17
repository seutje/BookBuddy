use bookbuddy_lib::services::{data_directory, launch_directory};
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

#[test]
fn installed_acceptance_runs_cannot_select_a_household_or_arbitrary_path() {
    let base = PathBuf::from("local-app-data");
    let id = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        launch_directory(
            base.clone(),
            false,
            Some("ignored"),
            &[format!("--acceptance-run={id}")]
        )
        .unwrap(),
        base.join("acceptance").join(&id)
    );
    assert_eq!(
        launch_directory(base.clone(), false, Some(&id), &[]).unwrap(),
        base
    );
    assert!(launch_directory(
        base.clone(),
        false,
        None,
        &["--acceptance-run=../../household".into()]
    )
    .is_err());
    assert!(launch_directory(
        base,
        false,
        None,
        &[
            format!("--acceptance-run={id}"),
            format!("--acceptance-run={id}")
        ]
    )
    .is_err());
}
