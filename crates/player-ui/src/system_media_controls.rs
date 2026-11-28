use crate::Action;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MediaCommand {
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
    SeekTo(f64),
    SeekBy(f64),
}

pub fn action_for_command(command: MediaCommand) -> Option<Action> {
    match command {
        MediaCommand::Play => Some(Action::SetPaused(false)),
        MediaCommand::Pause => Some(Action::SetPaused(true)),
        MediaCommand::Toggle => Some(Action::TogglePause),
        MediaCommand::Stop => Some(Action::Stop),
        MediaCommand::Next => Some(Action::Next),
        MediaCommand::Previous => Some(Action::Previous),
        MediaCommand::SeekTo(position) if position.is_finite() && position >= 0.0 => {
            Some(Action::SeekAbsolute(position))
        }
        MediaCommand::SeekTo(_) => None,
        MediaCommand::SeekBy(offset) if offset.is_finite() => Some(Action::Seek(offset)),
        MediaCommand::SeekBy(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{MediaCommand, action_for_command};
    use crate::Action;

    #[test]
    fn operating_system_play_resumes_without_toggling() {
        assert_eq!(
            action_for_command(MediaCommand::Play),
            Some(Action::SetPaused(false))
        );
        assert_eq!(
            action_for_command(MediaCommand::Play),
            Some(Action::SetPaused(false))
        );
    }

    #[test]
    fn operating_system_transport_commands_use_the_existing_player_actions() {
        assert_eq!(
            action_for_command(MediaCommand::Pause),
            Some(Action::SetPaused(true))
        );
        assert_eq!(
            action_for_command(MediaCommand::Toggle),
            Some(Action::TogglePause)
        );
        assert_eq!(action_for_command(MediaCommand::Stop), Some(Action::Stop));
        assert_eq!(action_for_command(MediaCommand::Next), Some(Action::Next));
        assert_eq!(
            action_for_command(MediaCommand::Previous),
            Some(Action::Previous)
        );
        assert_eq!(
            action_for_command(MediaCommand::SeekTo(12.5)),
            Some(Action::SeekAbsolute(12.5))
        );
        assert_eq!(
            action_for_command(MediaCommand::SeekTo(0.0)),
            Some(Action::SeekAbsolute(0.0))
        );
        assert_eq!(action_for_command(MediaCommand::SeekTo(-1.0)), None);
        assert_eq!(action_for_command(MediaCommand::SeekTo(f64::NAN)), None);
        assert_eq!(
            action_for_command(MediaCommand::SeekTo(f64::INFINITY)),
            None
        );
        assert_eq!(
            action_for_command(MediaCommand::SeekBy(-5.0)),
            Some(Action::Seek(-5.0))
        );
        assert_eq!(action_for_command(MediaCommand::SeekBy(f64::NAN)), None);
        assert_eq!(
            action_for_command(MediaCommand::SeekBy(f64::INFINITY)),
            None
        );
    }
}
