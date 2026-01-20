//! CSS generation for the launcher window

use crate::config::Theme;
use crate::widgets;
use crate::widgets::design::search_bar as design;
use tracing::debug;

/// Apply theme CSS to the provider
// CSS template - splitting would scatter related style rules
#[allow(clippy::too_many_lines)]
pub(crate) fn apply_css(provider: &gtk4::CssProvider, theme: &Theme) {
    let colors = &theme.colors;
    let config = &theme.config;

    // Base launcher styles
    let base_css = format!(
        r#"
            * {{
                all: unset;
                background: transparent;
                background-color: transparent;
            }}

            window {{
                background-color: transparent;
                background: transparent;
            }}

            box.launcher-container {{
                background-color: alpha({surface_container_highest}, {bg_opacity});
                background: alpha({surface_container_highest}, {bg_opacity});
                border-radius: {radius_normal}px;
            }}

            .icon-container {{
                background-color: {primary_container};
                border-radius: {icon_container_radius}px;
                min-width: {icon_size}px;
                min-height: {icon_size}px;
                border: {border_thin}px solid transparent;
                transition: all 150ms ease-in-out;
            }}

            .icon-container.plugin-active {{
                background-color: {primary_container};
                border-color: alpha({outline}, 0.28);
                border-radius: {icon_container_active_radius}px;
            }}

            .material-icon {{
                font-family: "{icon_font}";
                font-size: {icon_font_size}px;
                color: {on_surface_variant};
                min-width: {icon_size}px;
                min-height: {icon_size}px;
            }}

            .icon-container.plugin-active .material-icon {{
                color: {on_primary_container};
            }}

            @keyframes spin {{
                from {{ transform: rotate(0deg); }}
                to {{ transform: rotate(360deg); }}
            }}

            .search-spinner {{
                color: {on_primary_container};
                animation: spin 1s linear infinite;
            }}

            .search-input-container {{
                background-color: alpha({surface_container_low}, {content_opacity});
                border-radius: 9999px;
                padding: {padding_xs}px {padding_md}px;
                min-width: 0;
                border: {border_thin}px solid transparent; /* Always have border to prevent layout shift */
                border-color: alpha({outline}, 0.28);
                transition: background-color 180ms cubic-bezier(0.25, 0.1, 0.25, 1),
                            background 180ms cubic-bezier(0.25, 0.1, 0.25, 1),
                            border-color 180ms cubic-bezier(0.25, 0.1, 0.25, 1);
            }}

            .search-input-container:focus-within {{
                border-color: {primary};
                border: {border_thick}px solid {primary};
            }}

            .launcher-search {{
                background: transparent;
                color: {on_surface_variant};
                caret-color: {on_surface_variant};
                font-family: "{main_font}", "Inter", sans-serif;
                font-size: {font_search}px;
                min-height: {search_min_height}px;
                padding: 0;
            }}

            .launcher-search placeholder {{
                color: {outline};
            }}

            .caret-toggle {{
                background-color: transparent;
                border: none;
                padding: {padding_xs}px;
                min-width: {caret_size}px;
                min-height: {caret_size}px;
            }}

            .caret-toggle:hover {{
                background-color: {surface_container_highest};
                border-radius: 9999px;
            }}

            .caret-icon {{
                font-family: "{icon_font}";
                font-size: {font_normal}px;
                color: {outline};
            }}

            .caret-toggle.drag-mode {{
                background-color: {primary_container};
                border-radius: 9999px;
            }}

            .caret-toggle.drag-mode .caret-icon {{
                color: {on_primary_container};
            }}

            .click-catcher {{
                background-color: transparent;
                background: transparent;
            }}
            "#,
        surface_container_low = colors.surface_container_low,
        surface_container_highest = colors.surface_container_highest,
        primary_container = colors.primary_container,
        on_primary_container = colors.on_primary_container,
        on_surface_variant = colors.on_surface_variant,
        primary = colors.primary,
        outline = colors.outline,
        bg_opacity = theme.bg_opacity(),
        content_opacity = theme.content_opacity(),
        main_font = config.fonts.main,
        icon_font = config.fonts.icon,
        radius_normal = theme.scaled(design::RADIUS),
        icon_size = theme.scaled(design::ICON_CONTAINER_SIZE),
        icon_container_radius = theme.scaled(15), // between radius::MD=12 and radius::LG=16
        icon_container_active_radius = theme.scaled(10), // between radius::SM=8 and radius::MD=12
        icon_font_size = theme.scaled_font(design::ICON_SIZE),
        font_search = theme.scaled_font(design::FONT_SIZE_SEARCH),
        font_normal = theme.scaled_font(design::FONT_SIZE_NORMAL),
        caret_size = theme.scaled(design::CARET_TOGGLE_SIZE),
        padding_xs = theme.scaled(4),  // spacing::XS
        padding_md = theme.scaled(12), // spacing::MD
        border_thin = theme.scaled(1),
        border_thick = theme.scaled(2),
        search_min_height = theme.scaled(24), // icon::LG
    );

    let result_item_css = widgets::result_item::result_item_css(theme);
    let result_list_css = widgets::result_list::result_list_css(theme);
    let result_grid_css = widgets::result_grid::result_grid_css(theme);
    let grid_item_css = widgets::grid_item::grid_item_css(theme);
    let result_visual_css = widgets::result_visual::result_visual_css(theme);
    let result_card_css = widgets::result_card::result_card_css(theme);
    let preview_panel_css = widgets::preview_panel::preview_panel_css(theme);
    let badge_css = widgets::badge::badge_css(theme);
    let chip_css = widgets::chip::chip_css(theme);
    let gauge_css = widgets::gauge::gauge_css(theme);
    let graph_css = widgets::graph::graph_css(theme);
    let ripple_button_css = widgets::ripple_button::ripple_button_css(theme);
    let kbd_css = widgets::kbd::kbd_css(theme);
    let action_bar_css = widgets::action_bar::action_bar_css(theme);
    let keybinding_map_css = widgets::keybinding_map::keybinding_map_css(theme);
    let ambient_item_css = widgets::ambient_item::ambient_item_css(theme);

    let form_css = format!(
        r"
            .form-container {{
                padding: {padding_md}px {padding_lg}px {padding_lg}px {padding_lg}px;
                background: {surface_container_low};
                background-color: {surface_container_low};
                border-radius: {radius}px;
                margin-top: {margin_sm}px;
            }}

            .form-title {{
                font-size: {font_large}px;
                font-weight: 600;
                color: {on_surface};
                margin-bottom: {margin_sm}px;
            }}

            .form-field-label {{
                color: {on_surface_variant};
                font-size: {font_small}px;
                margin-bottom: {margin_xs}px;
            }}

            .form-entry {{
                background-color: alpha({surface_container_low}, {content_opacity});
                border-radius: {radius}px;
                padding: {padding_sm}px {padding_entry}px;
                min-height: {entry_height}px;
                border: {border_thin}px solid alpha({outline}, 0.28);
                color: {on_surface_variant};
                caret-color: {on_surface_variant};
                font-size: {font_normal}px;
                transition: border-color 180ms cubic-bezier(0.25, 0.1, 0.25, 1);
            }}

            .form-entry text {{
                background-color: transparent;
                border: none;
                color: {on_surface_variant};
                caret-color: {on_surface_variant};
            }}

            .form-entry:focus-within {{
                border: {border_thick}px solid {primary};
                padding: {padding_sm_minus_1}px {padding_entry_minus_1}px;
            }}

            .form-entry placeholder {{
                color: {outline};
            }}

            .form-textarea {{
                background-color: alpha({surface_container_low}, {content_opacity});
                border-radius: {radius}px;
                padding: {padding_sm}px {padding_entry}px;
                min-height: {textarea_height}px;
                border: {border_thin}px solid alpha({outline}, 0.28);
                color: {on_surface_variant};
                caret-color: {on_surface_variant};
                font-size: {font_normal}px;
                transition: border-color 180ms cubic-bezier(0.25, 0.1, 0.25, 1);
            }}

            .form-textarea:focus-within {{
                border: {border_thick}px solid {primary};
                padding: {padding_sm_minus_1}px {padding_entry_minus_1}px;
            }}

            .form-textarea text {{
                background-color: transparent;
                color: {on_surface_variant};
                caret-color: {on_surface_variant};
            }}

            .form-container dropdown {{
                background-color: alpha({surface_container_low}, {content_opacity});
                border-radius: 9999px;
                padding: {padding_sm}px {padding_entry}px;
                border: {border_thin}px solid alpha({outline}, 0.28);
                color: {on_surface_variant};
            }}

            .form-container dropdown:focus {{
                border: {border_thick}px solid {primary};
            }}

            .form-container checkbutton,
            .form-container switch {{
                color: {on_surface_variant};
            }}

            .form-container scale {{
                color: {on_surface_variant};
            }}

            .form-container scale trough {{
                background-color: alpha({outline}, 0.3);
                border-radius: {scale_radius}px;
                min-height: {scale_height}px;
            }}

            .form-container scale highlight {{
                background-color: {primary};
                border-radius: {scale_radius}px;
            }}

            .form-container scale slider {{
                background-color: {primary};
                border-radius: 50%;
                min-width: {slider_thumb}px;
                min-height: {slider_thumb}px;
            }}

            .form-actions {{
                margin-top: {margin_md}px;
            }}

            .form-actions button {{
                background-color: {surface_container_high};
                border-radius: 9999px;
                border: {border_thin}px solid alpha({outline}, 0.28);
                padding: {padding_sm}px {padding_lg}px;
                color: {on_surface_variant};
                font-size: {font_normal}px;
                font-weight: 500;
                transition: all 150ms ease-in-out;
            }}

            .form-actions button:hover {{
                background-color: {surface_container_highest};
            }}

            .form-actions button.submit-button {{
                background-color: {primary_container};
                color: {on_primary_container};
                border-color: transparent;
            }}

            .form-actions button.submit-button:hover {{
                background-color: alpha({primary_container}, 0.85);
            }}
            ",
        surface_container_low = colors.surface_container_low,
        surface_container_high = colors.surface_container_high,
        surface_container_highest = colors.surface_container_highest,
        primary_container = colors.primary_container,
        on_primary_container = colors.on_primary_container,
        primary = colors.primary,
        outline = colors.outline,
        on_surface = colors.on_surface,
        on_surface_variant = colors.on_surface_variant,
        content_opacity = theme.content_opacity(),
        radius = theme.scaled(design::RADIUS),
        font_large = theme.scaled_font(design::FONT_SIZE_LARGE),
        font_normal = theme.scaled_font(design::FONT_SIZE_NORMAL),
        font_small = theme.scaled_font(design::FONT_SIZE_SMALL),
        // Spacing tokens
        padding_sm = theme.scaled(8),             // spacing::SM
        padding_md = theme.scaled(12),            // spacing::MD
        padding_lg = theme.scaled(16),            // spacing::LG
        padding_entry = theme.scaled(14),         // between spacing::MD=12 and spacing::LG=16
        padding_sm_minus_1 = theme.scaled(7),     // 8-1 for focus border compensation
        padding_entry_minus_1 = theme.scaled(13), // 14-1 for focus border compensation
        margin_xs = theme.scaled(4),              // spacing::XS
        margin_sm = theme.scaled(8),              // spacing::SM
        margin_md = theme.scaled(12),             // spacing::MD
        border_thin = theme.scaled(1),
        border_thick = theme.scaled(2),
        entry_height = theme.scaled(28), // action_bar::HEIGHT_NORMAL
        textarea_height = theme.scaled(80),
        scale_radius = theme.scaled(4), // radius::XS
        scale_height = theme.scaled(6),
        slider_thumb = theme.scaled(16), // icon::SM
    );

    let css = format!(
        "{base_css}\n{result_list_css}\n{result_grid_css}\n{grid_item_css}\n{result_visual_css}\n{result_card_css}\n{preview_panel_css}\n{result_item_css}\n{badge_css}\n{chip_css}\n{gauge_css}\n{graph_css}\n{ripple_button_css}\n{kbd_css}\n{action_bar_css}\n{keybinding_map_css}\n{ambient_item_css}\n{form_css}"
    );

    provider.load_from_string(&css);
    debug!(
        "CSS applied with bg_opacity={}, content_opacity={}, primary={}, surface_container={}",
        theme.bg_opacity(),
        theme.content_opacity(),
        colors.primary,
        colors.surface_container
    );
}
