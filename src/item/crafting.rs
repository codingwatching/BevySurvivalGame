use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;

use crate::{
    attributes::ItemAttributes,
    inventory::{Inventory, InventoryItemStack, ItemStack},
    item::{ItemDisplayMetaData, WorldObject},
    GameState,
};

pub struct CraftingPlugin;
impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CraftingSlotUpdateEvent>()
            .insert_resource(Recipes::default())
            .add_system(Self::handle_crafting_slot_update.in_set(OnUpdate(GameState::Main)));
    }
}
#[derive(Resource, Default, Deserialize)]
pub struct Recipes {
    // map of recipie result and its recipe matrix
    pub recipes_list: RecipeList,
}
pub type RecipeList = HashMap<WorldObject, CraftingGrid>;

impl CraftingPlugin {
    fn handle_crafting_slot_update(
        mut events: EventReader<CraftingSlotUpdateEvent>,
        mut inv: Query<&mut Inventory>,
        recipes_list: Res<Recipes>,
    ) {
        for _ in events.iter() {
            let crafting_slots = &inv.single().crafting_items;
            let mut recipe: CraftingGrid = [[None; 2]; 2];
            for stack_option in crafting_slots.iter() {
                if let Some(stack) = stack_option {
                    let item = stack.item_stack.obj_type;
                    let x = if stack.slot < 2 { 0 } else { 1 };
                    let y = if stack.slot % 2 == 0 { 0 } else { 1 };
                    recipe[x][y] = Some(item);
                }
            }

            let recipe = CraftingRecipe { recipe };
            let result_option = if let Some(result) =
                recipe.get_potential_reward(bevy::prelude::Res::<'_, Recipes>::clone(&recipes_list))
            {
                let attributes = ItemAttributes::default();
                //TODO: get correct metadata for new item. add as .ron data?
                Some(InventoryItemStack {
                    item_stack: ItemStack {
                        obj_type: result,
                        count: 1,
                        attributes: ItemAttributes::default(),
                        metadata: ItemDisplayMetaData {
                            name: result.to_string(),
                            desc: "A cool piece of Equipment".to_string(),
                            attributes: attributes.get_tooltips(),
                            durability: attributes.get_durability_tooltip(),
                        },
                    },
                    slot: 0,
                })
            } else {
                None
            };
            inv.single_mut().crafting_result_item = result_option;
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CraftingSlotUpdateEvent;

#[derive(PartialEq)]
pub struct CraftingRecipe {
    recipe: CraftingGrid,
}
pub type CraftingGrid = [[Option<WorldObject>; 2]; 2];

impl CraftingRecipe {
    fn get_potential_reward(self, recipes_list: Res<Recipes>) -> Option<WorldObject> {
        for (result, recipe) in recipes_list.recipes_list.iter() {
            if self == (Self { recipe: *recipe }) {
                return Some(*result);
            }
        }
        None
    }
}
