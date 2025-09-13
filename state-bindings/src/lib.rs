use std::sync::Arc;

uniffi::setup_scaffolding!();

type CurrentStateUpdate = visualisation::CurrentStateUpdate;
type SandPileStateUpdate = visualisation::SandPileStateUpdate;
type TestVisUpdate = visualisation::TestVisUpdate;
type GameOfLifeUpdate = visualisation::GameOfLifeUpdate;
type TurmiteUpdate = visualisation::TurmiteUpdate;
type IsingUpdate = visualisation::IsingUpdate;

#[uniffi::remote(Enum)]
pub enum SandPileStateUpdate {
    Reset,
}

#[uniffi::remote(Enum)]
pub enum TestVisUpdate {
    Reset,
}

#[uniffi::remote(Enum)]
pub enum GameOfLifeUpdate {
    Reset,
}

#[uniffi::remote(Enum)]
pub enum TurmiteUpdate {
    Reset,
}

#[uniffi::remote(Enum)]
pub enum IsingUpdate {
    Reset,
}

#[uniffi::remote(Enum)]
pub enum CurrentStateUpdate {
    SandPile(SandPileStateUpdate),
    TestVis(TestVisUpdate),
    GameOfLife(GameOfLifeUpdate),
    Turmite(TurmiteUpdate),
    Ising(IsingUpdate),
}

#[uniffi::export]
pub fn serialize_state(state: &CurrentStateUpdate) -> Option<Vec<u8>> {
    let mut buffer = vec![0u8; 2048];
    if let Ok(buf) = postcard::to_slice(state, &mut buffer) {
	let len = buf.len();
        buffer.truncate(len);
        Some(buffer)
    } else {
        None
    }
}

#[uniffi::export]
pub fn deserialize_state(buffer: &[u8]) -> Option<CurrentStateUpdate> {
    postcard::from_bytes(buffer).ok()
}

#[uniffi::export]
pub fn add(x: u32, y: u32) -> u32 {
    x + y
}

#[derive(uniffi::Object)]
pub struct Obj;

#[uniffi::export]
impl Obj {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Obj)
    }

    #[uniffi::method]
    pub fn do_something(self: Arc<Self>) -> String {
        "hi".into()
    }
}
