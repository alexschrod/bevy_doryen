//! bevy_doryen is a Bevy plugin that lets you use [Bevy] with the [Doryen]
//! roguelike library.
//!
//! Usage:
//! ```no_run
//! # use bevy_app::App;
//! # use bevy_doryen::{
//! #     DoryenPluginSettings,
//! #     DoryenPlugin,
//! #     RenderSystemExtensions,
//! #     ResizeMode,
//! #     MouseButton
//! # };
//! # use bevy_doryen::doryen::AppOptions;
//! # use bevy_ecs::system::IntoSystem;
//! App::build()
//!     // Insert a `DoryenPluginSettings` resource to configure the plugin.
//!     .insert_resource(DoryenPluginSettings {
//!         // `app_options` lets you configure Doryen just as if you were
//!         // using Doryen without Bevy. The default is `AppOptions::default()`.
//!         app_options: AppOptions {
//!             show_cursor: true,
//!             resizable: true,
//!             ..AppOptions::default()
//!         },
//!         // Lets you configure which mouse buttons to listen for. The default
//!         // is left, middle and right click.
//!         mouse_button_listeners: vec![
//!             MouseButton::Left,
//!             MouseButton::Middle,
//!             MouseButton::Right,
//!         ],
//!         // Lets you configure how the application should behave when resized.
//!         // The default is `ResizeMode::Nothing`. See `ResizeMode`'s
//!         // documentation for more information.
//!         resize_mode: ResizeMode::Nothing
//!     })
//!     // Add the `DoryenPlugin` to Bevy.
//!     .add_plugin(DoryenPlugin)
//!     // Add your Bevy systems like usual. Excluding startup systems, which
//!     // only run once, these systems are run during Doryen's update phase;
//!     // i.e. 60 times per second.
//!     .add_startup_system(init.system())
//!     .add_system(input.system())
//!     // The `RenderSystemExtensions` trait lets you add systems that should
//!     // be run during Doryen's render phase.
//!     .add_doryen_render_system(render.system())
//! .run();
//!
//! # fn init() { }
//! # fn input() { }
//! # fn render() { }
//! ```
//!
//! [Bevy]: https://bevyengine.org/
//! [Doryen]: https://github.com/jice-nospam/doryen-rs

// <editor-fold desc="Coding conventions" defaultstate="collapsed">
// Coding conventions
//
// Forbid (just no)
#![forbid(unsafe_code)]
// Deny (don't do this)
#![deny(anonymous_parameters)]
#![deny(elided_lifetimes_in_paths)]
#![deny(ellipsis_inclusive_range_patterns)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
#![deny(trivial_numeric_casts)]
#![deny(broken_intra_doc_links)]
//#![deny(unused)]
//
// Warn (try not to do this)
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(variant_size_differences)]
//
// Clippy conventions
//
// Deny (don't do this)
#![deny(clippy::cast_lossless)]
#![deny(clippy::default_trait_access)]
#![deny(clippy::empty_enum)]
#![deny(clippy::enum_glob_use)]
#![deny(clippy::expl_impl_clone_on_copy)]
#![deny(clippy::explicit_into_iter_loop)]
#![deny(clippy::explicit_iter_loop)]
#![deny(clippy::filter_map)]
#![deny(clippy::filter_map_next)]
#![deny(clippy::manual_find_map)]
#![deny(clippy::if_not_else)]
#![deny(clippy::invalid_upcast_comparisons)]
#![deny(clippy::items_after_statements)]
#![deny(clippy::large_digit_groups)]
#![deny(clippy::map_flatten)]
#![deny(clippy::match_same_arms)]
#![deny(clippy::mut_mut)]
#![deny(clippy::needless_continue)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::map_unwrap_or)]
#![deny(clippy::redundant_closure_for_method_calls)]
#![deny(clippy::single_match_else)]
#![deny(clippy::string_add_assign)]
#![deny(clippy::type_repetition_in_bounds)]
#![deny(clippy::unseparated_literal_suffix)]
#![deny(clippy::unused_self)]
#![deny(clippy::use_self)] // Sometimes gives false positives; feel free to disable.
#![deny(clippy::used_underscore_binding)]
//
// Warn (try not to do this)
//#![warn(clippy::must_use_candidate)]
#![warn(clippy::pub_enum_variant_names)]
#![warn(clippy::shadow_unrelated)]
#![warn(clippy::similar_names)]
#![warn(clippy::too_many_lines)]
// </editor-fold>

