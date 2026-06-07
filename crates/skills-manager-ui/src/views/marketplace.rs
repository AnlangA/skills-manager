use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, scrollable, text},
};
use skills_manager_core::{
    AgentToolTarget, MarketplaceDocument, MarketplaceEntry, MarketplaceSearchEntry,
    MarketplaceSource, MarketplaceSourceRecord, SkillScope,
};

use crate::theme::*;
use crate::{
    app::{App, Message},
    components, icons, theme,
};

const TARGET_OPTIONS: [AgentToolTarget; 3] = [
    AgentToolTarget::Generic,
    AgentToolTarget::Codex,
    AgentToolTarget::ClaudeCode,
];

const TARGET_ORDER: [AgentToolTarget; 3] = [
    AgentToolTarget::Codex,
    AgentToolTarget::ClaudeCode,
    AgentToolTarget::Generic,
];

pub fn view(app: &App) -> Element<'_, Message> {
    let configured = app.marketplace.sources.len();
    let discovered = app
        .resources
        .iter()
        .filter(|r| r.kind == skills_manager_core::ResourceKind::Marketplace)
        .count();
    let search_results = app.marketplace.search_results.len();
    let inspected_entries = app
        .marketplace
        .inspected_marketplace
        .as_ref()
        .map(|doc| doc.entries.len())
        .unwrap_or_default();

    let summary = row![
        components::summary_stat("sources", configured.to_string(), PRIMARY),
        text("\u{00B7}").size(FONT_BODY).color(TEXT_MUTED),
        components::summary_stat("local docs", discovered.to_string(), INFO),
        text("\u{00B7}").size(FONT_BODY).color(TEXT_MUTED),
        components::summary_stat("search results", search_results.to_string(), SUCCESS),
        text("\u{00B7}").size(FONT_BODY).color(TEXT_MUTED),
        components::summary_stat("inspected entries", inspected_entries.to_string(), WARNING),
    ]
    .spacing(SPACING_SM)
    .align_y(Alignment::Center);

    let sources_panel = components::card(
        column![add_source_form(app), configured_sources_list(app),].spacing(SPACING_LG),
    )
    .width(Length::FillPortion(3))
    .height(Length::Fill);

    let inspector = discovery_inspector(app)
        .width(Length::FillPortion(2))
        .height(Length::Fill);

    column![
        summary,
        row![sources_panel, inspector]
            .spacing(SPACING_LG)
            .height(Length::Fill)
    ]
    .spacing(SPACING_MD)
    .height(Length::Fill)
    .into()
}

fn add_source_form(app: &App) -> Element<'_, Message> {
    components::detail_section(
        "ADD SOURCE",
        column![
            row![
                container(components::compact_field(
                    "Label",
                    "team-codex",
                    &app.marketplace.source_label,
                    Message::MarketplaceSourceLabelChanged,
                ))
                .width(Length::FillPortion(1)),
                container(components::compact_field(
                    "Source",
                    "file path, GitHub tree URL, or raw marketplace.json URL",
                    &app.marketplace.source_value,
                    Message::MarketplaceSourceValueChanged,
                ))
                .width(Length::FillPortion(2)),
            ]
            .spacing(SPACING_MD)
            .align_y(Alignment::Center),
            row![
                container(components::styled_pick_list(
                    &TARGET_OPTIONS,
                    Some(app.marketplace.source_target),
                    Message::MarketplaceSourceTargetSelected,
                    Length::Fill,
                ))
                .width(Length::FillPortion(2)),
                container(components::compact_field(
                    "Provider",
                    "optional",
                    &app.marketplace.source_provider,
                    Message::MarketplaceSourceProviderChanged,
                ))
                .width(Length::FillPortion(3)),
            ]
            .spacing(SPACING_MD)
            .align_y(Alignment::Center),
            components::primary_button("Add source", Some(icons::DATABASE))
                .on_press_maybe((!app.busy).then_some(Message::AddMarketplaceSource)),
        ]
        .spacing(SPACING_MD),
    )
}

fn configured_sources_list(app: &App) -> Element<'_, Message> {
    if app.marketplace.sources.is_empty() {
        return components::empty_state(
            "No marketplace sources",
            "Add a local file, GitHub tree, or raw marketplace.json URL.",
        )
        .into();
    }

    let targets = active_targets(&app.marketplace.sources);
    let groups = components::list_column(targets, SPACING_LG, |target| {
        let scoped: Vec<&MarketplaceSourceRecord> = app
            .marketplace
            .sources
            .iter()
            .filter(|s| {
                let source_target = s
                    .target
                    .as_deref()
                    .and_then(|t| match t {
                        "codex" => Some(AgentToolTarget::Codex),
                        "claude_code" | "claude-code" => Some(AgentToolTarget::ClaudeCode),
                        _ => Some(AgentToolTarget::Generic),
                    })
                    .unwrap_or(AgentToolTarget::Generic);
                source_target == target
            })
            .collect();
        target_section(app, target, scoped)
    });

    column![
        components::section_label("CONFIGURED SOURCES"),
        scrollable(groups).height(Length::Fill),
    ]
    .spacing(SPACING_SM)
    .into()
}

