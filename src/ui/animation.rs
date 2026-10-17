// SPDX-License-Identifier: GPL-3.0-or-later
//
// Small scale/translate entry animations driven by `UiTransform`.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    #[default]
    CubicOut,
    BackOut,
}

/// An entry transition applied to a UI node.
#[derive(Component, Debug, Clone, Copy)]
pub struct UiTransition {
    pub elapsed: f32,
    pub delay: f32,
    pub duration: f32,
    pub from_scale: f32,
    pub from_y: f32,
    pub easing: Easing,
}

impl Default for UiTransition {
    fn default() -> Self {
        Self::none()
    }
}

impl UiTransition {
    pub const fn none() -> Self {
        Self {
            elapsed: 0.0,
            delay: 0.0,
            duration: 0.0,
            from_scale: 1.0,
            from_y: 0.0,
            easing: Easing::CubicOut,
        }
    }

    pub const fn screen(enabled: bool) -> Self {
        if enabled {
            Self {
                elapsed: 0.0,
                delay: 0.0,
                duration: 0.22,
                from_scale: 0.99,
                from_y: 12.0,
                easing: Easing::CubicOut,
            }
        } else {
            Self::none()
        }
    }

    pub const fn dialog(enabled: bool) -> Self {
        if enabled {
            Self {
                elapsed: 0.0,
                delay: 0.02,
                duration: 0.18,
                from_scale: 0.96,
                from_y: 10.0,
                easing: Easing::BackOut,
            }
        } else {
            Self::none()
        }
    }

    pub const fn cell(enabled: bool, exploded: bool) -> Self {
        if enabled {
            Self {
                elapsed: 0.0,
                delay: 0.0,
                duration: if exploded { 0.3 } else { 0.16 },
                from_scale: if exploded { 0.6 } else { 0.82 },
                from_y: 0.0,
                easing: Easing::BackOut,
            }
        } else {
            Self::none()
        }
    }

    pub const fn pop(enabled: bool) -> Self {
        if enabled {
            Self {
                elapsed: 0.0,
                delay: 0.0,
                duration: 0.14,
                from_scale: 0.8,
                from_y: 0.0,
                easing: Easing::BackOut,
            }
        } else {
            Self::none()
        }
    }
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate_transitions);
    }
}

fn animate_transitions(
    time: Res<Time>,
    mut transitions: Query<(Entity, &mut UiTransition, &mut UiTransform)>,
    mut commands: Commands,
) {
    for (entity, mut transition, mut transform) in &mut transitions {
        if transition.duration <= 0.0 {
            *transform = UiTransform::IDENTITY;
            commands.entity(entity).remove::<UiTransition>();
            continue;
        }

        transition.elapsed += time.delta_secs();
        let linear = ((transition.elapsed - transition.delay) / transition.duration).clamp(0.0, 1.0);
        let eased = easing(linear, transition.easing);
        let scale = transition.from_scale + (1.0 - transition.from_scale) * eased;
        let y = transition.from_y * (1.0 - eased);
        transform.scale = Vec2::splat(scale);
        transform.translation = Val2::px(0.0, y);

        if linear >= 1.0 {
            *transform = UiTransform::IDENTITY;
            commands.entity(entity).remove::<UiTransition>();
        }
    }
}

fn easing(value: f32, easing: Easing) -> f32 {
    match easing {
        Easing::CubicOut => 1.0 - (1.0 - value).powi(3),
        Easing::BackOut => {
            const OVERSHOOT: f32 = 1.70158;
            let shifted = value - 1.0;
            1.0 + (OVERSHOOT + 1.0) * shifted.powi(3) + OVERSHOOT * shifted.powi(2)
        }
    }
}
