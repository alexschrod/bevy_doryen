use bevy_app::AppBuilder;
use bevy_ecs::component::Component;
use bevy_ecs::schedule::{Schedule, StageLabel, State, SystemSet, SystemStage};
use bevy_ecs::system::System;
use bevy_ecs::world::WorldCell;
use std::fmt::Debug;
use std::hash::Hash;

/// The names of the Doryen plugin render stages.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, StageLabel)]
pub enum RenderStage {
    /// This stage runs before all the other stages.
    First,
    /// This stage runs right before the render stage.
    PreRender,
    /// This stage is where rendering should be done.
    Render,
    /// This stage runs right after the render stage.
    PostRender,
    /// This stage runs after all the other stages.
    Last,
}

/// RenderState is a resource that gets added to Bevy to facilitate certain
/// features of Bevy's [`State`]s.
///
/// By default, only system sets in the same
/// stage as the one a `State` was changed in can make use of the
/// [`on_inactive_update`](State::on_inactive_update) and
/// [`on_in_stack_update`](State::on_in_stack_update) run criteria. Since
/// bevy_doryen runs render systems in an entirely different [`Schedule`], we're
/// obviously way outside "the same stage" as where you typically run your
/// update code.
///
/// By calling [`RenderState::state_updated`] when you change a [`State`],
/// you enable the use of the two run criteria mentioned above in the render
/// schedule as well.
pub struct RenderState(pub(crate) bool, pub(crate) Vec<fn(&WorldCell<'_>)>);
impl RenderState {
    /// Call this method whenever you change a [`State`], i.e. when you call
    /// [`State::push`] and friends to tell bevy_doryen to run some extra code
    /// in the [`State`] that lets them work in the render [`Schedule`].
    pub fn state_updated(&mut self) {
        self.0 = true;
    }
}
impl Default for RenderState {
    fn default() -> Self {
        Self(true, Vec::new())
    }
}
impl std::fmt::Debug for RenderState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RenderState")
            .field(&self.0)
            .field(&format!("fn(&WorldCell<'_>) count = {}", self.1.len()))
            .finish()
    }
}

pub(crate) struct DoryenRenderSystems(pub(crate) Option<Schedule>);
impl Default for DoryenRenderSystems {
    fn default() -> Self {
        let mut doryen_render_systems = Self(Some(Schedule::default()));

        let schedule: &mut Schedule = doryen_render_systems.0.as_mut().unwrap();
        schedule
            .add_stage(RenderStage::First, SystemStage::single_threaded())
            .add_stage_after(
                RenderStage::First,
                RenderStage::PreRender,
                SystemStage::single_threaded(),
            )
            .add_stage_after(
                RenderStage::PreRender,
                RenderStage::Render,
                SystemStage::single_threaded(),
            )
            .add_stage_after(
                RenderStage::Render,
                RenderStage::PostRender,
                SystemStage::single_threaded(),
            )
            .add_stage_after(
                RenderStage::PostRender,
                RenderStage::Last,
                SystemStage::single_threaded(),
            );

        doryen_render_systems
    }
}

/// Adds methods to the [`AppBuilder`] for adding systems to the Doryen
/// [`render`](crate::doryen::Engine::render) schedule.
pub trait RenderSystemExtensions {
    /// Adds a system to the [`RenderStage::Render`] stage of the
    /// render schedule.
    fn add_doryen_render_system<S: System<In = (), Out = ()>>(&mut self, system: S) -> &mut Self;
    /// Adds a system to the given stage of the render schedule.
    fn add_doryen_render_system_to_stage<S: System<In = (), Out = ()>>(
        &mut self,
        stage_name: impl StageLabel,
        system: S,
    ) -> &mut Self;
    /// Adds a system set to the [`RenderStage::Render`] stage of the
    /// render schedule.
    fn add_doryen_render_system_set(&mut self, system_set: SystemSet) -> &mut Self;
    /// Adds a system set to the given stage of the render schedule.
    fn add_doryen_render_system_set_to_stage(
        &mut self,
        stage_label: impl StageLabel,
        system_set: SystemSet,
    ) -> &mut Self;

    /// Adds a [`State`] to the render schedule. This method assumes you've
    /// already added the State to the main Bevy app through
    /// [`AppBuilder::add_state`] or similar means.
    ///
    /// If you want to make use of
    /// [`on_inactive_update`](State::on_inactive_update) and
    /// [`on_in_stack_update`](State::on_in_stack_update) run criteria, you must
    /// ask for [`ResMut<RenderState>`](RenderState) in the same systems that
    /// call one of the `State` transition methods, and call
    /// [`state_updated`](RenderState::state_updated) on it, otherwise they
    /// won't work. This is due to a limitation with how `State` works in
    /// general; even trying to use those from a different
    /// [`Stage`](bevy_ecs::schedule::Stage) in Bevy will have the same issue.
    ///
    /// Important note: this must be inserted **before** all other
    /// state-dependant sets to work properly!
    fn add_doryen_render_state<T>(&mut self) -> &mut Self
    where
        T: Component + Debug + Clone + Eq + Hash;
}

#[inline(always)]
fn do_to_doryen_render_systems<F: FnOnce(&mut DoryenRenderSystems)>(
    app_builder: &mut AppBuilder,
    operation: F,
) {
    let mut doryen_render_systems = app_builder
        .app
        .world
        .get_resource_mut::<DoryenRenderSystems>()
        .unwrap();
    operation(&mut *doryen_render_systems)
}

#[inline(always)]
fn do_to_doryen_render_systems_schedule<F: FnOnce(&mut Schedule)>(
    app_builder: &mut AppBuilder,
    operation: F,
) {
    do_to_doryen_render_systems(app_builder, |drs| operation(drs.0.as_mut().unwrap()));
}

impl RenderSystemExtensions for AppBuilder {
    fn add_doryen_render_system<S: System<In = (), Out = ()>>(&mut self, system: S) -> &mut Self {
        do_to_doryen_render_systems_schedule(self, move |drss| {
            drss.add_system_to_stage(RenderStage::Render, system);
        });

        self
    }

    fn add_doryen_render_system_to_stage<S: System<In = (), Out = ()>>(
        &mut self,
        stage_label: impl StageLabel,
        system: S,
    ) -> &mut Self {
        do_to_doryen_render_systems_schedule(self, move |drss| {
            drss.add_system_to_stage(stage_label, system);
        });

        self
    }

    fn add_doryen_render_system_set(&mut self, system_set: SystemSet) -> &mut Self {
        do_to_doryen_render_systems_schedule(self, move |drss| {
            drss.add_system_set_to_stage(RenderStage::Render, system_set);
        });

        self
    }

    fn add_doryen_render_system_set_to_stage(
        &mut self,
        stage_label: impl StageLabel,
        system_set: SystemSet,
    ) -> &mut Self {
        do_to_doryen_render_systems_schedule(self, move |drss| {
            drss.add_system_set_to_stage(stage_label, system_set);
        });

        self
    }

    fn add_doryen_render_state<T>(&mut self) -> &mut Self
    where
        T: Component + Debug + Clone + Eq + Hash,
    {
        let mut rs = self.app.world.get_resource_mut::<RenderState>().unwrap();
        rs.1.push(|w| w.get_resource_mut::<State<T>>().unwrap().run_full_search());

        self.add_doryen_render_system_set_to_stage(RenderStage::Render, State::<T>::get_driver())
    }
}