fn active_targets(sources: &[MarketplaceSourceRecord]) -> Vec<AgentToolTarget> {
    let mut targets = Vec::new();
    for target in TARGET_ORDER {
        let prefix = target.id_prefix();
        if sources
            .iter()
            .any(|s| s.target.as_deref().unwrap_or("generic") == prefix)
        {
            targets.push(target);
        }
    }
    targets
}

fn target_section<'a>(
    app: &'a App,
    target: AgentToolTarget,
    sources: Vec<&'a MarketplaceSourceRecord>,
) -> Element<'a, Message> {
    let count = sources.len();
    let rows = components::list_column(sources, SPACING_SM, |source| source_row(app, source));

    column![
        row![
            target_chip(target),
            text(format!("{} source(s)", count))
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

fn source_row<'a>(app: &'a App, source: &'a MarketplaceSourceRecord) -> Element<'a, Message> {
    let pending = app
        .marketplace
        .pending_remove_source
        .as_ref()
        .is_some_and(|label| label == &source.label);
    let refreshed = source
        .last_refreshed_at
        .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Never refreshed".to_string());
    let refresh = components::small_ghost_button("Refresh", Some(icons::REFRESH)).on_press_maybe(
        (!app.busy).then_some(Message::RefreshMarketplaceSource(source.label.clone())),
    );
    let remove = components::confirm_button(
        pending,
        "Remove",
        "Confirm",
        Some(icons::TRASH),
        Message::RequestRemoveMarketplaceSource(source.label.clone()),
        Message::ConfirmRemoveMarketplaceSource(source.label.clone()),
        app.busy,
    );

    container(
        row![
            column![
                text(&source.label).size(FONT_BODY).color(TEXT),
                text(&source.source)
                    .size(FONT_CAPTION)
                    .color(TEXT_SECONDARY)
                    .wrapping(text::Wrapping::WordOrGlyph),
                text(refreshed).size(FONT_MICRO).color(TEXT_MUTED),
            ]
            .spacing(SPACING_XS)
            .width(Length::Fill),
            row![refresh, remove]
                .spacing(SPACING_SM)
                .align_y(Alignment::Center),
        ]
        .spacing(SPACING_MD)
        .align_y(Alignment::Center),
    )
    .padding([SPACING_MD, SPACING_MD + 2.0])
    .style(theme::card)
    .into()
}

fn discovery_inspector(app: &App) -> iced::widget::Container<'_, Message> {
    components::card(
        scrollable(
            column![
                search_section(app),
                inspected_section(app.marketplace.inspected_marketplace.as_ref()),
            ]
            .spacing(SPACING_LG),
        )
        .height(Length::Fill),
    )
    .height(Length::Fill)
}

fn search_section(app: &App) -> Element<'_, Message> {
    components::detail_section(
        "SEARCH",
        column![
            components::compact_field(
                "SkillsMP query",
                "Search public SKILL.md entries",
                &app.marketplace.search_query,
                Message::MarketplaceSearchQueryChanged,
            ),
            row![
                components::secondary_button("SkillsMP", Some(icons::SEARCH)),
                components::primary_button("Search", Some(icons::SEARCH))
                    .on_press_maybe((!app.busy).then_some(Message::SearchMarketplace)),
            ]
            .spacing(SPACING_SM)
            .align_y(Alignment::Center),
            search_results_list(app),
        ]
        .spacing(SPACING_MD),
    )
}

fn search_results_list(app: &App) -> Element<'_, Message> {
    if app.marketplace.search_results.is_empty() {
        return components::empty_state(
            "No search results",
            "Search SkillsMP to discover public skill entries.",
        )
        .into();
    }

    components::list_column(app.marketplace.search_results.iter(), SPACING_SM, |entry| {
        search_result_row(app, entry)
    })
    .into()
}

