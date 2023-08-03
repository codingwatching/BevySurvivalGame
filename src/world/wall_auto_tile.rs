use bevy::{prelude::*, tasks::AsyncComputeTaskPool, utils::HashMap};

use crate::{
    assets::SpriteAnchor,
    item::{PlaceItemEvent, Wall, WorldObject},
    proto::proto_param::ProtoParam,
    GameParam,
};

use super::{
    chunk::{GenerateObjectsEvent, VisibleObject},
    generation::WallBreakEvent,
    world_helpers::{get_neighbour_obj_data, get_neighbour_quadrant, world_pos_to_tile_pos},
    TileMapPosition,
};
#[derive(Component)]
pub struct Dirty;
#[derive(Component)]
pub struct AutoTileComplete;

#[derive(Component)]
pub struct ChunkWallCache {
    pub walls: HashMap<TileMapPosition, bool>,
}

pub fn update_wall(
    mut commands: Commands,
    proto_param: ProtoParam,
    mut walls_to_update: Query<(Entity, &mut TextureAtlasSprite), (With<Wall>, With<Dirty>)>,
    mut game: GameParam,
    txns: Query<&GlobalTransform>,
    chunk_wall_cache: Query<&mut ChunkWallCache>,
    gen_check: Res<Events<GenerateObjectsEvent>>,
) {
    if !gen_check.is_empty() {
        return;
    }
    'outer: for (wall_entity, mut wall_sprite) in walls_to_update.iter_mut() {
        let mut has_wall_above = false;
        let mut has_wall_below = false;
        let mut has_wall_on_left_side = false;
        let mut has_wall_on_right_side = false;

        let new_wall_pos = world_pos_to_tile_pos(
            txns.get(wall_entity).unwrap().translation().truncate() - Vec2::new(0., 8.),
        );

        let mut neighbour_walls: HashMap<TileMapPosition, _> = HashMap::new();
        let mut final_sprite_index = 0;
        let mut final_sprite_offset = 0;
        for dy in -1i8..=1 {
            for dx in -1i8..=1 {
                //skip corner block updates for walls
                if dx == 0 && dy == 0 {
                    continue;
                }
                let neighbour_pos = get_neighbour_quadrant(new_wall_pos.clone(), (dx, dy));
                let Some(neighbour_chunk_e) = game.get_chunk_entity(neighbour_pos.chunk_pos) else {
                    continue 'outer;};
                if let Ok(cache) = chunk_wall_cache.get(*neighbour_chunk_e) {
                    if let Some(cached_wall) = cache.walls.get(&neighbour_pos) {
                        neighbour_walls.insert(neighbour_pos, *cached_wall);
                    } else {
                        neighbour_walls.insert(neighbour_pos, false);
                    }
                }
            }
        }

        commands.entity(wall_entity).remove::<Dirty>();
        for dy in -1i8..=1 {
            for dx in -1i8..=1 {
                //skip corner block updates for walls
                if (dx != 0 && dy != 0) || (dx == 0 && dy == 0) {
                    continue;
                }

                // only use neighbours that are a wall
                let mut neighbour_is_wall = false;
                let neighbour_pos = get_neighbour_quadrant(new_wall_pos.clone(), (dx, dy));

                if let Some(is_wall) = neighbour_walls.get(&neighbour_pos) {
                    if *is_wall {
                        if dy == 1 {
                            has_wall_above = true;
                        } else if dy == -1 {
                            has_wall_below = true;
                        } else if dx == -1 {
                            has_wall_on_left_side = true;
                        } else if dx == 1 {
                            has_wall_on_right_side = true;
                        }
                        neighbour_is_wall = true;
                    }
                }
                let updated_bit_index =
                    compute_wall_index(final_sprite_index, (dx, dy), !neighbour_is_wall);

                final_sprite_offset = 0;
                final_sprite_index = updated_bit_index;
            }
        }
        let mut first_corner_neighbour_is_not_wall = false;
        for dy in -1i8..=1 {
            for dx in -1i8..=1 {
                //only bottom corner block updates now
                if dx == 0 || dy != -1 {
                    continue;
                }
                // only use neighbours that are walls
                let neighbour_pos = get_neighbour_quadrant(new_wall_pos.clone(), (dx, dy));

                let mut this_corner_neighbour_is_wall = false;

                if let Some(is_wall) = neighbour_walls.get(&neighbour_pos) {
                    this_corner_neighbour_is_wall = *is_wall;
                }

                let has_wall_on_this_side = if dx == -1 {
                    has_wall_on_left_side
                } else {
                    has_wall_on_right_side
                };
                if !(this_corner_neighbour_is_wall || !has_wall_on_this_side || !has_wall_below) {
                    let updated_bit_index = if has_wall_above
                        && has_wall_on_left_side
                        && has_wall_on_right_side
                    {
                        if first_corner_neighbour_is_not_wall && !this_corner_neighbour_is_wall {
                            10
                        } else if dx == -1 {
                            14
                        } else {
                            15
                        }
                    } else if has_wall_above {
                        if dx == -1 {
                            7
                        } else {
                            6
                        }
                    } else if !has_wall_above && has_wall_on_left_side && has_wall_on_right_side {
                        if first_corner_neighbour_is_not_wall && !this_corner_neighbour_is_wall {
                            4
                        } else if dx == -1 {
                            13
                        } else {
                            11
                        }
                    } else {
                        final_sprite_index
                    };
                    final_sprite_offset = 16;
                    final_sprite_index = updated_bit_index;
                    first_corner_neighbour_is_not_wall = true;
                }
            }
        }

        let mut new_wall_data = game
            .get_tile_obj_data_mut(new_wall_pos.clone(), &proto_param)
            .unwrap();

        new_wall_data.texture_offset = final_sprite_offset;

        new_wall_data.obj_bit_index = final_sprite_index;
        (*wall_sprite).index = (final_sprite_index + final_sprite_offset) as usize;
    }
}
pub fn handle_wall_break(
    mut game: GameParam,
    mut commands: Commands,
    proto_param: ProtoParam,
    mut obj_break_events: EventReader<WallBreakEvent>,
    mut chunk_wall_cache: Query<&mut ChunkWallCache>,
) {
    for broken_wall in obj_break_events.iter() {
        let chunk_e = game.get_chunk_entity(broken_wall.pos.chunk_pos).unwrap();
        if let Ok(mut cache) = chunk_wall_cache.get_mut(*chunk_e) {
            cache.walls.insert(broken_wall.pos.clone(), false);
        }
        mark_neighbour_walls_dirty(
            broken_wall.pos.clone(),
            &mut game,
            &proto_param,
            &mut commands,
            &chunk_wall_cache,
        );
    }
}
pub fn handle_wall_placed(
    mut game: GameParam,
    mut commands: Commands,
    proto_param: ProtoParam,
    mut events: EventReader<PlaceItemEvent>,

    mut chunk_wall_cache: Query<&mut ChunkWallCache>,
) {
    for PlaceItemEvent {
        obj,
        pos,
        loot_chest_type: _,
    } in events.iter()
    {
        if proto_param.get_component::<Wall, _>(*obj).is_none() {
            continue;
        }
        let new_wall_pos = world_pos_to_tile_pos(*pos);
        let chunk_e = game.get_chunk_entity(new_wall_pos.chunk_pos).unwrap();
        if let Ok(mut cache) = chunk_wall_cache.get_mut(*chunk_e) {
            cache.walls.insert(world_pos_to_tile_pos(*pos), true);
        }
        mark_neighbour_walls_dirty(
            new_wall_pos,
            &mut game,
            &proto_param,
            &mut commands,
            &chunk_wall_cache,
        );
    }
}

