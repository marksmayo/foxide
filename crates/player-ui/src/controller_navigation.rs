use crate::{PlayerWindow, dispatch};
use gilrs::{Button, EventType, Gilrs};
use player_render::Engine;
use player_ui::{Action, UiState};
use slint::{
    ComponentHandle, Timer, TimerMode,
    platform::{Key, WindowEvent},
};
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ControllerAction {
    FocusPrevious,
    FocusNext,
    Activate,
    Back,
    TogglePause,
    Previous,
    Next,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControllerEvent {
    Connected,
    Disconnected,
    ButtonPressed(Button),
}

fn translate_event(event: EventType) -> Option<ControllerEvent> {
    match event {
        EventType::Connected => Some(ControllerEvent::Connected),
        EventType::Disconnected => Some(ControllerEvent::Disconnected),
        EventType::ButtonPressed(button, _) => Some(ControllerEvent::ButtonPressed(button)),
        _ => None,
    }
}

fn action_for_button(button: Button) -> Option<ControllerAction> {
    Some(match button {
        Button::DPadUp | Button::DPadLeft => ControllerAction::FocusPrevious,
        Button::DPadDown | Button::DPadRight => ControllerAction::FocusNext,
        Button::South => ControllerAction::Activate,
        Button::East | Button::Select => ControllerAction::Back,
        Button::Start => ControllerAction::TogglePause,
        // gilrs calls the shoulder buttons LeftTrigger and RightTrigger.
        Button::LeftTrigger => ControllerAction::Previous,
        Button::RightTrigger => ControllerAction::Next,
        _ => return None,
    })
}

fn dispatch_key(window: &slint::Window, key: Key, pressed: bool) {
    let text = key.into();
    let event = if pressed {
        WindowEvent::KeyPressed { text }
    } else {
        WindowEvent::KeyReleased { text }
    };
    let _ = window.dispatch_event_with_result(event);
}

fn execute_action(
    action: ControllerAction,
    mut send_key: impl FnMut(Key, bool),
    mut send_player_action: impl FnMut(Action),
) {
    match action {
        ControllerAction::FocusPrevious => {
            send_key(Key::Shift, true);
            send_key(Key::Tab, true);
            send_key(Key::Tab, false);
            send_key(Key::Shift, false);
        }
        ControllerAction::FocusNext => {
            send_key(Key::Tab, true);
            send_key(Key::Tab, false);
        }
        ControllerAction::Activate => {
            send_key(Key::Return, true);
            send_key(Key::Return, false);
        }
        ControllerAction::Back => {
            send_key(Key::Escape, true);
            send_key(Key::Escape, false);
        }
        ControllerAction::TogglePause => send_player_action(Action::TogglePause),
        ControllerAction::Previous => send_player_action(Action::Previous),
        ControllerAction::Next => send_player_action(Action::Next),
    }
}

fn process_events(
    mut next_event: impl FnMut() -> Option<ControllerEvent>,
    mut set_status: impl FnMut(&'static str),
    mut perform_action: impl FnMut(ControllerAction),
) {
    while let Some(event) = next_event() {
        match event {
            ControllerEvent::Connected => set_status("Game controller connected"),
            ControllerEvent::Disconnected => set_status("Game controller disconnected"),
            ControllerEvent::ButtonPressed(button) => {
                if let Some(action) = action_for_button(button) {
                    perform_action(action);
                }
            }
        }
    }
}

pub(super) fn apply_action(
    ui: &PlayerWindow,
    state: &Rc<RefCell<UiState>>,
    engine: &Option<Rc<Engine>>,
    action: ControllerAction,
) {
    execute_action(
        action,
        |key, pressed| dispatch_key(ui.window(), key, pressed),
        |player_action| dispatch(ui, state, engine, player_action),
    );
}

pub(super) fn install(
    ui: &PlayerWindow,
    state: Rc<RefCell<UiState>>,
    engine: Option<Rc<Engine>>,
) -> Timer {
    let timer = Timer::default();
    let mut gilrs = match Gilrs::new() {
        Ok(gilrs) => gilrs,
        Err(error) => {
            ui.set_status(format!("Controller navigation unavailable: {error}").into());
            return timer;
        }
    };
    let weak = ui.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
        process_events(
            || {
                gilrs
                    .next_event()
                    .and_then(|event| translate_event(event.event))
            },
            |status| {
                if let Some(ui) = weak.upgrade() {
                    ui.set_status(status.into());
                }
            },
            |action| {
                if let Some(ui) = weak.upgrade() {
                    apply_action(&ui, &state, &engine, action);
                }
            },
        );
    });
    timer
}

