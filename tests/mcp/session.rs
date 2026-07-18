use mocari::mcp::session::{ModelSession, SessionError};

use super::helpers::ren_model_path;

#[test]
fn new_session_is_empty() {
    let session = ModelSession::new();
    assert!(session.models.is_empty());
    assert!(session.list_models().is_empty());
}

#[test]
fn load_model_with_real_file() {
    let mut session = ModelSession::new();
    let id = session.load_model(&ren_model_path()).expect("load should succeed");
    assert!(!id.is_empty());
    assert!(session.models.contains_key(&id));
}

#[test]
fn load_model_returns_unique_ids() {
    let mut session = ModelSession::new();
    let id1 = session.load_model(&ren_model_path()).expect("first load");
    let id2 = session.load_model(&ren_model_path()).expect("second load");
    assert_ne!(id1, id2);
    assert_eq!(session.models.len(), 2);
}

#[test]
fn load_model_nonexistent_path_fails() {
    let mut session = ModelSession::new();
    assert!(session.load_model("/nonexistent/path/to/model.model3.json").is_err());
}

#[test]
fn unload_model_valid_id() {
    let mut session = ModelSession::new();
    let id = session.load_model(&ren_model_path()).expect("load");
    assert!(session.unload_model(&id));
    assert!(session.models.is_empty());
}

#[test]
fn unload_model_invalid_id_returns_false() {
    let mut session = ModelSession::new();
    assert!(!session.unload_model("model_999"));
}

#[test]
fn list_models_reflects_state() {
    let mut session = ModelSession::new();
    assert!(session.list_models().is_empty());

    let id = session.load_model(&ren_model_path()).expect("load");
    let list = session.list_models();
    assert_eq!(list.len(), 1);
    assert!(list.iter().any(|(mid, _)| *mid == id));

    session.unload_model(&id);
    assert!(session.list_models().is_empty());
}

#[test]
fn with_model_valid_id() {
    let mut session = ModelSession::new();
    let id = session.load_model(&ren_model_path()).expect("load");
    assert_eq!(session.with_model(&id, |_| 42).unwrap(), 42);
}

#[test]
fn with_model_invalid_id_returns_error() {
    let session = ModelSession::new();
    match session.with_model("model_999", |_| 42).unwrap_err() {
        SessionError::ModelNotFound(id) => assert_eq!(id, "model_999"),
        other => panic!("expected ModelNotFound, got: {other}"),
    }
}

#[test]
fn with_model_mut_valid_id() {
    let mut session = ModelSession::new();
    let id = session.load_model(&ren_model_path()).expect("load");
    assert_eq!(session.with_model_mut(&id, |_| 99).unwrap(), 99);
}

#[test]
fn with_model_mut_invalid_id_returns_error() {
    let mut session = ModelSession::new();
    assert!(session.with_model_mut("model_999", |_| 42).is_err());
}

#[test]
fn load_unload_list_cycle() {
    let mut session = ModelSession::new();
    let id1 = session.load_model(&ren_model_path()).expect("load 1");
    let id2 = session.load_model(&ren_model_path()).expect("load 2");
    assert_eq!(session.list_models().len(), 2);

    session.unload_model(&id1);
    assert_eq!(session.list_models().len(), 1);
    assert!(session.list_models().iter().any(|(mid, _)| *mid == id2));

    session.unload_model(&id2);
    assert!(session.list_models().is_empty());
}
