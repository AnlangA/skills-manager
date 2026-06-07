use iced::{
    Alignment, Element, Length,
    widget::{button, checkbox, column, container, row, scrollable, text, text_input},
};
use skills_manager_core::{
    AgentToolTarget, ManagedResource, ResourceHealth, ResourceKind, SkillHealth, SkillScope,
};

use crate::theme::*;
use crate::{
    app::{App, HealthFilter, Message, PluginTargetFilter, SortKey},
    components, icons, theme,
};

const TARGET_ORDER: [AgentToolTarget; 3] = [
    AgentToolTarget::Codex,
    AgentToolTarget::ClaudeCode,
    AgentToolTarget::Generic,
];

pub fn view(app: &App) -> Element<'_, Message> {
    let plugins = filtered_plugins(app);
    let selected = selected_plugin(app, &plugins);

    let filters = filter_bar(app);

    let list = components::card(
        column![
            filters,
            table_header(app),
            scrollable(plugin_list(app, &plugins)).height(Length::Fill),
        ]
        .spacing(SPACING_MD),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    let inspector = plugin_inspector(
        selected,
        app.inventory.pending_remove_plugin.as_ref(),
        app.busy,
    )
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    column![
        row![list, inspector]
            .spacing(SPACING_LG)
            .height(Length::Fill)
    ]
    .spacing(SPACING_MD)
    .height(Length::Fill)
    .into()
}

fn filter_bar(app: &App) -> Element<'_, Message> {
    row![
        text_input("Filter plugins...", &app.inventory.search_query)
            .on_input(Message::SearchChanged)
            .padding([SPACING_SM, SPACING_MD])
            .style(theme::input)
            .width(Length::FillPortion(2)),
        container(components::styled_pick_list(
            &PluginTargetFilter::ALL,
            Some(app.inventory.plugin_target_filter),
            Message::PluginTargetFilterSelected,
            Length::Fill,
        ))
        .width(Length::FillPortion(1)),
        container(components::styled_pick_list(
            &HealthFilter::ALL,
            Some(app.inventory.health_filter),
            Message::HealthFilterSelected,
            Length::Fill,
        ))
        .width(Length::FillPortion(1)),
    ]
    .spacing(SPACING_MD)
    .align_y(Alignment::Center)
    .into()
}

fn table_header(app: &App) -> Element<'_, Message> {
    components::flat_card(
        row![
            text("").width(Length::Fixed(24.0)),
            sort_header("Plugin", SortKey::Name, app.inventory.sort_key).width(Length::Fill),
            sort_header("Health", SortKey::Health, app.inventory.sort_key),
            text("Actions").size(FONT_MICRO).color(TEXT_MUTED),
        ]
        .spacing(SPACING_MD)
        .align_y(Alignment::Center),
    )
    .into()
}

fn sort_header<'a>(
    label: &'a str,
    key: SortKey,
    current: SortKey,
) -> iced::widget::Button<'a, Message> {
    let active = key == current;
    button(
        text(if active {
            format!("{label} \u{2193}")
        } else {
            label.to_string()
        })
        .size(FONT_MICRO),
    )
    .padding([SPACING_XS, SPACING_SM])
    .style(theme::pill_button(active))
    .on_press(Message::SortSelected(key))
}

fn filtered_plugins(app: &App) -> Vec<&ManagedResource> {
    let query = app.inventory.search_query.trim().to_lowercase();
    let target_filter = app.inventory.plugin_target_filter;
    let health_filter = app.inventory.health_filter;
    app.resources
        .iter()
        .filter(|r| r.kind == ResourceKind::Plugin)
        .filter(|r| target_filter.matches(r.target))
        .filter(|r| {
            health_filter.matches(match r.health {
                ResourceHealth::Valid => SkillHealth::Valid,
                ResourceHealth::Warning => SkillHealth::Warning,
                ResourceHealth::Invalid => SkillHealth::Invalid,
            })
        })
        .filter(|r| {
            query.is_empty()
                || r.display_name.to_lowercase().contains(&query)
                || r.description
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&query)
                || r.root_dir
                    .display()
                    .to_string()
                    .to_lowercase()
                    .contains(&query)
        })
        .collect()
}

