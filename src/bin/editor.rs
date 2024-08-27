use std::{fs::File, iter::once};

use bevy::{
    prelude::*,
    render::camera::ScalingMode,
    tasks::{AsyncComputeTaskPool, Task},
    window::PrimaryWindow,
    winit::WinitWindows,
};
use bevy_egui::{
    egui::{self, Ui},
    EguiContexts, EguiPlugin,
};
use bevy_mod_async::prelude::*;
use futures_lite::future::{block_on, poll_once};
use gltf::Gltf;
use grid_builder::{
    board::{Board, BoardColor, BoardMesh, Cell, Mesh, Path},
    export::ExportBoardCmd,
    import::process_gltf,
    nav::{nav_plugin, Pick},
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, EguiPlugin, AsyncTasksPlugin, nav_plugin))
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<DrawToggles>()
        .init_resource::<Board>()
        .init_resource::<ImportedMeshes>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toolbar,
                meshes_panel,
                draw_toggle_window,
                (draw_board, board_panel).run_if(resource_exists::<Board>),
                handle_picks,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        projection: Projection::Orthographic(OrthographicProjection {
            near: -1.0,
            far: 1.0,
            scale: 10.0,
            scaling_mode: ScalingMode::FixedVertical(1.0),
            ..default()
        }),
        ..default()
    });
}

#[derive(Resource)]
struct LoadBoardTask(Task<Option<Board>>);

fn toolbar(
    ui: EguiContexts,
    window: Query<Entity, With<PrimaryWindow>>,
    windows: NonSend<WinitWindows>,
    load_task: Option<ResMut<LoadBoardTask>>,
    board: Option<Res<Board>>,
    mut commands: Commands,
) {
    egui::Window::new("Board Editor").show(ui.ctx(), |ui| {
        if let Some(mut load_task) = load_task {
            ui.spinner();
            match block_on(poll_once(&mut load_task.0)) {
                Some(Some(board)) => {
                    commands.remove_resource::<LoadBoardTask>();
                    commands.insert_resource(board);
                }
                Some(None) => commands.remove_resource::<LoadBoardTask>(),
                None => {}
            };
        } else {
            let parent = windows.get_window(window.single()).unwrap();
            if ui.button("Open Board...").clicked() {
                let task_pool = AsyncComputeTaskPool::get();
                let dialog = rfd::AsyncFileDialog::new()
                    .add_filter("JSON Files", &["json"])
                    .set_parent(parent)
                    .set_title("Open Board");
                let task = task_pool.spawn(async {
                    let Some(path) = dialog.pick_file().await else {
                        return None;
                    };
                    let file = File::open(path.path()).unwrap();
                    Some(serde_json::from_reader(file).unwrap())
                });
                commands.insert_resource(LoadBoardTask(task));
            }
            if let Some(board) = board {
                if ui.button("Save as...").clicked() {
                    commands.add(ExportBoardCmd(board.clone()));
                }
            }
        }
    });
}

#[derive(Resource, Default)]
struct ImportedMeshes(Vec<Vec<Cell>>, Vec<Mesh>);

fn meshes_panel(
    mut ui: EguiContexts,
    mut board: ResMut<Board>,
    mut meshes: ResMut<ImportedMeshes>,
) {
    egui::Window::new("Imported").show(ui.ctx_mut(), |ui| {
        if ui.button("Import...").clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                if let Ok(file) = File::open(path) {
                    if let Ok(model) = Gltf::from_reader(file) {
                        let imported = process_gltf(model);
                        meshes.0.extend(imported.0);
                        meshes.1.extend(imported.1);
                    }
                }
            }
        }
        ui.label("Boards");
        for cells in &meshes.0 {
            if ui.button("Load").clicked() {
                board.cells = cells.clone();
            }
        }
        ui.label("Meshes");
        for mesh in &meshes.1 {
            if ui.button("Add").clicked() {
                board.meshes.push(BoardMesh {
                    color: BoardColor::PlayerColor,
                    mesh: mesh.clone(),
                })
            }
        }
    });
}

