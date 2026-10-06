use bevy::prelude::*;

pub fn button(label: impl Into<String>) -> impl Scene {
    let label: String = label.into();
    bsn! {
        Button
        Node {
            padding: px(10),
            border: px(5),
            border_radius: BorderRadius::MAX,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BorderColor::from(Color::BLACK)
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
        Children [(
            Text(label)
            TextColor(Color::srgb(0.9, 0.9, 0.9))
            TextShadow
        )]
        on(|event: On<Pointer<Enter>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.1))
            );
        })
        on(|event: On<Pointer<Leave>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
            );
        })
        on(|event: On<Pointer<Press>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.2))
            );
        })
        on(|event: On<Pointer<Release>>, mut commands: Commands| {
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