fn plugin_list<'a>(app: &'a App, plugins: &[&'a ManagedResource]) -> Element<'a, Message> {
    if plugins.is_empty() {
        return components::empty_state(
            "No plugins found",
            "Try adjusting your search or filters, or add plugin marketplaces.",
        )
        .into();
    }

    let targets = active_targets(plugins);
    components::list_column(targets, SPACING_LG, |target| {
        let scoped: Vec<&ManagedResource> = plugins
            .iter()
            .copied()
            .filter(|p| p.target == target)
            .collect();
        target_section(app, target, scoped)
    })
    .into()
}

fn active_targets(plugins: &[&ManagedResource]) -> Vec<AgentToolTarget> {
    let mut targets = Vec::new();
    for target in TARGET_ORDER {
        if plugins.iter().any(|p| p.target == target) {
            targets.push(target);
        }
    }
    for plugin in plugins {
        if !targets.contains(&plugin.target) {
            targets.push(plugin.target);
        }
    }
    targets
}

fn target_section<'a>(
    app: &'a App,
    target: AgentToolTarget,
    plugins: Vec<&'a ManagedResource>,
) -> Element<'a, Message> {
    let count = plugins.len();
    let rows = components::list_column(plugins, SPACING_SM, |plugin| {
        let selected = app
            .inventory
            .selected_resource_id
            .as_ref()
            .is_some_and(|id| id == &plugin.id);
        let pending = app
            .inventory
            .pending_remove_plugin
            .as_ref()
            .is_some_and(|id| id == &plugin.id);
        plugin_row(plugin, selected, pending, app.busy)
    });

    column![
        row![
            target_chip(target),
            text(format!("{} plugin(s)", count))
                .size(FONT_CAPTION)
                .color(TEXT_MUTED),
        ]
        .spacing(SPACING_SM)
        .align_y(Alignment::Center),
        rows,
    ]
    .spacing(SPACING_SM)
    .into()
}

fn plugin_row<'a>(
    plugin: &'a ManagedResource,
    selected: bool,
    pending: bool,
    busy: bool,
) -> Element<'a, Message> {
    let enabled = plugin.enablement.is_enabled();
    let toggle = checkbox(enabled)
        .size(18)
        .on_toggle_maybe((!busy).then_some({
            let id = plugin_plugin_id(plugin);
            let target = plugin.target;
            move |checked| Message::SetPluginEnabled(id.clone(), target, checked)
        }));
    let select = components::small_ghost_button("View", Some(icons::EYE))
        .on_press(Message::SelectResource(plugin.id.clone()));
    let remove = components::confirm_button(
        pending,
        "Remove",
        "Confirm",
        Some(icons::TRASH),
        Message::RequestRemovePlugin(plugin.id.clone(), plugin.target),
        Message::ConfirmRemovePlugin(plugin.id.clone(), plugin.target),
        busy,
    );

    container(
        row![
            toggle,
            column![
                text(&plugin.display_name).size(FONT_BODY).color(TEXT),
                text(format!(
                    "{} - {}",
                    plugin.target.label(),
                    plugin.root_dir.display()
                ))
                .size(FONT_MICRO)
                .color(TEXT_MUTED)
                .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(SPACING_XS)
            .width(Length::Fill),
            resource_health_dot(plugin.health),
            row![select, remove]
                .spacing(SPACING_SM)
                .align_y(Alignment::Center),
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

fn plugin_inspector<'a>(
    plugin: Option<&'a ManagedResource>,
    pending_remove_plugin: Option<&'a String>,
    busy: bool,
) -> iced::widget::Container<'a, Message> {
    match plugin {
        Some(plugin) => components::card(
            scrollable(
                column![
                    row![
                        target_chip(plugin.target),
                        components::enablement_chip(plugin.enablement),
                        resource_health_dot(plugin.health),
                    ]
                    .spacing(SPACING_SM)
                    .align_y(Alignment::Center),
                    text(&plugin.display_name).size(FONT_DISPLAY).color(TEXT),
                    inspector_section("OVERVIEW", overview_section(plugin)),
                    inspector_section("COMPONENTS", components_section(plugin)),
                    inspector_section("DIAGNOSTICS", diagnostics_section(plugin)),
                    inspector_section(
                        "ACTIONS",
                        actions_section(plugin, pending_remove_plugin, busy),
                    ),
                ]
                .spacing(SPACING_LG),
            )
            .height(Length::Fill),
        ),
        None => components::card(
            column![
                components::section_header("Inspector", "No selection"),
                components::empty_state(
                    "No plugin selected",
                    "Select a plugin to inspect manifest details and actions.",
                ),
            ]
            .spacing(SPACING_MD),
        ),
    }
}

fn inspector_section<'a>(label: &'a str, content: Element<'a, Message>) -> Element<'a, Message> {
    column![components::section_label(label), content]
        .spacing(SPACING_SM)
        .into()
}

fn overview_section<'a>(plugin: &'a ManagedResource) -> Element<'a, Message> {
    column![
        components::detail_row(
            "Description",
            plugin.description.as_deref().unwrap_or("No description"),
        ),
        components::detail_row("Source", plugin.source_url.as_deref().unwrap_or("Unknown"),),
        components::detail_row("Root", plugin.root_dir.display().to_string()),
        components::detail_row(
            "Manifest",
            plugin
                .manifest_file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "N/A".to_string()),
        ),
    ]
    .spacing(SPACING_SM)
    .into()
}