fn search_result_row<'a>(app: &'a App, entry: &'a MarketplaceSearchEntry) -> Element<'a, Message> {
    let action = components::small_ghost_button("Preview", Some(icons::EYE)).on_press_maybe(
        (!app.busy)
            .then_some(entry.source_url.clone())
            .flatten()
            .map(Message::PreviewMarketplaceSearchEntry),
    );
    let meta = result_meta(entry);

    container(
        row![
            column![
                text(&entry.name).size(FONT_BODY).color(TEXT),
                text(entry.description.as_deref().unwrap_or("No description"))
                    .size(FONT_CAPTION)
                    .color(TEXT_SECONDARY)
                    .wrapping(text::Wrapping::WordOrGlyph),
                text(meta)
                    .size(FONT_MICRO)
                    .color(TEXT_MUTED)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(SPACING_XS)
            .width(Length::Fill),
            action,
        ]
        .spacing(SPACING_MD)
        .align_y(Alignment::Center),
    )
    .padding(SPACING_MD)
    .style(theme::flat_card)
    .into()
}

fn inspected_section(document: Option<&MarketplaceDocument>) -> Element<'_, Message> {
    let Some(document) = document else {
        return components::detail_section(
            "INSPECTED MARKETPLACE",
            components::empty_state(
                "No marketplace loaded",
                "Refresh a configured source to inspect its manifest.",
            ),
        );
    };

    components::detail_section(
        "INSPECTED MARKETPLACE",
        column![
            row![
                target_chip_from_str(document.target.id_prefix()),
                text(format!("{} entries", document.entries.len()))
                    .size(FONT_CAPTION)
                    .color(TEXT_MUTED),
            ]
            .spacing(SPACING_SM)
            .align_y(Alignment::Center),
            text(
                document
                    .display_name
                    .as_deref()
                    .unwrap_or(document.name.as_str())
            )
            .size(FONT_HEADING)
            .color(TEXT),
            components::diagnostic_lines(&document.diagnostics, "No diagnostics"),
            marketplace_entries(&document.entries),
        ]
        .spacing(SPACING_MD),
    )
}

fn marketplace_entries<'a>(entries: &'a [MarketplaceEntry]) -> Element<'a, Message> {
    if entries.is_empty() {
        return text("No plugin entries")
            .size(FONT_CAPTION)
            .color(TEXT_MUTED)
            .into();
    }

    components::list_column(entries.iter(), SPACING_SM, |entry| {
        marketplace_entry_row(entry)
    })
    .into()
}

fn marketplace_entry_row(entry: &MarketplaceEntry) -> Element<'_, Message> {
    let policies = [
        entry
            .policy_installation
            .as_ref()
            .map(|value| format!("install: {value}")),
        entry
            .policy_authentication
            .as_ref()
            .map(|value| format!("auth: {value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    container(
        column![
            row![
                column![
                    text(&entry.name).size(FONT_BODY).color(TEXT),
                    text(entry.description.as_deref().unwrap_or("No description"))
                        .size(FONT_CAPTION)
                        .color(TEXT_SECONDARY)
                        .wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(SPACING_XS)
                .width(Length::Fill),
                target_chip_from_str(entry.target.id_prefix()),
            ]
            .spacing(SPACING_MD)
            .align_y(Alignment::Center),
            text(source_summary(&entry.source))
                .size(FONT_MICRO)
                .color(TEXT_MUTED)
                .wrapping(text::Wrapping::WordOrGlyph),
            components::text_lines(
                policies,
                "No marketplace policy notes",
                TEXT_MUTED,
                FONT_MICRO
            ),
        ]
        .spacing(SPACING_SM),
    )
    .padding(SPACING_MD)
    .style(theme::flat_card)
    .into()
}

fn result_meta(entry: &MarketplaceSearchEntry) -> String {
    let mut parts = vec![entry.kind.label().to_string()];
    if let Some(author) = &entry.author {
        parts.push(format!("by {author}"));
    }
    if let Some(stars) = entry.stars {
        parts.push(format!("{stars} stars"));
    }
    if let Some(path) = &entry.source_path {
        parts.push(path.clone());
    }
    if let Some(url) = &entry.source_url {
        parts.push(url.clone());
    }
    parts.join(" - ")
}

fn source_summary(source: &MarketplaceSource) -> String {
    match source {
        MarketplaceSource::Local { path } => format!("local: {path}"),
        MarketplaceSource::Git {
            url,
            path,
            reference,
        } => source_parts("git", url, path.as_deref(), reference.as_deref()),
        MarketplaceSource::GitSubdir {
            url,
            path,
            reference,
        } => source_parts("git-subdir", url, Some(path), reference.as_deref()),
        MarketplaceSource::Url { url } => format!("url: {url}"),
        MarketplaceSource::SkillsMp { query } => format!("skillsmp: {query}"),
        MarketplaceSource::Unknown { raw } => format!("unknown: {raw}"),
    }
}

fn source_parts(kind: &str, url: &str, path: Option<&str>, reference: Option<&str>) -> String {
    let mut parts = vec![format!("{kind}: {url}")];
    if let Some(path) = path {
        parts.push(format!("path {path}"));
    }
    if let Some(reference) = reference {
        parts.push(format!("ref {reference}"));
    }
    parts.join(" - ")
}

fn target_chip<'a>(target: AgentToolTarget) -> iced::widget::Container<'a, Message> {
    match target {
        AgentToolTarget::Codex => components::scope_chip(SkillScope::Codex),
        AgentToolTarget::ClaudeCode => components::scope_chip(SkillScope::ClaudeCode),
        AgentToolTarget::Generic => components::scope_chip(SkillScope::Custom),
    }
}

fn target_chip_from_str(label: &str) -> iced::widget::Container<'_, Message> {
    match label {
        "codex" => components::scope_chip(SkillScope::Codex),
        "claude_code" | "claude-code" => components::scope_chip(SkillScope::ClaudeCode),
        _ => components::scope_chip(SkillScope::Custom),
    }
}
