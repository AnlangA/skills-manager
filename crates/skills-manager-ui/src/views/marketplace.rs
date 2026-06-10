//! Marketplace view for managing sources, searching providers, and inspecting documents.
//!
//! Provides forms for adding configured marketplace sources, a search panel
//! for querying remote providers like SkillsMP, and an inspector for
//! viewing loaded marketplace document entries and diagnostics.

use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, scrollable, text, text_input},
};
use skills_manager_core::{
    AgentToolTarget, ManagedResource, ResourceHealth, SkillHealth, SkillScope,
};

use crate::theme::*;
use crate::{
    app::{App, Message, filtered_marketplace_indices},
    components, icons, theme,
};

pub fn view(app: &App) -> Element<'_, Message> {
    let marketplaces = filtered_marketplaces(app);
    let selected = selected_marketplace(app, &marketplaces);

    let list = components::card(
        column![
            filter_bar(app),
            scrollable(marketplace_list(app, &marketplaces)).height(Length::Fill),
        ]
        .spacing(SPACING_MD),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    let inspector = marketplace_inspector(selected)
        .width(Length::FillPortion(2))
        .height(Length::Fill);

    row![list, inspector]
        .spacing(SPACING_LG)
        .height(Length::Fill)
        .into()
}

fn filter_bar(app: &App) -> Element<'_, Message> {
    row![
        text_input(
            "Filter marketplaces...",
            &app.inventory.marketplace_search_query
        )
        .on_input(Message::MarketplaceSearchChanged)
        .padding([SPACING_SM, SPACING_MD])
        .style(theme::input)
        .width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn filtered_marketplaces(app: &App) -> Vec<&ManagedResource> {
    filtered_marketplace_indices(
        &app.resources,
        &app.derived.resource_search,
        &app.inventory.marketplace_search_query,
    )
    .into_iter()
    .filter_map(|index| app.resources.get(index))
    .collect()
}

fn selected_marketplace<'a>(
    app: &'a App,
    marketplaces: &[&'a ManagedResource],
) -> Option<&'a ManagedResource> {
    app.inventory
        .selected_resource_id
        .as_ref()
        .and_then(|id| {
            marketplaces
                .iter()
                .copied()
                .find(|marketplace| marketplace.id == *id)
        })
        .or_else(|| marketplaces.first().copied())
}

fn marketplace_list<'a>(
    app: &'a App,
    marketplaces: &[&'a ManagedResource],
) -> Element<'a, Message> {
    if marketplaces.is_empty() {
        return components::empty_state(
            "No marketplaces found",
            "Add or refresh marketplace sources to discover marketplace manifests.",
        )
        .into();
    }

    components::list_column(marketplaces.iter().copied(), SPACING_SM, |marketplace| {
        marketplace_row(app, marketplace)
    })
    .into()
}

fn marketplace_row<'a>(app: &'a App, marketplace: &'a ManagedResource) -> Element<'a, Message> {
    let selected = app
        .inventory
        .selected_resource_id
        .as_ref()
        .is_some_and(|id| id == &marketplace.id);
    let select = components::small_ghost_button("View", Some(icons::EYE))
        .on_press(Message::SelectResource(marketplace.id.clone()));

    container(
        row![
            column![
                text(&marketplace.display_name).size(FONT_BODY).color(TEXT),
                text(format!(
                    "{} - {}",
                    marketplace.target.label(),
                    marketplace.root_dir.display()
                ))
                .size(FONT_MICRO)
                .color(TEXT_MUTED)
                .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(SPACING_XS)
            .width(Length::Fill),
            marketplace_health(marketplace.health),
            select,
        ]
        .spacing(SPACING_MD)
        .align_y(Alignment::Center),
    )
    .padding([SPACING_MD, SPACING_MD + 2.0])
    .style(if selected {
        theme::selected_card
    } else {
        theme::card
    })
    .into()
}

fn marketplace_inspector<'a>(
    marketplace: Option<&'a ManagedResource>,
) -> iced::widget::Container<'a, Message> {
    match marketplace {
        Some(marketplace) => components::card(
            scrollable(
                column![
                    row![
                        target_chip(marketplace.target),
                        marketplace_health(marketplace.health),
                    ]
                    .spacing(SPACING_SM)
                    .align_y(Alignment::Center),
                    text(&marketplace.display_name)
                        .size(FONT_DISPLAY)
                        .color(TEXT),
                    components::detail_section(
                        "OVERVIEW",
                        column![
                            components::detail_row(
                                "Description",
                                marketplace
                                    .description
                                    .as_deref()
                                    .unwrap_or("No description"),
                            ),
                            components::detail_row(
                                "Source",
                                marketplace.source_url.as_deref().unwrap_or("Unknown"),
                            ),
                            components::detail_row(
                                "Root",
                                marketplace.root_dir.display().to_string(),
                            ),
                            components::detail_row(
                                "Manifest",
                                marketplace
                                    .manifest_file
                                    .as_ref()
                                    .map(|path| path.display().to_string())
                                    .unwrap_or_else(|| "N/A".to_string()),
                            ),
                        ]
                        .spacing(SPACING_SM),
                    ),
                    components::detail_section(
                        "METADATA",
                        components::text_lines(
                            marketplace
                                .metadata
                                .iter()
                                .map(|(key, value)| format!("{key}: {value}")),
                            "No metadata",
                            TEXT_SECONDARY,
                            FONT_CAPTION,
                        ),
                    ),
                    components::detail_section(
                        "DIAGNOSTICS",
                        components::diagnostic_lines(&marketplace.diagnostics, "No diagnostics"),
                    ),
                ]
                .spacing(SPACING_LG),
            )
            .height(Length::Fill),
        ),
        None => components::card(components::empty_state(
            "No marketplace selected",
            "Select a marketplace to inspect its manifest and diagnostics.",
        )),
    }
}

fn marketplace_health<'a>(health: ResourceHealth) -> Element<'a, Message> {
    components::health_dot(match health {
        ResourceHealth::Valid => SkillHealth::Valid,
        ResourceHealth::Warning => SkillHealth::Warning,
        ResourceHealth::Invalid => SkillHealth::Invalid,
    })
}

fn target_chip<'a>(target: AgentToolTarget) -> iced::widget::Container<'a, Message> {
    match target {
        AgentToolTarget::Codex => components::scope_chip(SkillScope::Codex),
        AgentToolTarget::ClaudeCode => components::scope_chip(SkillScope::ClaudeCode),
        AgentToolTarget::Droid => components::scope_chip(SkillScope::Droid),
        AgentToolTarget::OpenCode => components::scope_chip(SkillScope::OpenCode),
        AgentToolTarget::Zed => components::scope_chip(SkillScope::Zed),
        AgentToolTarget::Generic => components::scope_chip(SkillScope::Custom),
    }
}
