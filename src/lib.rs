// Copyright (c) 2023 Brandon Elam Barker
// Copyright 2019 Herbert Wolverson (DBA Bracket Productions)
// (Copyright (c) 2017 The Specs Project Developers)

use bevy_ecs::prelude::*;
use serde::ser::Serialize;
use serde_json::Value;

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
    /// also have a specified marker component (`M`). The serialization is performed and the result is
    /// returned as a `serde_json::Value`.
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
    ///
    /// # Returns
    /// A result containing either a `serde_json::Value` representing the serialized data or an error
    /// (`serde_json::Error`).
    fn serialize(self, world: &World) -> Result<Value, serde_json::Error>;
}

impl<C, M> SerializeComponents<C, M> for QueryState<(Entity, &C), With<M>>
where
    M: Component,
    C: Component + Serialize,
{
    fn serialize(mut self, world: &World) -> Result<Value, serde_json::Error> {
        let comp_values_res: Result<Vec<Value>, serde_json::Error> =
            self.iter(world).map(serde_json::value::to_value).collect();
        comp_values_res.map(Value::Array)
    }
}

// TODO: For deserialization, we may actually want to serialize this as a map, with the
// component type as the key and the data as the value.
// https://chat.openai.com/c/a5296810-68cc-4232-b9af-a47512454323
// Otherwise I'm not sure how we will know which component type to deserialize to.

#[macro_export]
macro_rules! serialize_individually {
  ($world:expr, $ser:expr, $marker:ty, $( $comp_type:ty),*, $(,)?) => {
      let mut data_map: HashMap<String, Value> = HashMap::new();
      $(
        let comp_name_fq = stringify!($comp_type);
        let comp_name = comp_name_fq.rsplit("::").next().unwrap_or(&comp_name_fq);
        let comp_data_res = SerializeComponents::<$comp_type, SerializeMe>::serialize(
            $world.query_filtered::<(Entity, &$comp_type), With<$marker>>(),
            $world,
        );
        data_map.insert(comp_name.to_string(), comp_data_res.unwrap());
      )*
      data_map.serialize(&mut $ser).unwrap();
  };
}

/*
// Notes for specs migration:
// Removed params:
//   1. allocator
// Added params:
//   2. EntityMapper


/// A trait which allows to deserialize entities and their components.
pub trait DeserializeComponents<M>
where
    Self: Sized,
    M: Component,
{
    /// The data representation that a component group gets deserialized to.
    type Data: DeserializeOwned;

    /// Loads `Component`s to entity from `Data` deserializable representation
    // fn deserialize_entity<F>(
    //     &mut self,
    //     entity: Entity,
    //     components: Self::Data,
    //     ids: F,
    // ) -> Result<(), E>
    // where
    //     F: FnMut(M) -> Option<Entity>;

    /// Deserialize entities according to markers.
    fn deserialize<'a: 'b, 'b, 'de, D>(
        &'b mut self,
        entities: &'b EntitiesRes,
        markers: &'b mut WriteStorage<'a, M>,
        deserializer: D,
    ) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(VisitEntities::<E, M, Self> {
            entities,
            markers,
            storages: self,
            pd: PhantomData,
        })
    }
}

*/

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use bevy_ecs::entity::EntityMapper;
    use serde::{Deserialize, Serialize};

    use assert_json_diff::{assert_json_matches, CompareMode, Config, NumericMode};

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
        let json_assert_config = Config::new(CompareMode::Strict);

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
        let save_json: HashMap<String, Value> = serde_json::from_slice(&save_data).unwrap();
        let expected_json: HashMap<String, Value> =
            serde_json::from_str(r#"{"Component3": [], "Component2": [], "Component1": []}"#)
                .unwrap();
        assert_eq!(save_json, expected_json);

        world.get_entity_mut(entity1).unwrap().insert(SerializeMe);
        world.get_entity_mut(entity2).unwrap().insert(SerializeMe);

        let save_data = save_game(&mut world); // Normally you would save this to a file
        let save_json: HashMap<String, Value> = serde_json::from_slice(&save_data).unwrap();
        let expected_json: HashMap<String, Value> = serde_json::from_str(
            r#"{"Component3": [[1, {"target": 0, "test_enum": {"ATest": "test"}}]], "Component2": [[1, {"target": 0}]], "Component1": [[0, null], [1, null]]}"#,
        ).unwrap();
        assert_eq!(save_json, expected_json);

        // let entity_map: HashMap<Entity, Entity> = HashMap::new();
        // example: f(world, &mut mapper);
        // let entity_mapper = EntityMapper::world_scope(entity_map, &mut world, f)
    }
}
