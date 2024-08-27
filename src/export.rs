use bevy::{ecs::system::Command, prelude::*, window::PrimaryWindow, winit::WinitWindows};
use bevy_mod_async::SpawnTaskExt;

use crate::board::Board;

pub struct ExportBoardCmd(pub Board);

#[derive(Resource)]
pub struct Exporting;

impl Command for ExportBoardCmd {
    fn apply(self, world: &mut World) {
        world.spawn_task(|cx| async move {
            let dialog = rfd::AsyncFileDialog::new()
                .add_filter("JSON Files", &["json"])
                .set_title("Export JSON");
            let dialog = cx
                .with_world(|world: &mut World| {
                    world.insert_resource(Exporting);
                    let primary_window = world
                        .query_filtered::<Entity, With<PrimaryWindow>>()
                        .single(world);
                    let parent_window_handle = world
                        .non_send_resource::<WinitWindows>()
                        .get_window(primary_window)
                        .unwrap();
                    dialog.set_parent(parent_window_handle)
                })
                .await;
            if let Some(file) = dialog.save_file().await {
                let Self(board) = self;
                let json = serde_json::to_string(&board).unwrap();
                match file.write(json.as_bytes()).await {
                    Err(e) => println!("Error writing board: {e:?}"),
                    _ => {}
                }
            };
            cx.with_world(|world| world.remove_resource::<Exporting>())
                .await;
        });
    }
}