enum BoardResponse {
    Remove(usize),
}

fn vec2_ui(v: &mut Vec2, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label("X");
        ui.add(egui::DragValue::new(&mut v.x));
        ui.label("Y");
        ui.add(egui::DragValue::new(&mut v.y));
    });
}

fn vec3_ui(v: &mut Vec3, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label("X");
        ui.add(egui::DragValue::new(&mut v.x));
        ui.label("Y");
        ui.add(egui::DragValue::new(&mut v.y));
        ui.label("Z");
        ui.add(egui::DragValue::new(&mut v.z));
    });
}

fn cell_ui(index: usize, cell: &mut Cell, ui: &mut Ui) -> Option<BoardResponse> {
    ui.collapsing("Neighbors", |ui| {
        for (&neighbor, path) in &mut cell.neighbors {
            ui.horizontal(|ui| {
                ui.label(neighbor.to_string());
                egui::CollapsingHeader::new("Path")
                    .id_source(neighbor)
                    .show(ui, |ui| {
                        for (keyframe, point) in &mut path.0 {
                            ui.horizontal(|ui| {
                                ui.label(keyframe.0.to_string());
                                vec2_ui(point, ui);
                            });
                        }
                    });
            });
        }
    });
    ui.collapsing("Shape", |ui| {
        cell.shape.points.iter_mut().for_each(|x| vec2_ui(x, ui));
    });
    ui.label("Position");
    vec2_ui(&mut cell.position, ui);
    if ui.button("Remove 🗑").clicked() {
        return Some(BoardResponse::Remove(index));
    }
    None
}

fn board_color_ui(color: &mut BoardColor, ui: &mut Ui) {
    if let BoardColor::StaticColor(r, g, b) = color {
        let mut override_color = true;
        ui.checkbox(&mut override_color, "Override color");
        ui.horizontal(|ui| {
            ui.label("R");
            ui.add(egui::DragValue::new(r));
            ui.label("G");
            ui.add(egui::DragValue::new(g));
            ui.label("B");
            ui.add(egui::DragValue::new(b));
        });
        if !override_color {
            *color = BoardColor::PlayerColor;
        }
    } else {
        let mut override_color = false;
        ui.checkbox(&mut override_color, "Override color");
        if override_color {
            *color = BoardColor::StaticColor(1.0, 0.0, 0.0);
        }
    }
}

fn board_mesh_ui(mesh: &mut Mesh, ui: &mut Ui) {
    match mesh {
        Mesh::IndexedLineMesh { vertices, lines } => {
            ui.label("Vertices");
            for vertex in vertices {
                vec3_ui(vertex, ui);
            }
            ui.separator();
            ui.label("Lines");
            for [a, b] in lines {
                ui.horizontal(|ui| {
                    ui.label("A");
                    ui.add(egui::DragValue::new(a));
                    ui.label("B");
                    ui.add(egui::DragValue::new(b));
                });
            }
        }
        Mesh::IndexedTriMesh {
            vertices,
            triangles,
        } => {
            ui.label("Vertices");
            for vertex in vertices {
                vec3_ui(vertex, ui);
            }
            ui.separator();
            ui.label("Triangles");
            for [a, b, c] in triangles {
                ui.horizontal(|ui| {
                    ui.label("A");
                    ui.add(egui::DragValue::new(a));
                    ui.label("B");
                    ui.add(egui::DragValue::new(b));
                    ui.label("C");
                    ui.add(egui::DragValue::new(c));
                });
            }
        }
    }
}