fn components_section<'a>(plugin: &'a ManagedResource) -> Element<'a, Message> {
    components::text_lines(
        plugin
            .metadata
            .iter()
            .map(|(key, value)| format!("{key}: {value}")),
        "No component metadata",
        TEXT_SECONDARY,
        FONT_CAPTION,
    )
}

fn diagnostics_section<'a>(plugin: &'a ManagedResource) -> Element<'a, Message> {
    components::diagnostic_lines(&plugin.diagnostics, "No diagnostics")
}

fn actions_section<'a>(
    plugin: &'a ManagedResource,
    pending_remove_plugin: Option<&'a String>,
    busy: bool,
) -> Element<'a, Message> {
    let toggle_label = if plugin.enablement.is_enabled() {
        "Disable"
    } else {
        "Enable"
    };
    let toggle_icon = if plugin.enablement.is_enabled() {
        icons::EYE_OFF
    } else {
        icons::EYE
    };
    let pending = pending_remove_plugin.is_some_and(|id| id == &plugin.id);
    let remove = components::confirm_button(
        pending,
        "Remove",
        "Confirm remove",
        Some(icons::TRASH),
        Message::RequestRemovePlugin(plugin.id.clone(), plugin.target),
        Message::ConfirmRemovePlugin(plugin.id.clone(), plugin.target),
        busy,
    );

    row![
        components::primary_button(toggle_label, Some(toggle_icon)).on_press_maybe(
            (!busy).then_some(Message::SetPluginEnabled(
                plugin_plugin_id(plugin),
                plugin.target,
                !plugin.enablement.is_enabled(),
            ))
        ),
        remove,
    ]
    .spacing(SPACING_MD)
    .align_y(Alignment::Center)
    .into()
}

fn plugin_plugin_id(plugin: &ManagedResource) -> String {
    plugin
        .metadata
        .get("plugin_id")
        .cloned()
        .unwrap_or_else(|| plugin.id.clone())
}

fn selected_plugin<'a>(
    app: &'a App,
    plugins: &[&'a ManagedResource],
) -> Option<&'a ManagedResource> {
    app.inventory
        .selected_resource_id
        .as_ref()
        .and_then(|id| plugins.iter().copied().find(|plugin| plugin.id == *id))
        .or_else(|| plugins.first().copied())
}

fn resource_health_dot<'a>(health: ResourceHealth) -> Element<'a, Message> {
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
        AgentToolTarget::Generic => components::scope_chip(SkillScope::Custom),
    }
}
