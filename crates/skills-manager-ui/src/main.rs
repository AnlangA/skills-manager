mod app;
mod components;
mod icons;
mod tasks;
mod theme;
mod views;

pub fn main() -> iced::Result {
    iced::application(app::App::init, app::update, app::view)
        .title("Agent Skills Manager")
        .theme(theme::app_theme())
        .font(iced_lucide::FONT_BYTES)
        .run()
}
