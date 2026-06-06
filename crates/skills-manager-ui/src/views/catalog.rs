use iced::{
    Alignment, Element, Length,
    widget::{column, container, pick_list, row, scrollable, text, text_input},
};

use crate::{
    app::{App, Message, UiCatalogFormat},
    components, icons, theme,
};

pub fn view(app: &App) -> Element<'_, Message> {
    let exportable = app
        .skills
        .iter()
        .filter(|skill| skill.is_exportable())
        .count();
    let output: Element<'_, Message> = if app.catalog_output.is_empty() {
        components::empty_state(
            "No export generated",
            "Generate a catalog to preview JSON, XML, or Markdown output.",
        )
        .into()
    } else {
        container(
            scrollable(
                text(&app.catalog_output)
                    .size(12)
                    .color(theme::TEXT)
                    .wrapping(text::Wrapping::WordOrGlyph),
            )
            .height(Length::Fill),
        )
        .padding(12)
        .style(theme::flat_panel)
        .into()
    };

    components::panel(
        column![
            components::section_header(
                "Catalog Export",
                format!("{exportable} exportable skill(s)")
            ),
            row![
                pick_list(
                    UiCatalogFormat::ALL,
                    Some(app.catalog_format),
                    Message::CatalogFormatSelected
                )
                .padding([9, 12])
                .style(theme::select)
                .width(Length::Fixed(160.0)),
                components::primary_button("Generate", Some(icons::FILE))
                    .on_press_maybe((!app.busy).then_some(Message::GenerateCatalog)),
                components::secondary_button("Copy", Some(icons::COPY)).on_press_maybe(
                    (!app.catalog_output.is_empty()).then_some(Message::CopyCatalog)
                ),
                components::secondary_button("Save", Some(icons::DOWNLOAD)).on_press_maybe(
                    (!app.busy && !app.catalog_output.is_empty()).then_some(Message::SaveCatalog)
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text_input("Save path", &app.catalog_save_path)
                .on_input(Message::CatalogSavePathChanged)
                .padding([10, 12])
                .style(theme::input),
            output,
        ]
        .spacing(12),
    )
    .height(Length::Fill)
    .into()
}