mod input;
mod render_system;
mod root_console;

/// Re-export of the Doryen library types.
pub mod doryen {
    pub use doryen_rs::*;
}

pub use input::{Input, Keys, MouseButton};
pub use render_system::{RenderStage, RenderState, RenderSystemExtensions};
pub use root_console::RootConsole;

use crate::doryen::{AppOptions, Console};
use crate::render_system::DoryenRenderSystems;
use bevy_app::{App as BevyApp, AppBuilder, AppExit, Events, ManualEventReader, Plugin};
use bevy_ecs::schedule::{Schedule, Stage};
use doryen_rs::{App as DoryenApp, DoryenApi, Engine, UpdateEvent};
use std::borrow::Cow;

/// The Bevy Doryen plugin.
#[derive(Default, Clone, Copy, Debug)]
pub struct DoryenPlugin;

/// DoryenPlugin settings.
pub struct DoryenPluginSettings {
    /// The [`AppOptions`] passed to the [`DoryenApp`].
    pub app_options: AppOptions,
    /// Which mouse buttons to request input data for from Doryen during the
    /// input handling.
    /// Defaults to left, middle and right mouse buttons.
    pub mouse_button_listeners: Vec<MouseButton>,
    /// What to do when the Doryen window is resized.
    pub resize_mode: ResizeMode,
}

impl std::fmt::Debug for DoryenPluginSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DoryenPluginSettings")
            .field("app_options", &"<Not Debug>")
            .field("mouse_button_listeners", &self.mouse_button_listeners)
            .field("resize_mode", &self.resize_mode)
            .finish()
    }
}

impl Default for DoryenPluginSettings {
    fn default() -> Self {
        Self {
            app_options: AppOptions::default(),
            mouse_button_listeners: vec![
                MouseButton::Left,
                MouseButton::Middle,
                MouseButton::Right,
            ],
            resize_mode: ResizeMode::Nothing,
        }
    }
}

impl Plugin for DoryenPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<RootConsole>()
            .init_resource::<Input>()
            .init_resource::<FpsInfo>()
            .add_event::<SetFontPath>()
            .add_event::<Resized>()
            .init_resource::<DoryenRenderSystems>()
            .init_resource::<RenderState>()
            .set_runner(doryen_runner);
    }
}

struct DoryenPluginEngine {
    bevy_app: BevyApp,
    app_exit_event_reader: ManualEventReader<AppExit>,
    set_font_path_event_reader: ManualEventReader<SetFontPath>,
    swap_console: Option<Console>,
    mouse_button_listeners: Vec<MouseButton>,
    previous_screen_size: (u32, u32),
    previous_console_size: (u32, u32),
    resize_mode: ResizeMode,
}

impl DoryenPluginEngine {
    #[inline]
    fn take_root_console_ownership(&mut self, api: &mut dyn DoryenApi) {
        use std::mem::swap;

        // Take ownership of the Doryen root console
        swap(api.con(), &mut self.swap_console.as_mut().unwrap());

        // Insert it into the DoryenRootConsole resource
        let mut doryen_root_console = self
            .bevy_app
            .world
            .get_resource_mut::<RootConsole>()
            .unwrap();
        doryen_root_console.0 = self.swap_console.take();
    }