pub fn mark_neighbour_walls_dirty(
    target_pos: TileMapPosition,
    game: &mut GameParam,
    proto_param: &ProtoParam,
    commands: &mut Commands,
    chunk_wall_cache: &Query<&mut ChunkWallCache>,
) {
    for dy in -1i8..=1 {
        for dx in -1i8..=1 {
            //skip corner block updates
            if dx == 0 && dy == 0 {
                continue;
            }
            let wall_pos = target_pos;
            let neighbour_pos = get_neighbour_quadrant(wall_pos.clone(), (dx, dy));
            let neighbour_chunk_e = game.get_chunk_entity(neighbour_pos.chunk_pos).unwrap();
            if let Ok(cache) = chunk_wall_cache.get(*neighbour_chunk_e) {
                if let Some(true) = cache.walls.get(&neighbour_pos).cloned() {
                    let new_wall_entity = game
                        .get_obj_entity_at_tile(neighbour_pos.clone(), proto_param)
                        .unwrap();
                    commands.entity(new_wall_entity).insert(Dirty);
                }
            } else if let Some((_, neighbour_block_entity_data)) =
                get_neighbour_obj_data(wall_pos, (dx, dy), game, proto_param)
            {
                if let Some(_wall) =
                    proto_param.get_component::<Wall, _>(neighbour_block_entity_data.object)
                {
                    let new_wall_entity = game
                        .get_obj_entity_at_tile(neighbour_pos.clone(), proto_param)
                        .unwrap();

                    commands.entity(new_wall_entity).insert(Dirty);
                }
            }
        }
    }
}
pub fn compute_wall_index(neighbour_bits: u8, edge: (i8, i8), remove: bool) -> u8 {
    let mut index = 0;
    if edge == (0, 1) {
        //above me...
        // Top edge needs b0 b1
        if !remove {
            index = 0b0010;
        }
        index |= neighbour_bits & 0b1101;
    } else if edge == (1, 0) {
        // Right edge
        if !remove {
            index = 0b1000;
        }
        index |= neighbour_bits & 0b0111;
    } else if edge == (0, -1) {
        // Bottom edge
        if !remove {
            index = 0b0100;
        }
        index |= neighbour_bits & 0b1011;
    } else if edge == (-1, 0) {
        // Left edge
        if !remove {
            index = 0b0001;
        }
        index |= neighbour_bits & 0b1110;
    } else if edge == (-1, -1) {
        // Bottom Left edge, remove left bit
        // index |= new_tile_bits & 0b0001;
        index |= neighbour_bits & 0b1110;
    } else if edge == (1, -1) {
        // Bottom Right edge, remove Right bit
        // index |= new_tile_bits & 0b0001;
        index |= neighbour_bits & 0b0111;
    }
    index
}
