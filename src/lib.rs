// Copyright (c) 2023 Brandon Elam Barker
// Copyright 2019 Herbert Wolverson (DBA Bracket Productions)
// (Copyright (c) 2017 The Specs Project Developers)

use bevy_ecs::entity::EntityMapper;
use bevy_ecs::prelude::*;
use bevy_utils::hashbrown::HashMap;
use serde::de::{Deserialize, DeserializeOwned};
use serde::ser::Serialize;
use serde_json::Value;
use std::error::Error;

const EMPTY_JS_ARRAY: Value = serde_json::json!([]);

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

fn revive_or_rejuv_entity<'de, C: Component + Deserialize<'de>>(
    entity_comps: Vec<(Entity, C)>,
) -> Box<dyn FnOnce(&mut World, &mut EntityMapper)> {
    Box::new(|world: &mut World, mapper: &mut EntityMapper| {
        entity_comps.into_iter().for_each(|(entity, comp)| {
            let new_entity = mapper.get_or_reserve(entity);
            world.entity_mut(new_entity).insert(comp);
        });
    })
}

// Notes for specs migration:
//
// Fn passed to `worldScope` takes a `&mut World` and `&mut EntityMapper`
// but `deserialize` also likely needs a deserializer, so I think we need to call
// world_scope inside of deserialize as part of a closure containing the deserializer.
// We need the hashmap to persist between calls to deserialize.
//
//
// Removed params:
//   1. allocator
//   2. entities (EntitiesRes)
//   3. markers
//   4. deserializer: D,
// Added params:
//   1. World
//   2. HashMap<Entity, Entity>
//   3. HashMap<String, Value>
// TODO: extract this data map and pass it in as a parameter to avoid
// deserializing it multiple times; we can have a caller: `deserialize_all`;
// likely need to just have it as a trait fn since to implement, need to use
// execute_with_type_list!
// TODO: deserialize outer map:
//      let save_data_string = String::from_utf8(self).unwrap();
//   4. String value of component name (corresponding to `C`)
/// A trait which allows to deserialize entities and their components.
pub trait DeserializeComponents<C, M>
where
    Self: Sized, // TODO: what do we want to use as Self, if anything?
    M: Component,
    C: Component + DeserializeOwned,
{
    fn deserialize(
        world: &mut World,
        entity_map: &mut HashMap<Entity, Entity>,
        component_json_obj: &mut HashMap<String, Value>,
        component_name: String,
    ) -> Result<(), Box<dyn Error>> // TODO: consider removing Box after tests work
    {
        // to avoid memory duplication, we remove the component vec from the map,
        // allowing the deserializer to take ownership
        let comp_vec_value = component_json_obj
            .remove(&component_name)
            .unwrap_or(EMPTY_JS_ARRAY);
        component_json_obj.shrink_to_fit();

        let entity_comps: Vec<(Entity, C)> = serde_json::from_value(comp_vec_value)?;

        Ok(EntityMapper::world_scope(entity_map, world, |world, em| {
            revive_or_rejuv_entity(entity_comps)(world, em)
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
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

        let entity_map: HashMap<Entity, Entity> = HashMap::new();
        // example: f(world, &mut mapper);
        // let entity_mapper = EntityMapper::world_scope(&mut entity_map, &mut world, f)
    }
}
