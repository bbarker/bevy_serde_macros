// Copyright (c) 2023 Brandon Elam Barker
// Copyright 2019 Herbert Wolverson (DBA Bracket Productions)
// (Copyright (c) 2017 The Specs Project Developers)

use bevy_ecs::prelude::*;
use bevy_utils::hashbrown::HashMap;
use serde::de::{Deserialize, DeserializeOwned};
use serde::ser::Serialize;
use serde_json::Value;

const EMPTY_JS_ARRAY: Value = serde_json::json!([]);
type EntityMapperDynFn = dyn FnOnce(&mut World, &mut HashMap<Entity, Entity>);

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
    fn serialize(self, world: &World) -> Result<Option<Value>, serde_json::Error>;
}

impl<C, M> SerializeComponents<C, M> for QueryState<(Entity, &C), With<M>>
where
    M: Component,
    C: Component + Serialize,
{
    fn serialize(mut self, world: &World) -> Result<Option<Value>, serde_json::Error> {
        let comp_data: Vec<(Entity, &C)> = self.iter(world).collect();
        if comp_data.is_empty() {
            Ok(None)
        } else {
            let comp_values = comp_data
                .into_iter()
                .map(serde_json::to_value)
                .collect::<Result<Vec<Value>, serde_json::Error>>()?;
            Ok(Some(Value::Array(comp_values)))
        }
    }
}

#[macro_export]
macro_rules! serialize_individually {
  ($world:expr, $ser:expr, $marker:ty, $( $comp_type:ty),*, $(,)?) => {
      use serde_json::Value;
      let mut data_map: HashMap<String, Value> = HashMap::new();
      $(
        let comp_name_fq = stringify!($comp_type);
        let comp_name = comp_name_fq.rsplit("::").next().unwrap_or(&comp_name_fq);
        let comp_data_res = SerializeComponents::<$comp_type, $marker>::serialize(
            $world.query_filtered::<(Entity, &$comp_type), With<$marker>>(),
            $world,
        );
        match comp_data_res.unwrap() {
            Some(comp_data) => data_map.insert(comp_name.to_string(), comp_data),
            None => None,
        };
      )*
      data_map.serialize(&mut $ser).unwrap();
  };
}

/// Some entities may exist in the World prior to deserialization, however we assume
/// these are mutually exclusive from the entities we are restoring. As such, we
/// don't need to worry about them, as the table below shows (unmapped entities
/// are those that are pre-existing and exclusive from those we are restoring):
///  
/// Entity exists in unmapped | Entity is in entity_map | Result
///              0            |             0           | create new entity; add to map
///              0            |             1           | reuse entity in map
///              1            |             0           | create new entity; add to map
///              1            |             1           | reuse entity in entity map
fn get_or_insert(
    world: &mut World,
    entity_map: &mut HashMap<Entity, Entity>,
    entity: Entity,
) -> Entity {
    match entity_map.get(&entity) {
        Some(new_entity) => *new_entity,
        None => {
            let new_entity = world.spawn_empty().id();
            entity_map.insert(entity, new_entity);
            new_entity
        }
    }
}

fn revive_or_rejuv_entity<'de, C: Component + Deserialize<'de>, M: Component + Clone>(
    entity_comps: Vec<(Entity, C)>,
    marker: M,
) -> Box<EntityMapperDynFn> {
    Box::new(
        move |world: &mut World, mapper: &mut HashMap<Entity, Entity>| {
            entity_comps.into_iter().for_each(|(entity, comp)| {
                let new_entity = get_or_insert(world, mapper, entity);
                world.entity_mut(new_entity).insert((comp, marker.clone()));
            });
        },
    )
}

