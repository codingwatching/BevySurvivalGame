use bevy::{prelude::*, render::view::RenderLayers};

use crate::{
    assets::Graphics,
    attributes::{add_item_glows, AttributeChangeEvent},
    inventory::{Inventory, InventoryItemStack, ItemStack},
    item::WorldObject,
    ui::{CHEST_INVENTORY_UI_SIZE, INVENTORY_UI_SIZE},
    ScreenResolution, GAME_HEIGHT,
};

use super::{
    crafting_ui::CraftingContainer, interactions::Interaction, Interactable,
    ShowInvPlayerStatsEvent, UIContainersParam, UIElement, CRAFTING_INVENTORY_UI_SIZE,
    FURNACE_INVENTORY_UI_SIZE, UI_SLOT_SIZE,
};

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States, Component)]
pub enum UIState {
    #[default]
    Closed,
    Inventory,
    NPC,
    Chest,
    Skills,
    Crafting,
    Furnace,
    Essence,
    Options,
    Scrapper,
}
impl UIState {
    pub fn is_inv_open(&self) -> bool {
        self == &UIState::Inventory
            || self == &UIState::Chest
            || self == &UIState::Scrapper
            || self == &UIState::Crafting
            || self == &UIState::Furnace
    }
}

#[derive(Component, Default, Clone)]
pub struct InventoryUI;
#[derive(Component, FromReflect, Reflect, Clone, Debug)]
pub struct InventorySlotState {
    pub slot_index: usize,
    pub item: Option<Entity>,
    pub count: Option<usize>,
    pub obj_type: Option<WorldObject>,
    pub r#type: InventorySlotType,
    pub dirty: bool,
}
#[derive(Resource, Default, Debug)]
pub struct InventoryState {
    pub active_hotbar_slot: usize,
    pub inv_size: Vec2,
    pub hotbar_dirty: bool,
}
#[derive(FromReflect, PartialEq, Reflect, Debug, Clone, Copy)]
pub enum InventorySlotType {
    Normal,
    Hotbar,
    Crafting,
    Equipment,
    Accessory,
    Chest,
    Furnace,
    Scrapper,
}
impl InventorySlotType {
    pub fn is_crafting(self) -> bool {
        self == InventorySlotType::Crafting
    }
    pub fn is_furnace(self) -> bool {
        self == InventorySlotType::Furnace
    }
    pub fn is_hotbar(self) -> bool {
        self == InventorySlotType::Hotbar
    }
    pub fn is_equipment(self) -> bool {
        self == InventorySlotType::Equipment
    }
    pub fn is_accessory(self) -> bool {
        self == InventorySlotType::Accessory
    }
    pub fn is_inventory(self) -> bool {
        self == InventorySlotType::Normal
    }
    pub fn is_chest(self) -> bool {
        self == InventorySlotType::Chest
    }
    pub fn is_scrapper(self) -> bool {
        self == InventorySlotType::Scrapper
    }
}
pub fn setup_inv_ui(
    mut commands: Commands,
    graphics: Res<Graphics>,
    mut inv_state: ResMut<InventoryState>,
    cur_inv_state: Res<State<UIState>>,
    mut stats_event: EventWriter<ShowInvPlayerStatsEvent>,
    resolution: Res<ScreenResolution>,
) {
    let (size, texture, pos_offset) = match cur_inv_state.0 {
        UIState::Inventory => (
            INVENTORY_UI_SIZE,
            graphics.get_ui_element_texture(UIElement::Inventory),
            Vec2::new(22., 0.5),
        ),
        UIState::Chest => (
            CHEST_INVENTORY_UI_SIZE,
            graphics.get_ui_element_texture(UIElement::ChestInventory),
            Vec2::new(22.5, 0.),
        ),
        UIState::Scrapper => (
            CHEST_INVENTORY_UI_SIZE,
            graphics.get_ui_element_texture(UIElement::ChestInventory),
            Vec2::new(22.5, 0.),
        ),
        UIState::Crafting => (
            CRAFTING_INVENTORY_UI_SIZE,
            graphics.get_ui_element_texture(UIElement::CraftingInventory),
            Vec2::new(22.5, 0.),
        ),
        UIState::Furnace => (
            FURNACE_INVENTORY_UI_SIZE,
            graphics.get_ui_element_texture(UIElement::FurnaceInventory),
            Vec2::new(22.5, 0.),
        ),
        _ => return,
    };

    let overlay = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(146. / 255., 116. / 255., 65. / 255., 0.3),
                custom_size: Some(Vec2::new(resolution.game_width, GAME_HEIGHT)),
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(-pos_offset.x, 0., -1.),
                scale: Vec3::new(1., 1., 1.),
                ..Default::default()
            },
            ..default()
        })
        .insert(RenderLayers::from_layers(&[3]))
        .insert(Name::new("overlay"))
        .id();
    let inv_e = commands
        .spawn(SpriteBundle {
            texture,
            sprite: Sprite {
                custom_size: Some(size),
                ..Default::default()
            },
            transform: Transform {
                translation: Vec3::new(pos_offset.x, pos_offset.y, 10.),
                scale: Vec3::new(1., 1., 1.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(InventoryUI)
        .insert(cur_inv_state.0.clone())
        .insert(Name::new("INVENTORY"))
        .insert(RenderLayers::from_layers(&[3]))
        .id();

    inv_state.inv_size = size;
    commands.entity(inv_e).push_children(&[overlay]);

    stats_event.send(ShowInvPlayerStatsEvent {
        stat: None,
        ignore_timer: true,
    });
}

pub fn setup_inv_slots_ui(
    mut commands: Commands,
    graphics: Res<Graphics>,
    inv_query: Query<Entity, With<InventoryUI>>,
    inv_state_res: Res<InventoryState>,
    inv_state: Res<State<UIState>>,
    inv_spawn_check: Query<Entity, Added<InventoryUI>>,

    asset_server: Res<AssetServer>,
    mut inv: Query<&mut Inventory>,
    crafting_container: Option<Res<CraftingContainer>>,
) {
    if inv_spawn_check.get_single().is_err() {
        return;
    }
    let (should_spawn_equipment, crafting_items_option) = match inv_state.0 {
        UIState::Inventory => (true, Some(inv.single().crafting_items.clone())),
        UIState::Crafting => (true, Some(crafting_container.unwrap().items.clone())),
        UIState::Chest => (false, None),
        UIState::Scrapper => (false, None),
        UIState::Furnace => (true, None),
        _ => return,
    };
    for (slot_index, item) in inv.single_mut().items.items.iter().enumerate() {
        spawn_inv_slot(
            &mut commands,
            &inv_state,
            &graphics,
            slot_index,
            Interaction::None,
            &inv_state_res,
            &inv_query,
            &asset_server,
            InventorySlotType::Normal,
            item.clone(),
        );

        // equipment slots
        if slot_index < 3 && should_spawn_equipment {
            spawn_inv_slot(
                &mut commands,
                &inv_state,
                &graphics,
                slot_index,
                Interaction::None,
                &inv_state_res,
                &inv_query,
                &asset_server,
                InventorySlotType::Equipment,
                None,
            );
        }
        // accessoyr slots
        if slot_index < 3 && should_spawn_equipment {
            spawn_inv_slot(
                &mut commands,
                &inv_state,
                &graphics,
                slot_index,
                Interaction::None,
                &inv_state_res,
                &inv_query,
                &asset_server,
                InventorySlotType::Accessory,
                None,
            );
        }
    }
    if let Some(crafting_items) = crafting_items_option {
        for (slot_index, item) in crafting_items.items.iter().enumerate() {
            spawn_inv_slot(
                &mut commands,
                &inv_state,
                &graphics,
                slot_index,
                Interaction::None,
                &inv_state_res,
                &inv_query,
                &asset_server,
                InventorySlotType::Crafting,
                item.to_owned(),
            );
        }
    }
}

pub fn spawn_inv_slot(
    commands: &mut Commands,
    inv_ui_state: &Res<State<UIState>>,
    graphics: &Graphics,
    slot_index: usize,
    interactable_state: Interaction,
    inv_state: &InventoryState,
    inv_query: &Query<Entity, With<InventoryUI>>,
    asset_server: &AssetServer,
    slot_type: InventorySlotType,
    item_stack: Option<InventoryItemStack>,
) -> Entity {
    // spawns an inv slot, with an item icon as its child if an item exists in that inv slot.
    // the slot's parent is set to the inv ui entity.
    let inv_slot_offset = match inv_ui_state.0 {
        UIState::Chest => Vec2::new(0., 0.),
        UIState::Crafting => Vec2::new(0., -4.),
        _ => Vec2::new(0., 0.),
    };

    let mut x = ((slot_index % 6) as f32 * UI_SLOT_SIZE) - (inv_state.inv_size.x) / 2.
        + UI_SLOT_SIZE / 2.
        + 4.;
    let mut y = ((slot_index / 6) as f32).trunc() * UI_SLOT_SIZE - (inv_state.inv_size.y) / 2.
        + 7.
        + UI_SLOT_SIZE / 2.;

    if slot_type.is_hotbar() {
        y = -GAME_HEIGHT / 2. + 14.;
        x = ((slot_index % 6) as f32 * UI_SLOT_SIZE) - 2. * UI_SLOT_SIZE;
    } else if slot_type.is_crafting() {
        x = ((slot_index % 8) as f32 * UI_SLOT_SIZE) - (inv_state.inv_size.x) / 2.
            + UI_SLOT_SIZE / 2.
            + 6.;

        y = -((slot_index / 8) as f32).trunc() * UI_SLOT_SIZE - (inv_state.inv_size.y) / 2.
            + 7. * UI_SLOT_SIZE
            + 15.;
        if inv_ui_state.0 == UIState::Inventory {
            x -= 2.;
            y -= 29.;
        }
    } else if slot_type.is_equipment() {
        x = UI_SLOT_SIZE - (inv_state.inv_size.x) / 2. + UI_SLOT_SIZE / 2. + 7. + 5. * UI_SLOT_SIZE;
        y = slot_index as f32 * UI_SLOT_SIZE - (inv_state.inv_size.y + UI_SLOT_SIZE) / 2.
            + UI_SLOT_SIZE
            + 1. * slot_index as f32
            + 4.;
    } else if slot_type.is_accessory() {
        x = UI_SLOT_SIZE - (inv_state.inv_size.x) / 2. + UI_SLOT_SIZE / 2. + 8. + 6. * UI_SLOT_SIZE;
        y = slot_index as f32 * UI_SLOT_SIZE - (inv_state.inv_size.y + UI_SLOT_SIZE) / 2.
            + UI_SLOT_SIZE
            + 1. * slot_index as f32
            + 4.;
    } else if slot_type.is_chest() || slot_type.is_scrapper() {
        y += 4. * UI_SLOT_SIZE + 11.;
    } else if slot_type.is_furnace() {
        if slot_index == 0 {
            x = -20.5;
            y = 26.;
        } else if slot_index == 1 {
            x = -20.5;
            y = 68.;
        } else if slot_index == 2 {
            x = 21.5;
            y = 47.;
        }
    } else if ((slot_index / 6) as f32).trunc() == 0. {
        y -= 3.;
    }
    let translation = Vec3::new(x, y, 1.) + inv_slot_offset.extend(0.);
    let mut item_icon_option = None;
    let mut item_type_option = None;
    let mut item_count_option = None;
    // check if we need to spawn an item icon for this slot
    if let Some(item) = item_stack {
        // player has item in this slot

        let obj_type = *item.get_obj();
        item_type_option = Some(obj_type);
        item_count_option = Some(item.item_stack.count);
        item_icon_option = Some(spawn_item_stack_icon(
            commands,
            graphics,
            &item.item_stack,
            asset_server,
            Vec2::ZERO,
            if slot_index == 5 || slot_index == 2 {
                Vec2::new(0.5, 0.)
            } else {
                Vec2::ZERO
            },
            3,
        ));
    }

    // Inv Slot Icon //
    let slot_icon = match slot_type {
        InventorySlotType::Equipment => match slot_index {
            2 => Some("ui/icons/ChestSlotIcon.png"),
            1 => Some("ui/icons/PantsSlotIcon.png"),
            0 => Some("ui/icons/ShoesSlotIcon.png"),
            _ => None,
        },
        InventorySlotType::Accessory => match slot_index {
            2 => Some("ui/icons/RingSlotIcon.png"),
            1 => Some("ui/icons/RingSlotIcon.png"),
            0 => Some("ui/icons/NecklaceSlotIcon.png"),
            _ => None,
        },
        _ => None,
    };
    let icon_entity_option = slot_icon.map(|slot_icon| {
        commands
            .spawn(SpriteBundle {
                texture: asset_server.load(slot_icon),
                transform: Transform {
                    translation: Vec3::new(0., 0., 1.),
                    scale: Vec3::new(1., 1., 1.),
                    ..Default::default()
                },
                sprite: Sprite {
                    custom_size: Some(Vec2::new(16., 16.)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(RenderLayers::from_layers(&[3]))
            .id()
    });

    let mut slot_entity = commands.spawn(SpriteBundle {
        texture: graphics.get_ui_element_texture(
            if slot_type.is_hotbar() && inv_state.active_hotbar_slot == slot_index {
                UIElement::InventorySlotHover
            } else if slot_type.is_hotbar() {
                UIElement::InventorySlotHotbar
            } else {
                UIElement::InventorySlot
            },
        ),

        transform: Transform {
            translation,
            scale: Vec3::new(1., 1., 1.),
            ..Default::default()
        },
        sprite: Sprite {
            custom_size: Some(Vec2::new(UI_SLOT_SIZE, UI_SLOT_SIZE)),
            ..Default::default()
        },
        ..Default::default()
    });
    slot_entity
        .insert(RenderLayers::from_layers(&[3]))
        .insert(InventorySlotState {
            slot_index,
            item: item_icon_option,
            obj_type: item_type_option,
            count: item_count_option,
            dirty: false,
            r#type: slot_type,
        })
        .insert(UIElement::InventorySlot)
        .insert(Name::new(if slot_type.is_crafting() {
            "CRAFTING SLOT"
        } else {
            "SLOT"
        }));
    if let Some(i) = item_icon_option {
        slot_entity.push_children(&[i]);
    }
    if !slot_type.is_hotbar() {
        let inv_e = inv_query.single();
        slot_entity
            .set_parent(inv_e)
            .insert(Interactable::from_state(interactable_state));
    }

    if let Some(icon_entity) = icon_entity_option {
        slot_entity.push_children(&[icon_entity]);
    }
    slot_entity.id()
}
pub fn spawn_item_stack_icon(
    commands: &mut Commands,
    graphics: &Graphics,
    item_stack: &ItemStack,
    asset_server: &AssetServer,
    icon_offset: Vec2,
    text_offset: Vec2,
    render_layer: u8,
) -> Entity {
    let has_icon = graphics.icons.as_ref().unwrap().get(&item_stack.obj_type);
    let sprite = if let Some(icon) = has_icon {
        icon.clone()
    } else {
        graphics
            .spritesheet_map
            .as_ref()
            .unwrap()
            .get(&item_stack.obj_type)
            .unwrap_or_else(|| panic!("No graphic for object {:?}", item_stack.obj_type))
            .clone()
    };
    let item_entity = commands
        .spawn(SpriteSheetBundle {
            sprite,
            texture_atlas: graphics.texture_atlas.as_ref().unwrap().clone(),
            transform: Transform {
                translation: Vec3::new(icon_offset.x, icon_offset.y, 2.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(item_stack.clone())
        .insert(RenderLayers::from_layers(&[render_layer]))
        .id();
    // glow effect for rarity
    if let Some(glow_e) = add_item_glows(commands, graphics, item_entity, item_stack.rarity.clone())
    {
        commands
            .entity(glow_e)
            .insert(RenderLayers::from_layers(&[3]));
    }

    if item_stack.count > 1 {
        let text = commands
            .spawn((
                Text2dBundle {
                    text: Text::from_section(
                        item_stack.count.to_string(),
                        TextStyle {
                            font: asset_server.load("fonts/4x5.ttf"),
                            font_size: 5.0,
                            color: Color::WHITE,
                        },
                    )
                    .with_alignment(TextAlignment::Center),
                    transform: Transform {
                        translation: Vec3::new(7., -5.5, 3.) + text_offset.extend(0.),
                        scale: Vec3::new(1., 1., 1.),
                        ..Default::default()
                    },
                    ..default()
                },
                Name::new("ITEM STACK TEXT"),
                RenderLayers::from_layers(&[render_layer]),
            ))
            .id();
        commands.entity(item_entity).push_children(&[text]);
    }
    item_entity
}
//TODO: make event?
pub fn change_hotbar_slot(
    slot: usize,
    inv_state: &mut InventoryState,
    inv_slots: &mut Query<&mut InventorySlotState>,
) {
    mark_slot_dirty(
        inv_state.active_hotbar_slot,
        InventorySlotType::Hotbar,
        inv_slots,
    );
    inv_state.active_hotbar_slot = slot;
    mark_slot_dirty(slot, InventorySlotType::Hotbar, inv_slots);
}
pub fn update_inventory_ui(
    mut commands: Commands,
    graphics: Res<Graphics>,
    mut ui_elements: Query<(Entity, &InventorySlotState)>,
    interactables: Query<&Interactable>,
    inv_state: Res<InventoryState>,
    inv_ui_state: Res<State<UIState>>,
    inv_query: Query<Entity, With<InventoryUI>>,
    asset_server: Res<AssetServer>,
    inv: Query<&mut Inventory>,
    cont_param: UIContainersParam,
) {
    for (e, slot_state) in ui_elements.iter_mut() {
        // check current inventory state against that slot's state
        // if they do not match, delete and respawn

        // hotbars are hidden when inventory is open, so defer update
        // untile inv is closed again.
        if inv_ui_state.0.is_inv_open() && slot_state.r#type.is_hotbar() {
            continue;
        }

        let interactable_option = interactables.get(e);
        let item_option = if slot_state.r#type.is_chest() {
            cont_param.chest_option.as_ref().unwrap().items.items[slot_state.slot_index].clone()
        } else if slot_state.r#type.is_scrapper() {
            cont_param.scrapper_option.as_ref().unwrap().items.items[slot_state.slot_index].clone()
        } else if slot_state.r#type.is_furnace() {
            cont_param.furnace_option.as_ref().unwrap().items.items[slot_state.slot_index].clone()
        } else if slot_state.r#type.is_crafting() && cont_param.crafting_option.is_some() {
            cont_param.crafting_option.as_ref().unwrap().items.items[slot_state.slot_index].clone()
        } else {
            inv.single()
                .get_items_from_slot_type(slot_state.r#type)
                .items[slot_state.slot_index]
                .clone()
        };
        let real_count = if let Some(item) = item_option.clone() {
            Some(item.item_stack.count)
        } else {
            None
        };

        if slot_state.dirty || slot_state.count != real_count {
            commands.entity(e).despawn_recursive();
            spawn_inv_slot(
                &mut commands,
                &inv_ui_state,
                &graphics,
                slot_state.slot_index,
                if let Ok(i) = interactable_option {
                    i.current().clone()
                } else {
                    Interaction::None
                },
                &inv_state,
                &inv_query,
                &asset_server,
                slot_state.r#type,
                item_option.clone(),
            );
        }
    }
}
/// when items in the inventory state change, update the matching entities in the UI
pub fn handle_update_inv_item_entities(
    mut inv: Query<&mut Inventory, Changed<Inventory>>,
    mut inv_slot_state: Query<&mut InventorySlotState>,
    mut att_event: EventWriter<AttributeChangeEvent>,
    mut commands: Commands,
    ui_state: Res<State<UIState>>,
) {
    if !ui_state.0.is_inv_open() {
        return;
    }
    if let Ok(inv) = inv.get_single_mut() {
        att_event.send(AttributeChangeEvent);
        for inv_item_option in inv.clone().items.items.iter() {
            if let Some(inv_item) = inv_item_option {
                let item = inv_item.item_stack.clone();
                for slot_state in inv_slot_state.iter_mut() {
                    if slot_state.slot_index == inv_item.slot
                        && (slot_state.r#type.is_inventory() || slot_state.r#type.is_hotbar())
                    {
                        if let Some(item_e) = slot_state.item {
                            commands.entity(item_e).insert(item.clone());
                        }
                    }
                }
            }
        }
    }
}

pub fn mark_slot_dirty(
    slot_index: usize,
    slot_type: InventorySlotType,
    inv_slots: &mut Query<&mut InventorySlotState>,
) {
    for mut state in inv_slots.iter_mut() {
        if state.slot_index == slot_index && (state.r#type == slot_type || state.r#type.is_hotbar())
        {
            state.dirty = true;
        }
    }
}
