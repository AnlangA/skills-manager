//! MCP server management view.
//!
//! Renders installed MCP servers grouped by agent target, with filters,
//! enable/disable controls, removal confirmation, and an inspector.

use iced::{
    Alignment, Element, Length,
    widget::{button, checkbox, column, container, row, scrollable, text, text_input},
};
use skills_manager_core::{
    AgentToolTarget, ManagedResource, ResourceHealth, SkillHealth, SkillScope,
};

use crate::theme::*;
use crate::{
    app::{App, HealthFilter, Message, PluginTargetFilter, SortKey, filtered_mcp_indices},
    components, icons, theme,
};

const TARGET_ORDER: [AgentToolTarget; 5] = [
    AgentToolTarget::Codex,
    AgentToolTarget::ClaudeCode,
    AgentToolTarget::Droid,
    AgentToolTarget::OpenCode,
    AgentToolTarget::Zed,
];

pub fn view(app: &App) -> Element<'_, Message> {
    let servers = filtered_mcp_servers(app);
    let selected = selected_mcp_server(app, &servers);

    let left = components::card(
        column![
            filter_bar(app),
            table_header(app),
            scrollable(mcp_list(app, &servers)).height(Length::Fill),
        ]
        .spacing(SPACING_MD),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    let inspector = mcp_inspector(selected, app.mcp.pending_remove.as_ref(), app.busy)
        .width(Length::FillPortion(2))
        .height(Length::Fill);

    row![left, inspector]
        .spacing(SPACING_LG)
        .height(Length::Fill)
        .into()
}

