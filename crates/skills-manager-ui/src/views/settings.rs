use iced::{
    Alignment, Element, Length,
    widget::{column, row, text, text_input},
};

use crate::{app::App, app::Message, components, icons, theme};

pub fn view(app: &App) -> Element<'_, Message> {
    let counts = app.counts();

    components::panel(
        column![
            components::section_header("Settings", "Open Agent Skills convention"),
            text_input("Project folder", &app.project_path)
                .on_input(Message::ProjectPathChanged)
                .padding([10, 12])
                .style(theme::input),
            row![
                components::metric("Project skills", counts.project.to_string(), theme::PRIMARY),
                components::metric("User skills", counts.user.to_string(), theme::CYAN),
                components::metric("Exportable", counts.exportable.to_string(), theme::SUCCESS),
            ]
            .spacing(8),
            components::flat_panel(
                column![
                    row![
                        icons::icon(icons::FOLDER, 16),
                        text("Project scope").size(14)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    text("<project>/.agents/skills")
                        .size(13)
                        .color(theme::MUTED),
                    text("Project skills take priority and can shadow user skills with the same name.")
                        .size(12)
                        .color(theme::SUBTLE),
                ]
                .spacing(6),
            ),
            components::flat_panel(
                column![
                    row![icons::icon(icons::GLOBE, 16), text("User scope").size(14)]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    text("~/.agents/skills").size(13).color(theme::MUTED),
                    text("User skills are available across projects unless a project skill shadows them.")
                        .size(12)
                        .color(theme::SUBTLE),
                ]
                .spacing(6),
            ),
            components::flat_panel(
                column![
                    row![
                        icons::icon(icons::SHIELD, 16),
                        text("Validation").size(14)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    text("The scanner checks required frontmatter, name shape, description length, compatibility notes, resources, and shadowing.")
                        .size(13)
                        .color(theme::MUTED)
                        .wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(6),
            ),
        ]
        .spacing(12),
    )
    .height(Length::Fill)
    .into()
}
