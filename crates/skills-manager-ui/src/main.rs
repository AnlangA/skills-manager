mod app;
mod components;
mod icons;
mod tasks;
mod theme;
mod views;

pub fn main() -> iced::Result {
    let smoke_test = std::env::args().any(|arg| arg == "--smoke-test");
    if smoke_test {
        return Ok(());
    }

    iced::application(
        move || {
            if smoke_test {
                app::App::init_with_smoke_test(true)
            } else {
                app::App::init()
            }
        },
        app::update,
        views::view,
    )
    .title("Agent Skills Manager")
    .theme(theme::app_theme())
    .subscription(subscription)
    .font(iced_lucide::FONT_BYTES)
    .run()
}

fn subscription(app: &app::App) -> iced::Subscription<app::Message> {
    use std::time::Duration;
    if app.smoke_test {
        iced::time::every(Duration::from_millis(250)).map(|_| app::Message::SmokeExit)
    } else {
        iced::Subscription::none()
    }
}