    #[inline]
    fn restore_root_console_ownership(&mut self, api: &mut dyn DoryenApi) {
        use std::mem::swap;

        // Take the root console out of the DoryenRootConsole resource
        let mut doryen_root_console = self
            .bevy_app
            .world
            .get_resource_mut::<RootConsole>()
            .unwrap();
        self.swap_console = doryen_root_console.0.take();

        // Hand ownership of the Doryen root console back to Doryen
        swap(api.con(), &mut self.swap_console.as_mut().unwrap());
    }

    #[inline]
    fn take_doryen_render_schedule(&mut self) -> Schedule {
        let mut doryen_render_systems = self
            .bevy_app
            .world
            .get_resource_mut::<DoryenRenderSystems>()
            .unwrap();
        doryen_render_systems.0.take().unwrap()
    }

    #[inline]
    fn restore_doryen_render_schedule(&mut self, doryen_render_schedule: Schedule) {
        let mut doryen_render_systems = self
            .bevy_app
            .world
            .get_resource_mut::<DoryenRenderSystems>()
            .unwrap();
        doryen_render_systems.0.replace(doryen_render_schedule);
    }

    #[inline]
    fn handle_input(&mut self, api: &mut dyn DoryenApi) {
        let mut doryen_input = self.bevy_app.world.get_resource_mut::<Input>().unwrap();
        let input = api.input();
        doryen_input.handle_input(&self.mouse_button_listeners, input);
    }
}

impl Engine for DoryenPluginEngine {
    fn update(&mut self, api: &mut dyn DoryenApi) -> Option<UpdateEvent> {
        let mut doryen_fps_info = self.bevy_app.world.get_resource_mut::<FpsInfo>().unwrap();
        doryen_fps_info.fps = api.fps();
        doryen_fps_info.average_fps = api.average_fps();
        drop(doryen_fps_info);

        self.handle_input(api);

        self.take_root_console_ownership(api);
        self.bevy_app.update();
        self.restore_root_console_ownership(api);

        // Process the latest SetFontPath event
        let doryen_set_font_path_events = self
            .bevy_app
            .world
            .get_resource_mut::<Events<SetFontPath>>()
            .unwrap();
        if let Some(doryen_set_font_path) = self
            .set_font_path_event_reader
            .iter(&doryen_set_font_path_events)
            .last()
        {
            api.set_font_path(doryen_set_font_path.0.as_ref());
        }

        if let Some(app_exit_events) = self.bevy_app.world.get_resource_mut::<Events<AppExit>>() {
            if self
                .app_exit_event_reader
                .iter(&app_exit_events)
                .last()
                .is_some()
            {
                return Some(UpdateEvent::Exit);
            }
        }

        None
    }

    fn render(&mut self, api: &mut dyn DoryenApi) {
        self.take_root_console_ownership(api);

        let wc = self.bevy_app.world.cell();
        let mut rs = wc.get_resource_mut::<RenderState>().unwrap();
        if rs.0 {
            for f in &rs.1 {
                f(&wc);
            }
            rs.0 = false;
        }
        drop(rs);
        drop(wc);

        let mut doryen_render_schedule = self.take_doryen_render_schedule();
        doryen_render_schedule.run(&mut self.bevy_app.world);
        self.restore_doryen_render_schedule(doryen_render_schedule);

        self.restore_root_console_ownership(api);
    }

    fn resize(&mut self, api: &mut dyn DoryenApi) {
        let (previous_width, previous_height) = self.previous_screen_size;
        let (new_width, new_height) = api.get_screen_size();

        let mut resized_events = self
            .bevy_app
            .world
            .get_resource_mut::<Events<Resized>>()
            .unwrap();
        let resized = Resized {
            previous_width,
            previous_height,
            new_width,
            new_height,
        };
        resized_events.send(resized);
        drop(resized_events);

        match self.resize_mode {
            ResizeMode::Nothing => (),
            ResizeMode::Automatic => {
                let (previous_console_width, previous_console_height) = self.previous_console_size;

                let w_ratio = previous_width / previous_console_width;
                let h_ratio = previous_height / previous_console_height;

                let new_console_width = new_width / w_ratio;
                let new_console_height = new_height / h_ratio;
                api.con().resize(new_console_width, new_console_height);
            }
            ResizeMode::Callback(callback) => {
                self.take_root_console_ownership(api);
                callback(
                    &mut *self.bevy_app.world.get_resource_mut().unwrap(),
                    resized,
                );
                self.restore_root_console_ownership(api);
            }
        }

        self.previous_screen_size = (new_width, new_height);
        self.previous_console_size = api.con().get_size();
    }
}

