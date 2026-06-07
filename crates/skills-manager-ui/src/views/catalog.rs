use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, scrollable, text},
};

use crate::theme::*;
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

    let output: Element<'_, Message> = if app.catalog.catalog_output.is_empty() {
        components::empty_state(
            "No export generated",
            "Generate a catalog to preview JSON, XML, or Markdown output.",
        )
        .into()
    } else {
        container(
            scrollable(
                text(&app.catalog.catalog_output)
                    .size(FONT_CAPTION)
                    .color(TEXT)
                    .wrapping(text::Wrapping::WordOrGlyph),
            )
            .height(Length::Fill),
        )
        .padding(SPACING_LG)
        .style(theme::flat_card)
        .into()
    };

    components::card(
        column![
            components::section_header(
                "Catalog Export",
                format!("{exportable} exportable skill(s)")
            ),
            row![
                components::styled_pick_list(
                    &UiCatalogFormat::ALL,
                    Some(app.catalog.catalog_format),
                    Message::CatalogFormatSelected,
                    Length::Fixed(140.0),
                ),
                components::primary_button("Generate", Some(icons::FILE))
                    .on_press_maybe((!app.busy).then_some(Message::GenerateCatalog)),
                components::secondary_button("Copy", Some(icons::COPY)).on_press_maybe(
                    (!app.catalog.catalog_output.is_empty()).then_some(Message::CopyCatalog)
                ),
                components::secondary_button("Save", Some(icons::DOWNLOAD)).on_press_maybe(
                    (!app.busy && !app.catalog.catalog_output.is_empty())
                        .then_some(Message::SaveCatalog)
                ),
            ]
            .spacing(SPACING_MD)
            .align_y(Alignment::Center),
            components::compact_field(
                "Save path",
                "agent-skills-catalog.json",
                &app.catalog.catalog_save_path,
                Message::CatalogSavePathChanged,
            ),
            output,
        ]
        .spacing(SPACING_LG),
    )
    .height(Length::Fill)
    .into()
}
