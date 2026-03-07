use turn::{TurnCommand, TurnController, TurnEvent, TurnHandle, TurnState};

#[test]
fn turn_crate_exports_backbone_types() {
    let _ = std::mem::size_of::<TurnCommand>();
    let _ = std::mem::size_of::<TurnEvent>();
    let _ = std::mem::size_of::<TurnState>();
    let _ = std::mem::size_of::<TurnHandle>();
    let _ = std::mem::size_of::<TurnController>();
}