fn board_panel(ui: EguiContexts, mut board: ResMut<Board>) {
    egui::Window::new("Board").show(ui.ctx(), |ui| {
        ui.heading("Cells");
        egui::ScrollArea::vertical()
            .id_source("cells")
            .max_height(200.0)
            .show(ui, |ui| {
                let mut response = None;
                for (i, cell) in board.cells.iter_mut().enumerate() {
                    egui::CollapsingHeader::new(i.to_string())
                        .id_source(format!("cell{i}"))
                        .show(ui, |ui| {
                            response = cell_ui(i, cell, ui);
                        });
                }
                match response {
                    Some(BoardResponse::Remove(x)) => {
                        board.cells.remove(x);
                        for cell in &mut board.cells {
                            cell.neighbors = cell
                                .neighbors
                                .drain()
                                .filter_map(|(n, path)| {
                                    if n > x {
                                        Some((n - 1, path))
                                    } else if n < x {
                                        Some((n, path))
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                        }
                    }
                    None => {}
                };
            });
        ui.separator();
        ui.heading("Meshes");
        egui::ScrollArea::vertical()
            .id_source("meshes")
            .max_height(200.0)
            .show(ui, |ui| {
                let mut response = None;
                for (i, mesh) in board.meshes.iter_mut().enumerate() {
                    egui::CollapsingHeader::new(i.to_string())
                        .id_source(format!("mesh{i}"))
                        .show(ui, |ui| {
                            if ui.button("🗑").clicked() {
                                response = Some(BoardResponse::Remove(i));
                            }
                            board_color_ui(&mut mesh.color, ui);
                            board_mesh_ui(&mut mesh.mesh, ui);
                        });
                }
                if let Some(response) = response {
                    match response {
                        BoardResponse::Remove(i) => {
                            board.meshes.remove(i);
                        }
                    }
                }
            });
    });
}

fn handle_picks(mut picks: EventReader<Pick>, mut board: ResMut<Board>) {
    for &Pick { down, up } in picks.read() {
        let (Some(down), Some(up)) = (board.pick(down), board.pick(up)) else {
            continue;
        };
        if down == up {
            continue;
        }
        let (start, end) = (board.cells[down].position, board.cells[up].position);
        if board.cells[down].neighbors.remove(&up).is_none() {
            board.cells[down]
                .neighbors
                .insert(up, Path::simple(start, end));
        }
    }
}

#[derive(Resource)]
struct DrawToggles {
    // None: don't draw edges
    // Some(false): draw all edges
    // Some(true): draw one-way edges only
    edges: Option<bool>,
}

impl Default for DrawToggles {
    fn default() -> Self {
        Self { edges: Some(false) }
    }
}

fn draw_toggle_window(mut ui: EguiContexts, mut toggles: ResMut<DrawToggles>) {
    egui::Window::new("Draw Toggles").show(ui.ctx_mut(), |ui| {
        let mut draw_edges = toggles.edges.is_some();
        ui.checkbox(&mut draw_edges, "Draw edges");
        if let Some(only_one_way) = &mut toggles.edges {
            ui.checkbox(only_one_way, "Only draw one-way edges");
            if !draw_edges {
                toggles.edges = None;
            }
        } else {
            if draw_edges {
                toggles.edges = Some(false);
            }
        }
    });
}

fn draw_board(board: Res<Board>, toggles: Res<DrawToggles>, mut gizmos: Gizmos) {
    for (x, cell) in board.cells.iter().enumerate() {
        let positions = cell
            .shape
            .points
            .iter()
            .map(|&x| x)
            .chain(once(cell.shape.points[0]));
        gizmos.linestrip_2d(positions, Color::RED);
        if let Some(only_one_way) = toggles.edges {
            for &n in cell.neighbors.keys() {
                let x_pos = cell.position;
                let n_pos = board.cells[n].position;
                let dir = n_pos - x_pos;
                let offset = dir.perp() * 0.15;
                let x_pos = cell.position + offset;
                let n_pos = board.cells[n].position + offset;
                if !(only_one_way && board.cells[n].neighbors.contains_key(&x)) {
                    gizmos
                        .arrow_2d(
                            x_pos.lerp(n_pos, 0.35),
                            x_pos.lerp(n_pos, 0.65),
                            Color::ORANGE_RED,
                        )
                        .with_tip_length(0.3);
                }
            }
        }
    }
}
