pub enum UiEvent {
    Quit,
    EnableCheats,
    LoadCart(String),
    ControllerInput(u8),
}

unsafe impl Send for UiEvent {}
