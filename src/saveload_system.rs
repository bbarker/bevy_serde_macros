/*
pub fn load_game(ecs: &mut World) {
    // Delete everything
    delete_state(ecs);
    let save_file_contents = fs::read_to_string(SAVE_FILE)
        .unwrap_or_else(|_| panic!("Unable to read file {}", SAVE_FILE));
    let mut de_ser = serde_json::Deserializer::from_str(&save_file_contents);
    {
        let mut de_ser_reqs = (
            ecs.entities(),
            ecs.write_storage::<SimpleMarker<SerializeMe>>(),
            ecs.write_resource::<SimpleMarkerAllocator<SerializeMe>>(),
        );
        execute_with_type_list!(deserialize_individually!(ecs, de_ser, de_ser_reqs));
    }

    let ser_helper_vec: Vec<Entity> = {
        let entities = ecs.entities();
        let helper = ecs.read_storage::<SerializationHelper>();
        (&entities, &helper)
            .join()
            .map(|(ent, help)| {
                // load the map
                let mut worldmap = ecs.write_resource::<super::map::Map>();
                *worldmap = help.map.clone();
                worldmap.tile_content = vec![Vec::new(); worldmap.tile_count()];
                ent
            })
            .collect()
    };
    // Delete serialization helper, so we don't keep an extra copy of it (and its contents)
    // each time we save.
    ser_helper_vec.into_iter().for_each(|help| {
        ecs.delete_entity(help)
            .unwrap_or_else(|er| panic!("Unable to delete helper: {}", er))
    });
}
*/

/*

macro_rules! deserialize_individually {
  ($ecs:expr, $de_ser:expr, $data:expr, $( $type:ty),* $(,)?) => {
      $(
      DeserializeComponents::<NoError, _>::deserialize(
          &mut ( &mut $ecs.write_storage::<$type>(), ),
          &$data.0, // entities
          &mut $data.1, // marker
          &mut $data.2, // allocater
          &mut $de_ser,
      )
      .unwrap();
      )*
  };
}

*/
