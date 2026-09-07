use crate::*;

#[test]
fn legal_stop_sequence() {
    let mut s = StopStatus::Pending;
    for a in [
        StopAction::Arrive,
        StopAction::StartActivity,
        StopAction::CompleteActivity,
        StopAction::Depart,
    ] {
        s = check_transition(s, a).unwrap();
    }
    assert_eq!(s, StopStatus::Departed);
}

#[test]
fn rejects_skipping_ahead() {
    assert!(check_transition(StopStatus::Pending, StopAction::CompleteActivity).is_err());
    assert!(check_transition(StopStatus::Arrived, StopAction::Arrive).is_err());
    assert!(check_transition(StopStatus::Working, StopAction::Depart).is_err());
}

#[test]
fn skip_only_from_pending() {
    assert!(check_transition(StopStatus::Pending, StopAction::Skip).is_ok());
    assert!(check_transition(StopStatus::Arrived, StopAction::Skip).is_err());
}
