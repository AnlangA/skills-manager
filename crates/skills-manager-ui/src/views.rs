mod catalog;
mod create;
mod install;
mod inventory;
mod settings;

use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, text},
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

fn sidebar(app: &App) -> Element<'_, Message> {
    let nav = ActiveView::ALL.into_iter().fold(
        column![
            row![icons::icon(icons::SPARKLES, 19), text("Skills").size(18)]
                .spacing(10)
                .align_y(Alignment::Center),
        ]
        .spacing(12),
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
        column![nav.width(Length::Fill),]
            .spacing(16)
            .height(Length::Fill),
    )
    .width(Length::Fixed(176.0))
    .height(Length::Fill)
    .padding(16)
    .style(theme::sidebar)
    .into()
}

fn main_content(app: &App) -> Element<'_, Message> {
    let refresh = components::secondary_button("Refresh", Some(icons::REFRESH))
        .on_press_maybe((!app.busy).then_some(Message::Refresh));
    let header = row![
        text(app.active_view.label())
            .size(24)
            .color(theme::TEXT)
            .width(Length::Fill),
        components::status_badge(&app.status, app.busy).max_width(420),
        refresh,
    ]
    .spacing(14)
    .align_y(Alignment::Center);

    let body = match app.active_view {
        ActiveView::Inventory => inventory::view(app),
        ActiveView::Install => install::view(app),
        ActiveView::Create => create::view(app),
        ActiveView::Catalog => catalog::view(app),
        ActiveView::Settings => settings::view(app),
    };

    container(column![header, body].padding([16, 18]).spacing(12))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn nav_icon(view: ActiveView) -> &'static str {
    match view {
        ActiveView::Inventory => icons::LIST,
        ActiveView::Install => icons::DOWNLOAD,
        ActiveView::Create => icons::FILE,
        ActiveView::Catalog => icons::DATABASE,
        ActiveView::Settings => icons::SETTINGS,
    }
}
