mod catalog;
mod install;
mod inventory;
mod settings;

use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, rule, text},
};

use crate::{app::ActiveView, app::App, app::Message, components, icons, theme};

pub fn view(app: &App) -> Element<'_, Message> {
    let content = row![sidebar(app), main_content(app)]
        .height(Length::Fill)
        .width(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::app_background)
        .into()
}

pub(super) fn detail_row<'a>(label: &'a str, value: String) -> iced::widget::Column<'a, Message> {
    column![
        text(label).size(11).color(theme::SUBTLE),
        text(value)
            .size(13)
            .color(theme::TEXT)
            .wrapping(text::Wrapping::WordOrGlyph),
    ]
    .spacing(3)
}

pub(super) fn diagnostics_text<'a>(diagnostics: &'a [String]) -> Element<'a, Message> {
    if diagnostics.is_empty() {
        return text("No diagnostics").size(12).color(theme::SUBTLE).into();
    }

    diagnostics
        .iter()
        .fold(column![].spacing(3), |list, diagnostic| {
            list.push(text(format!("- {diagnostic}")).size(12).color(theme::MUTED))
        })
        .into()
}

pub(super) fn source_summary(source: &str) -> &str {
    if source.contains("github.com") {
        "GitHub source"
    } else if source.trim().is_empty() {
        "unknown source"
    } else {
        "known source"
    }
}

fn sidebar(app: &App) -> Element<'_, Message> {
    let nav = ActiveView::ALL.into_iter().fold(
        column![
            row![
                icons::icon(icons::SPARKLES, 19),
                text("Agent Skills").size(18)
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text("Open local skill library")
                .size(12)
                .color(iced::Color::from_rgb8(203, 213, 225)),
            rule::horizontal(1),
        ]
        .spacing(10),
        |nav, view| {
            nav.push(components::nav_button(
                view.label(),
                nav_icon(view),
                app.active_view == view,
                Message::ActiveViewSelected(view),
            ))
        },
    );

    container(
        column![
            nav.width(Length::Fill),
            container(column![
                text("Open convention").size(12).color(iced::Color::WHITE),
                text("~/.agents/skills\n<project>/.agents/skills")
                    .size(11)
                    .color(iced::Color::from_rgb8(203, 213, 225)),
            ])
            .padding(10)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb8(31, 41, 55))),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..iced::Border::default()
                },
                ..iced::widget::container::Style::default()
            }),
        ]
        .spacing(16)
        .height(Length::Fill),
    )
    .width(Length::Fixed(220.0))
    .height(Length::Fill)
    .padding(16)
    .style(theme::sidebar)
    .into()
}

fn main_content(app: &App) -> Element<'_, Message> {
    let refresh = components::secondary_button("Refresh", Some(icons::REFRESH))
        .on_press_maybe((!app.busy).then_some(Message::Refresh));
    let header = row![
        column![
            text(app.active_view.label()).size(26).color(theme::TEXT),
            text(header_subtitle(app.active_view))
                .size(13)
                .color(theme::MUTED),
        ]
        .spacing(3)
        .width(Length::Fill),
        components::status_badge(&app.status, app.busy).max_width(460),
        refresh,
    ]
    .spacing(14)
    .align_y(Alignment::Center);

    let body = match app.active_view {
        ActiveView::Inventory => inventory::view(app),
        ActiveView::Install => install::view(app),
        ActiveView::Catalog => catalog::view(app),
        ActiveView::Settings => settings::view(app),
    };

    container(column![header, body].padding(18).spacing(14))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn header_subtitle(view: ActiveView) -> &'static str {
    match view {
        ActiveView::Inventory => "Search, validate, enable, disable, and inspect local skills",
        ActiveView::Install => "Preview GitHub, local, or catalog sources before installing",
        ActiveView::Catalog => "Export enabled and usable skills for agent runtimes",
        ActiveView::Settings => "Project path and open Agent Skills storage rules",
    }
}

fn nav_icon(view: ActiveView) -> &'static str {
    match view {
        ActiveView::Inventory => icons::LIST,
        ActiveView::Install => icons::DOWNLOAD,
        ActiveView::Catalog => icons::DATABASE,
        ActiveView::Settings => icons::SETTINGS,
    }
}
