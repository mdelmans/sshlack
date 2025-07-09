use terminal_keycode::Decoder;

use terminal_keycode::KeyCode;

use crate::app::{App, app_state::InputMode};

impl App {
    pub async fn process_input_data(&mut self, data: &[u8]) -> Result<(), anyhow::Error> {
        if let Some(controller) = &self.app_controller {
            let mut controller = controller.lock().await;
            match controller.app_state.input_mode {
                InputMode::Insert => {
                    for &byte in data {
                        for keycode in self.decoder.write(byte) {
                            match keycode {
                                KeyCode::CtrlN => {
                                    controller.set_mode(InputMode::Navigate);
                                    self.decoder = Decoder::new();
                                }
                                KeyCode::Char(c) => {
                                    controller.write_to_input(Some(c));
                                }
                                KeyCode::Space => {
                                    controller.write_to_input(Some(' '));
                                }
                                KeyCode::Backspace => {
                                    controller.write_to_input(None);
                                }
                                KeyCode::Enter => {
                                    let input_message = controller.get_input_message();
                                    if !input_message.is_empty() {
                                        if let Ok(_) = controller.send_message(input_message).await
                                        {
                                            controller.clear_input();
                                        };
                                    }
                                }
                                KeyCode::CtrlQ => {
                                    controller.disconnect().await;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                InputMode::Navigate => {
                    for &byte in data {
                        for keycode in self.decoder.write(byte) {
                            match keycode {
                                KeyCode::Enter => {
                                    controller.set_mode(InputMode::Insert);
                                }
                                KeyCode::Char('q') => {
                                    controller.disconnect().await;
                                }
                                KeyCode::Char('k') => {
                                    controller.scroll_up(1);
                                }
                                KeyCode::Char('j') => {
                                    controller.scroll_down(1);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