#[cfg(test)]
mod tests {
    use super::{
        ControllerAction, ControllerEvent, action_for_button, execute_action, process_events,
        translate_event,
    };
    use gilrs::{Button, EventType};
    use player_ui::Action;
    use slint::platform::Key;

    #[test]
    fn standard_dpad_moves_focus_in_both_directions() {
        assert_eq!(
            action_for_button(Button::DPadUp),
            Some(ControllerAction::FocusPrevious)
        );
        assert_eq!(
            action_for_button(Button::DPadLeft),
            Some(ControllerAction::FocusPrevious)
        );
        assert_eq!(
            action_for_button(Button::DPadDown),
            Some(ControllerAction::FocusNext)
        );
        assert_eq!(
            action_for_button(Button::DPadRight),
            Some(ControllerAction::FocusNext)
        );
    }

    #[test]
    fn standard_face_and_shoulder_buttons_map_to_player_ui_actions() {
        assert_eq!(
            action_for_button(Button::South),
            Some(ControllerAction::Activate)
        );
        assert_eq!(
            action_for_button(Button::East),
            Some(ControllerAction::Back)
        );
        assert_eq!(
            action_for_button(Button::Select),
            Some(ControllerAction::Back)
        );
        assert_eq!(
            action_for_button(Button::Start),
            Some(ControllerAction::TogglePause)
        );
        assert_eq!(
            action_for_button(Button::LeftTrigger),
            Some(ControllerAction::Previous)
        );
        assert_eq!(
            action_for_button(Button::RightTrigger),
            Some(ControllerAction::Next)
        );
        assert_eq!(action_for_button(Button::LeftTrigger2), None);
        assert_eq!(action_for_button(Button::RightTrigger2), None);
    }

    #[test]
    fn mapped_actions_drive_slint_focus_and_existing_player_actions() {
        let mut keys = Vec::new();
        let mut actions = Vec::new();
        execute_action(
            ControllerAction::FocusPrevious,
            |key, pressed| keys.push((key, pressed)),
            |action| actions.push(action),
        );
        execute_action(
            ControllerAction::FocusNext,
            |key, pressed| keys.push((key, pressed)),
            |action| actions.push(action),
        );
        execute_action(
            ControllerAction::Activate,
            |key, pressed| keys.push((key, pressed)),
            |action| actions.push(action),
        );
        execute_action(
            ControllerAction::Back,
            |key, pressed| keys.push((key, pressed)),
            |action| actions.push(action),
        );
        execute_action(
            ControllerAction::TogglePause,
            |key, pressed| keys.push((key, pressed)),
            |action| actions.push(action),
        );
        execute_action(
            ControllerAction::Previous,
            |key, pressed| keys.push((key, pressed)),
            |action| actions.push(action),
        );
        execute_action(
            ControllerAction::Next,
            |key, pressed| keys.push((key, pressed)),
            |action| actions.push(action),
        );

        assert_eq!(
            keys,
            vec![
                (Key::Shift, true),
                (Key::Tab, true),
                (Key::Tab, false),
                (Key::Shift, false),
                (Key::Tab, true),
                (Key::Tab, false),
                (Key::Return, true),
                (Key::Return, false),
                (Key::Escape, true),
                (Key::Escape, false),
            ]
        );
        assert_eq!(
            actions,
            vec![Action::TogglePause, Action::Previous, Action::Next]
        );
    }

    #[test]
    fn connected_disconnected_and_unsupported_gamepad_events_are_handled() {
        assert_eq!(
            translate_event(EventType::Connected),
            Some(ControllerEvent::Connected)
        );
        assert_eq!(
            translate_event(EventType::Disconnected),
            Some(ControllerEvent::Disconnected)
        );
        assert_eq!(translate_event(EventType::Dropped), None);
    }

    #[test]
    fn event_polling_surfaces_hotplug_and_ignores_unsupported_buttons() {
        let events = [
            ControllerEvent::Connected,
            ControllerEvent::ButtonPressed(Button::DPadUp),
            ControllerEvent::ButtonPressed(Button::Unknown),
            ControllerEvent::Disconnected,
        ];
        let mut events = events.into_iter();
        let mut statuses = Vec::new();
        let mut actions = Vec::new();

        process_events(
            || events.next(),
            |status| statuses.push(status),
            |action| actions.push(action),
        );

        assert_eq!(
            statuses,
            ["Game controller connected", "Game controller disconnected"]
        );
        assert_eq!(actions, [ControllerAction::FocusPrevious]);
    }
}
