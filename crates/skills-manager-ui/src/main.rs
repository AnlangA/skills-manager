mod app;
mod components;
mod icons;
mod tasks;
mod theme;
mod views;

pub fn main() -> iced::Result {
    let smoke_test = std::env::args().any(|arg| arg == "--smoke-test");

    iced::application(
        move || {
            if smoke_test {
                app::App::init_with_smoke_test(true)
            } else {
                app::App::init()
            }
        },
        app::update,
        app::view,
    )
    .title("Agent Skills Manager")
    .theme(theme::app_theme())
    .subscription(app::subscription)
    .font(iced_lucide::FONT_BYTES)
    .run()
}
