#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TipRouterState {
    Idle,
    Snapshotting,
    Voting,
    Routing,
}
