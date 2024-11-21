#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_enum_tag::derive_enum_tag;

    #[derive_enum_tag]
    enum EmptyEnum {}

    #[derive_enum_tag]
    enum TestEnum {
        Variant1,
        Variant2,
    }

    fn spawn_test_enum(mut commands: Commands) {
        commands.spawn(TestEnum::Variant1);
        commands.spawn(TestEnum::Variant2);
    }

    use test_enum::Variant1;
    use test_enum::Variant2;
    
    fn check_enum_tags(query1: Query<&TestEnum, With<Variant1>>,
                       query2: Query<&TestEnum, With<Variant2>>) {
        assert!(!query1.is_empty());
        assert!(!query2.is_empty());
    }

    fn remove_test_enum(mut commands: Commands, query: Query<Entity, With<TestEnum>>) {
        query.iter().for_each(|entity| {
            commands.entity(entity).remove::<TestEnum>();
        });
    }

    fn check_tags_removed(query1: Query<Entity, With<Variant1>>,
                          query2: Query<Entity, With<Variant2>>) {
        assert!(query1.is_empty());
        assert!(query2.is_empty());
    }

    #[test]
    fn test_enum_tags() {
        let mut app = App::new();
        app.add_systems(Update, (spawn_test_enum, check_enum_tags, remove_test_enum, check_tags_removed).chain());
        app.update();
    }
}