fn filter_bar(app: &App) -> Element<'_, Message> {
    row![
        text_input("Filter MCP servers...", &app.mcp.search_query)
            .on_input(Message::McpSearchChanged)
            .padding([SPACING_SM, SPACING_MD])
            .style(theme::input)
            .width(Length::FillPortion(2)),
        container(components::styled_pick_list(
            &PluginTargetFilter::ALL,
            Some(app.mcp.target_filter),
            Message::McpTargetFilterSelected,
            Length::Fill,
        ))
        .width(Length::FillPortion(1)),
        container(components::styled_pick_list(
            &HealthFilter::ALL,
            Some(app.mcp.health_filter),
            Message::McpHealthFilterSelected,
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
            sort_header("Server", SortKey::Name, app.inventory.sort_key).width(Length::Fill),
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

fn filtered_mcp_servers(app: &App) -> Vec<&ManagedResource> {
    filtered_mcp_indices(
        &app.resources,
        &app.derived.resource_search,
        &app.mcp.search_query,
        app.mcp.target_filter,
        app.mcp.health_filter,
        app.inventory.sort_key,
    )
    .into_iter()
    .filter_map(|index| app.resources.get(index))
    .collect()
}

fn mcp_list<'a>(app: &'a App, servers: &[&'a ManagedResource]) -> Element<'a, Message> {
    if servers.is_empty() {
        return components::empty_state(
            "No MCP servers found",
            "Install an MCP server from the Install page or adjust the active filters.",
        )
        .into();
    }

    components::list_column(active_targets(servers), SPACING_LG, |target| {
        let scoped = servers
            .iter()
            .copied()
            .filter(|server| server.target == target)
            .collect::<Vec<_>>();
        target_section(app, target, scoped)
    })
    .into()
}

fn active_targets(servers: &[&ManagedResource]) -> Vec<AgentToolTarget> {
    let mut targets = Vec::new();
    for target in TARGET_ORDER {
        if servers.iter().any(|server| server.target == target) {
            targets.push(target);
        }
    }
    for server in servers {
        if !targets.contains(&server.target) {
            targets.push(server.target);
        }
    }
    targets
}

fn target_section<'a>(
    app: &'a App,
    target: AgentToolTarget,
    servers: Vec<&'a ManagedResource>,
) -> Element<'a, Message> {
    let count = servers.len();
    let rows = components::list_column(servers, SPACING_SM, |server| {
        let selected = app
            .inventory
            .selected_resource_id
            .as_ref()
            .is_some_and(|id| id == &server.id);
        let pending = app
            .mcp
            .pending_remove
            .as_ref()
            .is_some_and(|id| id == &server.id);
        mcp_row(server, selected, pending, app.busy)
    });

    column![
        row![
            target_chip(target),
            text(format!("{} server(s)", count))
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

fn mcp_row<'a>(
    server: &'a ManagedResource,
    selected: bool,
    pending: bool,
    busy: bool,
) -> Element<'a, Message> {
    let enabled = server.enablement.is_enabled();
    let name = server.display_name.clone();
    let toggle_target = server.target;
    let toggle = checkbox(enabled)
        .size(18)
        .on_toggle_maybe((!busy).then_some(move |checked| {
            Message::SetMcpServerEnabled(name.clone(), toggle_target, checked)
        }));
    let select = components::small_ghost_button("View", Some(icons::EYE))
        .on_press(Message::SelectResource(server.id.clone()));
    let remove = components::confirm_button(
        pending,
        "Remove",
        "Confirm",
        Some(icons::TRASH),
        Message::RequestRemoveMcpServer(server.id.clone()),
        Message::ConfirmRemoveMcpServer(server.display_name.clone(), server.target),
        busy,
    );

    container(
        row![
            toggle,
            column![
                text(&server.display_name).size(FONT_BODY).color(TEXT),
                text(format!(
                    "{} - {}",
                    server
                        .metadata
                        .get("transport")
                        .cloned()
                        .unwrap_or_else(|| "mcp".to_string()),
                    server
                        .manifest_file
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "unknown config".to_string())
                ))
                .size(FONT_MICRO)
                .color(TEXT_MUTED)
                .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(SPACING_XS)
            .width(Length::Fill),
            resource_health_dot(server.health),
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

fn mcp_inspector<'a>(
    server: Option<&'a ManagedResource>,
    pending_remove: Option<&'a String>,
    busy: bool,
) -> iced::widget::Container<'a, Message> {
    match server {
        Some(server) => components::card(
            scrollable(
                column![
                    row![
                        target_chip(server.target),
                        components::enablement_chip(server.enablement),
                        resource_health_dot(server.health),
                    ]
                    .spacing(SPACING_SM)
                    .align_y(Alignment::Center),
                    text(&server.display_name).size(FONT_DISPLAY).color(TEXT),
                    inspector_section("OVERVIEW", overview_section(server)),
                    inspector_section("CONFIG", config_section(server)),
                    inspector_section("DIAGNOSTICS", diagnostics_section(server)),
                    inspector_section("ACTIONS", actions_section(server, pending_remove, busy)),
                ]
                .spacing(SPACING_LG),
            )
            .height(Length::Fill),
        ),
        None => components::card(
            column![
                components::section_header("Inspector", "No selection"),
                components::empty_state(
                    "No MCP server selected",
                    "Select a server to inspect configuration and actions.",
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

fn overview_section<'a>(server: &'a ManagedResource) -> Element<'a, Message> {
    column![
        components::detail_row(
            "Description",
            server.description.as_deref().unwrap_or("No description"),
        ),
        components::detail_row("Target", server.target.label()),
        components::detail_row("Enablement", server.enablement.label()),
        components::detail_row(
            "Source",
            server.source_url.as_deref().unwrap_or("Local config")
        ),
    ]
    .spacing(SPACING_SM)
    .into()
}

fn config_section<'a>(server: &'a ManagedResource) -> Element<'a, Message> {
    column![
        components::detail_row(
            "Config file",
            server
                .manifest_file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
        ),
        components::text_lines(
            server
                .metadata
                .iter()
                .map(|(key, value)| format!("{key}: {value}")),
            "No metadata",
            TEXT_SECONDARY,
            FONT_CAPTION,
        ),
    ]
    .spacing(SPACING_SM)
    .into()
}

fn diagnostics_section<'a>(server: &'a ManagedResource) -> Element<'a, Message> {
    components::diagnostic_lines(&server.diagnostics, "No diagnostics")
}

fn actions_section<'a>(
    server: &'a ManagedResource,
    pending_remove: Option<&'a String>,
    busy: bool,
) -> Element<'a, Message> {
    let toggle_label = if server.enablement.is_enabled() {
        "Disable"
    } else {
        "Enable"
    };
    let toggle_icon = if server.enablement.is_enabled() {
        icons::EYE_OFF
    } else {
        icons::EYE
    };
    let pending = pending_remove.is_some_and(|id| id == &server.id);
    let remove = components::confirm_button(
        pending,
        "Remove",
        "Confirm remove",
        Some(icons::TRASH),
        Message::RequestRemoveMcpServer(server.id.clone()),
        Message::ConfirmRemoveMcpServer(server.display_name.clone(), server.target),
        busy,
    );

    row![
        components::primary_button(toggle_label, Some(toggle_icon)).on_press_maybe(
            (!busy).then_some(Message::SetMcpServerEnabled(
                server.display_name.clone(),
                server.target,
                !server.enablement.is_enabled(),
            ))
        ),
        remove,
    ]
    .spacing(SPACING_MD)
    .align_y(Alignment::Center)
    .into()
}

fn selected_mcp_server<'a>(
    app: &'a App,
    servers: &[&'a ManagedResource],
) -> Option<&'a ManagedResource> {
    app.inventory
        .selected_resource_id
        .as_ref()
        .and_then(|id| servers.iter().copied().find(|server| server.id == *id))
        .or_else(|| servers.first().copied())
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
        AgentToolTarget::Droid => components::scope_chip(SkillScope::Droid),
        AgentToolTarget::OpenCode => components::scope_chip(SkillScope::OpenCode),
        AgentToolTarget::Zed => components::scope_chip(SkillScope::Zed),
        AgentToolTarget::Generic => components::scope_chip(SkillScope::Custom),
    }
}