#[allow(dead_code)]
pub fn deserialize<C: Component + DeserializeOwned, M: Component + Clone>(
    world: &mut World,
    entity_map: &mut HashMap<Entity, Entity>,
    component_json_obj: &mut HashMap<String, Value>,
    component_name: &str,
    marker: M,
) -> Result<(), serde_json::Error> {
    // to avoid memory duplication, we remove the component vec from the map,
    // allowing the deserializer to take ownership
    let comp_vec_value = component_json_obj
        .remove(component_name)
        .unwrap_or(EMPTY_JS_ARRAY);
    component_json_obj.shrink_to_fit();

    let entity_comps: Vec<(Entity, C)> = serde_json::from_value(comp_vec_value)?;

    revive_or_rejuv_entity(entity_comps, marker)(world, entity_map);
    Ok(())
}

#[macro_export]
macro_rules! deserialize_individually {
  ($world:expr, $emap:expr, $json_map:expr, $marker:expr, $( $comp_type:ty),*, $(,)?) => {
  {
      $(
          let comp_name_fq = stringify!($comp_type);
          let comp_name = comp_name_fq.rsplit("::").next().unwrap_or(&comp_name_fq);
          deserialize::<$comp_type, _>(
              $world,
              $emap,
              $json_map,
              &comp_name,
              $marker,
          )
          .unwrap();
      )*
  }
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

    #[derive(Clone, Component)]
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

    // We dont want to have any entities for this for testing purposes
    #[derive(Component, Serialize, Deserialize)]
    pub struct ComponentNotUsed;

    // see https://users.rust-lang.org/t/how-to-store-a-list-tuple-of-types-that-can-be-uses-as-arguments-in-another-macro/87891
    // credit to Michael F. Bryan for this approach
    #[macro_export]
    macro_rules! execute_with_type_list {
        ($name:ident!($($arg:tt)*)) => {
            $name!(
            $($arg)*,
            tests::Component1, tests::Component2, tests::Component3, tests::ComponentNotUsed,
            )
        }
    }

    pub fn save_game(ecs: &mut World) -> Vec<u8> {
        let writer = Vec::new();
        let mut serializer = serde_json::Serializer::new(writer);
        execute_with_type_list!(serialize_individually!(ecs, serializer, SerializeMe));
        serializer.into_inner()
    }

    #[allow(dead_code)]
    pub fn load_game(ecs: &mut World, save_data: Vec<u8>) -> () {
        ecs.clear_entities();
        let mut entity_map = HashMap::new();
        let mut component_value_map: HashMap<String, Value> =
            serde_json::from_slice(&save_data).unwrap();
        let marker = SerializeMe {};
        execute_with_type_list!(deserialize_individually!(
            ecs,
            &mut entity_map,
            &mut component_value_map,
            marker.clone()
        ))
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
        let save_json: HashMap<String, Value> = serde_json::from_slice(&save_data).unwrap();
        let expected_json: HashMap<String, Value> = serde_json::from_str("{}").unwrap();
        assert_eq!(save_json, expected_json);

        world.get_entity_mut(entity1).unwrap().insert(SerializeMe);
        world.get_entity_mut(entity2).unwrap().insert(SerializeMe);

        let save_data = save_game(&mut world); // Normally you would save this to a file
        let save_json: HashMap<String, Value> = serde_json::from_slice(&save_data).unwrap();
        let expected_json: HashMap<String, Value> = serde_json::from_str(
            r#"{"Component3": [[1, {"target": 0, "test_enum": {"ATest": "test"}}]], "Component2": [[1, {"target": 0}]], "Component1": [[0, null], [1, null]]}"#,
        ).unwrap();
        assert_eq!(save_json, expected_json);

        world.clear_all();
        let cleared_save_data = save_game(&mut world);
        assert_eq!(
            serde_json::from_slice::<HashMap<String, Value>>(&cleared_save_data).unwrap(),
            serde_json::from_str::<HashMap<String, Value>>("{}").unwrap()
        );
        load_game(&mut world, save_data.clone());

        let save_data2 = save_game(&mut world);
        assert_eq!(save_data2, save_data);
    }
}
