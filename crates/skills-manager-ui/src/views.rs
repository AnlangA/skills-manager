//! Screen composition and layout for the desktop UI.
//!
//! Contains the top-level `view` function, sidebar navigation, and
//! sub-modules for each screen (library, plugins, install, create,
//! marketplace, catalog, and targets/settings).

mod catalog;
mod create;
mod install;
mod inventory;
mod marketplace;
mod mcp;
mod plugins;
mod settings;

use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, text},
};
use skills_manager_core::{EnablementStrategy, LayoutPolicy};

use crate::theme::*;
use crate::{app::ActiveView, app::App, app::Message, components, icons, theme};

/// Builds the top-level view with sidebar and main content area.
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

fn sidebar(app: &App) -> Element<'_, Message> {
    let workspace_label = components::section_label("WORKSPACE");
    let workspace_nav = column![
        components::nav_button(
            ActiveView::Library.label(),
            nav_icon(ActiveView::Library),
            app.active_view == ActiveView::Library,
            Message::ActiveViewSelected(ActiveView::Library),
        ),
        components::nav_button(
            ActiveView::Plugins.label(),
            nav_icon(ActiveView::Plugins),
            app.active_view == ActiveView::Plugins,
            Message::ActiveViewSelected(ActiveView::Plugins),
        ),
        components::nav_button(
            ActiveView::Mcp.label(),
            nav_icon(ActiveView::Mcp),
            app.active_view == ActiveView::Mcp,
            Message::ActiveViewSelected(ActiveView::Mcp),
        ),
        components::nav_button(
            ActiveView::Marketplace.label(),
            nav_icon(ActiveView::Marketplace),
            app.active_view == ActiveView::Marketplace,
            Message::ActiveViewSelected(ActiveView::Marketplace),
        ),
        components::nav_button(
            ActiveView::Install.label(),
            nav_icon(ActiveView::Install),
            app.active_view == ActiveView::Install,
            Message::ActiveViewSelected(ActiveView::Install),
        ),
        components::nav_button(
            ActiveView::Create.label(),
            nav_icon(ActiveView::Create),
            app.active_view == ActiveView::Create,
            Message::ActiveViewSelected(ActiveView::Create),
        ),
    ]
    .spacing(SPACING_XS);

    let manage_label = components::section_label("MANAGE");
    let manage_nav = column![
        components::nav_button(
            ActiveView::Catalog.label(),
            nav_icon(ActiveView::Catalog),
            app.active_view == ActiveView::Catalog,
            Message::ActiveViewSelected(ActiveView::Catalog),
        ),
        components::nav_button(
            ActiveView::Targets.label(),
            nav_icon(ActiveView::Targets),
            app.active_view == ActiveView::Targets,
            Message::ActiveViewSelected(ActiveView::Targets),
        ),
    ]
    .spacing(SPACING_XS);

    let project_path = if app.settings.project_path.is_empty() {
        "No project".to_string()
    } else {
        std::path::Path::new(&app.settings.project_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project")
            .to_string()
    };

    container(
        column![
            row![
                icons::icon(icons::SPARKLES, 20),
                text("Skills Manager").size(FONT_HEADING)
            ]
            .spacing(SPACING_SM)
            .align_y(Alignment::Center),
            column![workspace_label, workspace_nav].spacing(SPACING_SM),
            column![manage_label, manage_nav].spacing(SPACING_SM),
            column![].height(Length::Fill),
            text(project_path)
                .size(FONT_MICRO)
                .color(TEXT_MUTED)
                .wrapping(text::Wrapping::WordOrGlyph),
        ]
        .spacing(SPACING_LG)
        .height(Length::Fill),
    )
    .width(Length::Fixed(180.0))
    .height(Length::Fill)
    .padding(SPACING_LG)
    .style(theme::sidebar)
    .into()
}

fn main_content(app: &App) -> Element<'_, Message> {
    let refresh = components::secondary_button("Refresh", Some(icons::REFRESH))
        .on_press_maybe((!app.busy).then_some(Message::Refresh));

    let status_indicator: Element<'_, Message> = if app.busy {
        row![
            iced_aw::Spinner::new(),
            text(&app.status).size(FONT_CAPTION).color(TEXT_SECONDARY),
        ]
        .spacing(SPACING_SM)
        .align_y(Alignment::Center)
        .into()
    } else {
        text(&app.status)
            .size(FONT_CAPTION)
            .color(TEXT_MUTED)
            .into()
    };

    let header = row![
        text(app.active_view.label()).size(FONT_DISPLAY).color(TEXT),
        column![].width(Length::Fill),
        status_indicator,
        refresh,
    ]
    .spacing(SPACING_MD)
    .align_y(Alignment::Center);

    let body = match app.active_view {
        ActiveView::Library => inventory::view(app),
        ActiveView::Plugins => plugins::view(app),
        ActiveView::Mcp => mcp::view(app),
        ActiveView::Install => install::view(app),
        ActiveView::Create => create::view(app),
        ActiveView::Marketplace => marketplace::view(app),
        ActiveView::Catalog => catalog::view(app),
        ActiveView::Targets => settings::view(app),
    };

    container(
        column![header, body]
            .padding([SPACING_LG, SPACING_XL])
            .spacing(SPACING_LG),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn nav_icon(view: ActiveView) -> &'static str {
    match view {
        ActiveView::Library => icons::LIST,
        ActiveView::Plugins => icons::DATABASE,
        ActiveView::Mcp => icons::SETTINGS,
        ActiveView::Install => icons::DOWNLOAD,
        ActiveView::Create => icons::FILE,
        ActiveView::Marketplace => icons::SEARCH,
        ActiveView::Catalog => icons::DATABASE,
        ActiveView::Targets => icons::SETTINGS,
    }
}

/// Returns a human-readable label for the given enablement strategy.
pub(super) fn strategy_label(strategy: EnablementStrategy) -> &'static str {
    match strategy {
        EnablementStrategy::ConfigToggle => "Config toggle",
        EnablementStrategy::DirectoryMove => "Directory move",
    }
}

/// Returns a human-readable label for the given layout policy.
pub(super) fn layout_label(layout: LayoutPolicy) -> &'static str {
    match layout {
        LayoutPolicy::NestedAllowed => "Nested resources allowed",
        LayoutPolicy::FlatTopLevel => "Flat top-level folder",
    }
}
