use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{
        self, Slider, SliderPrecision, SliderRange, SliderStep, SliderThumb, SliderValue,
    },
};

const SLIDER_TRACK: Color = Color::srgb(0.05, 0.05, 0.05);
const SLIDER_THUMB: Color = Color::srgb(0.35, 0.75, 0.35);
const RAIL_WIDTH: f32 = 30.0;
const RAIL_LENGTH: f32 = 180.0;
const THUMB_HEIGHT: f32 = 26.0;
const THUMB_INSET: f32 = 3.0;

pub fn button(label: impl Into<String>) -> impl Scene {
    let label: String = label.into();
    bsn! {
        ui_widgets::Button
        Node {
            padding: px(10),
            border: px(5),
            border_radius: BorderRadius::MAX,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BorderColor::from(Color::BLACK)
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
        Children [
            Text(label)
            TextColor(Color::srgb(0.9, 0.9, 0.9))
            TextShadow
        ]
        on(|event: On<PointerEnter>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.1))
            );
        })
        on(|event: On<PointerLeave>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
            );
        })
        on(|event: On<PointerPress>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.2))
            );
        })
        on(|event: On<PointerRelease>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.1))
            );
        })
    }
}

pub fn text(content: impl Into<String>) -> impl Scene {
    let content: String = content.into();
    bsn! {
        Text::new(content)
        TextFont {
            font: bevy::text::FontSourceTemplate::Family("Noto Naskh Arabic")
        }
    }
}

pub fn slider(min: f32, max: f32, value: f32, step: f32, precision: i32) -> impl Scene {
    let initial = SliderRange::new(min, max).thumb_position(value) * 100.0;

    bsn! {
        Node {
            width: px(RAIL_WIDTH),
            height: px(RAIL_LENGTH),
            border_radius: BorderRadius::all(px(RAIL_WIDTH / 2.0)),
        }
        BackgroundColor(SLIDER_TRACK)
        Hovered
        Slider {
            track_click: ui_widgets::TrackClick::Snap,
            orientation: ui_widgets::SliderOrientation::Vertical,
        }
        SliderValue(value)
        SliderRange::new(min, max)
        SliderStep(step)
        SliderPrecision(precision)
        on(|insert: On<Insert<(SliderValue, SliderRange)>>,
            sliders: Query<(&SliderValue, &SliderRange)>,
            children: Query<&Children>,
            mut thumbs: Query<&mut Node, With<SliderThumb>>| {
                let Ok((value, range)) = sliders.get(insert.entity) else {
                    return;
                };
                for child in children.iter_descendants(insert.entity) {
                    if let Ok(mut thumb) = thumbs.get_mut(child) {
                        thumb.bottom = percent(range.thumb_position(value.0) * 100.0);
                    }
                }
        })
        on(|insert: On<Insert<Hovered>>,
            sliders: Query<&Hovered, With<Slider>>,
            children: Query<&Children>,
            mut thumbs: Query<&mut BackgroundColor, With<SliderThumb>>| {
                let Ok(hovered) = sliders.get(insert.entity) else {
                    return;
                };
                for child in children.iter_descendants(insert.entity) {
                    if let Ok(mut thumb) = thumbs.get_mut(child) {
                        thumb.0 = if hovered.0 {
                            SLIDER_THUMB.lighter(0.3)
                        } else {
                            SLIDER_THUMB
                        };
                    }
                }
        })
        Children [
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(THUMB_HEIGHT),
                bottom: px(0),
            }
            Children [
                SliderThumb
                Node {
                    position_type: PositionType::Absolute,
                    left: px(THUMB_INSET),
                    right: px(THUMB_INSET),
                    height: px(THUMB_HEIGHT),
                    // Overwritten by the observer above on every change.
                    bottom: percent(initial),
                    border_radius: BorderRadius::MAX,
                }
                BackgroundColor(SLIDER_THUMB)
            ]
        ]
    }
}

pub fn slider_label<L: Component>(
    insert: On<Insert<SliderValue>>,
    sliders: Query<(&SliderValue, &ChildOf)>,
    children: Query<&Children>,
    mut labels: Query<&mut Text, With<L>>,
) {
    let Ok((value, child_of)) = sliders.get(insert.entity) else {
        return;
    };

    for sibling in children.iter_descendants(child_of.parent()) {
        if let Ok(mut label) = labels.get_mut(sibling) {
            label.0 = format!("{}", value.0.round() as i32);
        }
    }
}