fn doryen_runner(mut app: BevyApp) {
    let mut resource_settings = app
        .world
        .get_resource_or_insert_with(DoryenPluginSettings::default);
    let DoryenPluginSettings {
        app_options,
        mouse_button_listeners,
        resize_mode,
    } = std::mem::take(&mut *resource_settings);
    drop(resource_settings);

    let AppOptions {
        screen_height,
        screen_width,
        console_height,
        console_width,
        ..
    } = app_options;

    let mut doryen_app = DoryenApp::new(app_options);

    doryen_app.set_engine(Box::new(DoryenPluginEngine {
        bevy_app: app,
        app_exit_event_reader: ManualEventReader::default(),
        set_font_path_event_reader: ManualEventReader::default(),
        swap_console: Some(Console::new(1, 1)),
        mouse_button_listeners,
        previous_screen_size: (screen_width, screen_height),
        previous_console_size: (console_width, console_height),
        resize_mode,
    }));

    doryen_app.run();
}

/// This resource contains the values given by [`fps`](DoryenApi::fps) and
/// [`average_fps`](DoryenApi::average_fps) on the current update tick.
#[derive(Default, Debug, Clone, Copy)]
pub struct FpsInfo {
    /// The value given by [`fps`](DoryenApi::fps) on the current update tick.
    pub fps: u32,
    /// The value given by [`average_fps`](DoryenApi::average_fps) on the
    /// current update tick.
    pub average_fps: u32,
}

/// When you want to change Doryen's font path, emit an event of this type.
/// bevy_doryen will call [`set_font_path`](DoryenApi::set_font_path) with the
/// provided value.
#[derive(Debug, Clone)]
pub struct SetFontPath(pub Cow<'static, str>);

/// Resized event object. Whenever Doryen's [`resize`](Engine::resize) method is
/// called, an event of this type is emitted.
#[derive(Debug, Clone, Copy)]
pub struct Resized {
    /// The previous width of the Doryen game window.
    pub previous_width: u32,
    /// The previous height of the Doryen game window.
    pub previous_height: u32,
    /// The new width of the Doryen game window.
    pub new_width: u32,
    /// The new height of the Doryen game window.
    pub new_height: u32,
}

/// How the [`DoryenPlugin`] reacts to the resize event from Doryen.
#[derive(Clone, Copy)]
pub enum ResizeMode {
    /// Do nothing when the window is resized. This is the default behavior.
    Nothing,
    /// Set the console size to match the window size automatically. This
    /// retains the ratio defined between the console size and the screen size
    /// as given in the [`AppOptions`] at the start of the program.
    Automatic,
    /// Call the given function when the resize event is triggered. Because
    /// Doryen is sensitive to when the root console is resized, the safest
    /// place to make a call to do so and always have the correct behavior is
    /// during this resize callback which comes directly from Doryen itself. The
    /// [`Resized`] event is useful for reacting to resizing within Bevy
    /// systems for other reasons, but will arrive at a point that is too late
    /// to do the root console resizing correctly.
    Callback(fn(&mut RootConsole, Resized)),
}

impl std::fmt::Debug for ResizeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nothing => f.write_str("Nothing"),
            Self::Automatic => f.write_str("Automatic"),
            Self::Callback(_) => f.write_str("Callback"),
        }
    }
}
