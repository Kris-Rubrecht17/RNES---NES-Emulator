use crate::cpu::CPU;








pub enum UiEvent {
    Quit,
    LoadCart(String),
    ControllerInput(u8)
}




unsafe impl Send for UiEvent {}