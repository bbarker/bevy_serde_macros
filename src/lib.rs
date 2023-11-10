// Copyright (c) 2023 Brandon Elam Barker
// Copyright 2019 Herbert Wolverson (DBA Bracket Productions)
// (Copyright (c) 2017 The Specs Project Developers)

use bevy_ecs::prelude::*;
use serde::ser::{Serialize, SerializeSeq, Serializer};

/// A trait which allows to serialize entities and their components. Loosely based on the component
/// of the same name from the specs ECS library.
pub trait SerializeComponents<C, M>
where
    M: Component,
    C: Component + Serialize,
{
    /// A trait for serializing components of entities in a `World`.
    ///
    /// This trait allows serializing components of a specified component type (`C`) for all entities that
    /// also have a specified marker component (`M`). The serialization is performed using the
    /// provided serializer.
    ///
    /// # Notes
    /// - The `serialize_individually!` macro will call this function for each component type of interest.
    ///   As a result, each entity is serialized for every component it has. While the entity itself
    ///   might be represented by just a couple of integers, there might be more efficient ways to handle
    ///   this in Bevy. For instance, creating a query for an option of all serializable components could
    ///   be considered, but this approach would introduce the complexity of handling an option wrapper for
    ///   each component, potentially increasing the data size.
    ///
    /// # Type Parameters
    /// - `C`: The type of the component to be serialized. Must implement `Component` and `Serialize`.
    /// - `M`: The marker component type, implementing `Component`.
    ///
    /// # Parameters
    /// - `self`: The instance of the trait.
    /// - `world`: A reference to the `World` containing the entities and components.
    /// - `serializer`: The serializer to use for serializing the components.
    ///
    /// # Returns
    /// A result containing either `S::Ok` or an error (`S::Error`).
    fn serialize<S: Serializer>(self, world: &World, serializer: S) -> Result<S::Ok, S::Error>;
}

impl<C, M> SerializeComponents<C, M> for QueryState<(Entity, &C), With<M>>
where
    M: Component,
    C: Component + Serialize,
{
    fn serialize<S: Serializer>(mut self, world: &World, serializer: S) -> Result<S::Ok, S::Error> {
        let count = self.iter(world).count();
        match serializer.serialize_seq(Some(count)) {
            Ok(mut serseq) => {
                self.iter(world).for_each(|ent_comp| {
                    serseq.serialize_element(&ent_comp).unwrap();
                });
                serseq.end()
            }
            Err(er) => Err(er),
        }
    }
}

#[macro_export]
macro_rules! serialize_individually {
  ($world:expr, $ser:expr, $marker:ty, $( $comp_type:ty),*, $(,)?) => {
      $(
      SerializeComponents::<$comp_type, SerializeMe>::serialize(
          $world.query_filtered::<(Entity, &$comp_type), With<$marker>>(),
          $world,
          &mut $ser,
      )
      .unwrap();
      )*
  };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    enum TestEnum {
        ATest(String),
        BTest(u32),
        CTest,
    }

    #[derive(Component)]
    pub struct SerializeMe;

    #[derive(Component, Serialize, Deserialize)]
    pub struct Component1;

    #[derive(Component, Serialize, Deserialize)]
    pub struct Component2 {
        target: Entity,
    }

    #[derive(Component, Serialize, Deserialize)]
    pub struct Component3 {
        target: Entity,
        test_enum: TestEnum,
    }

    // see https://users.rust-lang.org/t/how-to-store-a-list-tuple-of-types-that-can-be-uses-as-arguments-in-another-macro/87891
    // credit to Michael F. Bryan for this approach
    #[macro_export]
    macro_rules! execute_with_type_list {
        ($name:ident!($($arg:tt)*)) => {
            $name!(
            $($arg)*,
            tests::Component1, tests::Component2, tests::Component3,
            )
        }
    }

    pub fn save_game(ecs: &mut World) -> Vec<u8> {
        let writer = Vec::new();
        let mut serializer = serde_json::Serializer::new(writer);
        execute_with_type_list!(serialize_individually!(ecs, serializer, SerializeMe));
        serializer.into_inner()
    }

    #[test]
    fn test_serialization() {
        let mut world = World::default();
        let entity1 = world.spawn(Component1).id();
        let entity2 = world
            .spawn((
                Component1,
                Component2 { target: entity1 },
                Component3 {
                    target: entity1,
                    test_enum: TestEnum::ATest("test".to_string()),
                },
            ))
            .id();

        let save_data = save_game(&mut world);
        let save_json = String::from_utf8(save_data.clone()).unwrap();
        let expected_json = "[][][]";
        assert_eq!(save_json, expected_json);

        world.get_entity_mut(entity1).unwrap().insert(SerializeMe);
        world.get_entity_mut(entity2).unwrap().insert(SerializeMe);

        let save_data = save_game(&mut world); // Normally you would save this to a file
        let save_json = String::from_utf8(save_data.clone()).unwrap(); // But we read it as a string to test
        let expected_json = r#"[[0,null],[1,null]][[1,{"target":0}]][[1,{"target":0,"test_enum":{"ATest":"test"}}]]"#;
        assert_eq!(save_json, expected_json);
    }
}
