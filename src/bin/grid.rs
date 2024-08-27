use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_mod_async::prelude::*;
use grid_builder::{
    basic_grid::{hex, square, BaseCell, BaseCorner, Edge},
    board::{self, Board, BoardColor, BoardMesh, Cell, Path},
    custom_gizmos::CustomGizmos,
    export::{ExportBoardCmd, Exporting},
    nav::{nav_plugin, Pick},
    util::MinMax,
};
use std::collections::{HashMap, HashSet};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, EguiPlugin, AsyncTasksPlugin, nav_plugin))
        .init_resource::<Grid>()
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (control_panel, count_capacity, handle_picks, draw_grid),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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
    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(0.1).mesh().resolution(32)),
        material: materials.add(StandardMaterial::default()),
        ..default()
    });
}

/// Helper type for generating board meshes. `Corner` represents a vertex at a cell corner, and
/// `Tip` represents the tip of an isoceles triangle with its base between the given corners and the
/// tip pointing along `ab.perp()`, where `ab` is a vector pointing from the first corner to the
/// second.
enum TriVert<C: BaseCell> {
    Corner(C::Corner),
    Tip(C::Corner, C::Corner),
}

impl<C: BaseCell> PartialEq for TriVert<C> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Corner(l0), Self::Corner(r0)) => l0 == r0,
            (Self::Tip(l0, l1), Self::Tip(r0, r1)) => l0 == r0 && l1 == r1,
            _ => false,
        }
    }
}

impl<C: BaseCell> Eq for TriVert<C> {}

impl<C: BaseCell> std::hash::Hash for TriVert<C> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            TriVert::Corner(a) => a.hash(state),
            TriVert::Tip(a, b) => (a, b).hash(state),
        }
    }
}

fn build_board<C: BaseCell>(cells: Vec<C>, edges: Edges<C>, arrow_offset: f32) -> Board {
    let mut boring_edges = HashSet::<Edge<C>>::default();
    let mut directed_edges = HashSet::<Edge<C>>::default();

    for cell in &cells {
        for neighbor in cell.neighbors() {
            // we're going to assume the edge is moving ccw around `cell`, so cw around `neighbor`
            let mut edge = cell.neighboring_edge(&neighbor);
            if cells.contains(&neighbor) {
                match edges.edge_dir(&cell, &neighbor) {
                    Some(EdgeDir::AToB) => {
                        edge.reverse();
                        directed_edges.insert(edge);
                    }
                    Some(EdgeDir::BToA) => {
                        directed_edges.insert(edge);
                    }
                    None => {
                        let (a, b) = edge[0].min_max(edge[1]);
                        boring_edges.insert([a, b]);
                    }
                }
            } else {
                let (a, b) = edge[0].min_max(edge[1]);
                boring_edges.insert([a, b]);
            }
        }
    }

    // First just get a big (deduped) list of all the corners we'll be using
    let corners = boring_edges
        .iter()
        .flat_map(|[x, y]| [*x, *y])
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    // Then turn each boring edge into a pair of indices
    let lines = boring_edges
        .into_iter()
        .map(|[x, y]| {
            [
                corners.iter().position(|it| *it == x).unwrap(),
                corners.iter().position(|it| *it == y).unwrap(),
            ]
        })
        .collect();
    let line_vertices = corners
        .into_iter()
        .map(|x| x.position().extend(0.0))
        .collect();

    use TriVert::*;

    // First just get a big (deduped) list of all the corners we'll be using
    let corners = directed_edges
        .iter()
        .flat_map(|[x, y]| [Corner::<C>(*x), Corner(*y), Tip(*x, *y)])
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let triangles = directed_edges
        .into_iter()
        .map(|[x, y]| {
            [
                corners.iter().position(|it| *it == Corner(x)).unwrap(),
                corners.iter().position(|it| *it == Corner(y)).unwrap(),
                corners.iter().position(|it| *it == Tip(x, y)).unwrap(),
            ]
        })
        .collect::<Vec<_>>();

    let triangle_vertices = corners
        .into_iter()
        .map(|x| {
            match x {
                Corner(x) => x.position(),
                Tip(a, b) => {
                    let (a, b) = (a.position(), b.position());
                    let midpoint = (a + b) / 2.0;
                    midpoint + (b - a).perp() * arrow_offset
                }
            }
            .extend(0.0)
        })
        .collect();

    let mut meshes = vec![BoardMesh {
        color: BoardColor::PlayerColor,
        mesh: board::Mesh::IndexedLineMesh {
            vertices: line_vertices,
            lines,
        },
    }];
    if triangles.len() > 0 {
        meshes.push(BoardMesh {
            color: BoardColor::PlayerColor,
            mesh: board::Mesh::IndexedTriMesh {
                vertices: triangle_vertices,
                triangles,
            },
        });
    }

    let old_cells = cells;

    let mut cells = Vec::new();

    for cell in &old_cells {
        let neighbors = cell
            .neighbors()
            .into_iter()
            .filter(|x| edges.edge_dir(cell, x) != Some(EdgeDir::BToA))
            .filter_map(|n| old_cells.iter().position(|x| n == *x));
        let neighbors = neighbors
            .map(|n| (n, Path::simple(cell.position(), old_cells[n].position())))
            .collect();
        cells.push(Cell {
            neighbors,
            shape: cell.shape(),
            position: cell.position(),
        });
    }

    Board { cells, meshes }
}

#[derive(Resource, Clone)]
enum Grid {
    BasicSquare {
        cells: HashSet<square::Cell>,
        edges: Edges<square::Cell>,
    },
    BasicHex {
        cells: HashSet<hex::Cell>,
        edges: Edges<hex::Cell>,
    },
}

impl Default for Grid {
    fn default() -> Self {
        Self::default_square()
    }
}

impl Grid {
    fn default_square() -> Self {
        Self::BasicSquare {
            cells: default(),
            edges: default(),
        }
    }

    fn default_hex() -> Self {
        Self::BasicHex {
            cells: default(),
            edges: default(),
        }
    }
}

impl Into<Board> for Grid {
    fn into(self) -> Board {
        match self {
            Grid::BasicSquare { cells, edges } => {
                let mut cells = cells.iter().copied().collect::<Vec<_>>();
                cells.sort_unstable();
                build_board(cells, edges, 0.14)
            }
            Grid::BasicHex { cells, edges } => {
                let cells = cells.iter().copied().collect::<Vec<_>>();
                build_board(cells, edges, 0.21)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EdgeDir {
    AToB,
    BToA,
}

#[derive(Clone, Debug)]
struct Edges<C: BaseCell>(HashMap<C, HashSet<C>>);

impl<C: BaseCell> Default for Edges<C> {
    fn default() -> Self {
        Self(default())
    }
}

impl<C: BaseCell> Edges<C> {
    fn add_one_way_edge(&mut self, from: C, to: C) {
        self.0.entry(to).and_modify(|x| {
            x.remove(&from);
        });
        self.0.entry(from).or_default().insert(to);
    }

    fn remove_cell(&mut self, cell: &C) {
        self.0.remove(cell);
        for other in self.0.values_mut() {
            other.remove(cell);
        }
    }

    fn edge_dir(&self, a: &C, b: &C) -> Option<EdgeDir> {
        if self.0.get(a).is_some_and(|x| x.contains(b)) {
            Some(EdgeDir::AToB)
        } else if self.0.get(b).is_some_and(|x| x.contains(a)) {
            Some(EdgeDir::BToA)
        } else {
            None
        }
    }
}

impl<'a, C: BaseCell> IntoIterator for &'a Edges<C> {
    type Item = <&'a HashMap<C, HashSet<C>> as IntoIterator>::Item;

    type IntoIter = <&'a HashMap<C, HashSet<C>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

fn control_panel(
    mut ui: EguiContexts,
    grid: Res<Grid>,
    exporting: Option<Res<Exporting>>,
    mut commands: Commands,
) {
    egui::Window::new("Control Panel").show(ui.ctx_mut(), |ui| {
        ui.add_enabled_ui(exporting.is_none(), |ui| {
            if exporting.is_some() {
                ui.spinner();
            }
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(matches!(&*grid, Grid::BasicSquare { .. }), "Square")
                    .clicked()
                {
                    if !matches!(grid.as_ref(), Grid::BasicSquare { .. }) {
                        commands.insert_resource(Grid::default_square());
                    }
                }
                if ui
                    .selectable_label(matches!(grid.as_ref(), Grid::BasicHex { .. }), "Hexagon")
                    .clicked()
                {
                    if !matches!(grid.as_ref(), Grid::BasicHex { .. }) {
                        commands.insert_resource(Grid::default_hex());
                    }
                }
            });
            if ui.button("Export JSON...").clicked() {
                commands.add(ExportBoardCmd(grid.clone().into()));
            }
        });
    });
}

fn handle_picks(mut picks: EventReader<Pick>, mut grid: ResMut<Grid>) {
    for &Pick { down, up } in picks.read() {
        match &mut *grid {
            Grid::BasicSquare { cells, edges } => {
                let (down, up) = (square::Cell::pick(down), square::Cell::pick(up));
                if down == up {
                    if !cells.remove(&down) {
                        cells.insert(down);
                    } else {
                        edges.remove_cell(&down);
                    }
                } else if down.adjacent_to(&up) {
                    if !(cells.contains(&up) && cells.contains(&down)) {
                        continue;
                    }
                    edges.add_one_way_edge(down, up);
                }
            }
            Grid::BasicHex { cells, edges } => {
                let (down, up) = (hex::Cell::pick(down), hex::Cell::pick(up));
                if down == up {
                    if !cells.remove(&down) {
                        cells.insert(down);
                    } else {
                        edges.remove_cell(&down);
                    }
                } else if down.adjacent_to(&up) {
                    if !(cells.contains(&up) && cells.contains(&down)) {
                        continue;
                    }
                    edges.add_one_way_edge(down, up);
                }
            }
        };
    }
}

fn draw_grid(grid: Res<Grid>, mut gizmos: Gizmos) {
    match &*grid {
        Grid::BasicSquare { cells, edges } => {
            for x in cells {
                gizmos.square(x.position(), Color::RED)
            }
            for (a, other) in edges {
                for b in other {
                    let (start, end) = (cells.get(a).unwrap(), cells.get(b).unwrap());
                    let (start, end) = (start.position(), end.position());
                    let (start, end) = (start.lerp(end, 0.35), start.lerp(end, 0.65));
                    gizmos
                        .arrow_2d(start, end, Color::ORANGE)
                        .with_tip_length(0.3);
                }
            }
        }
        Grid::BasicHex { cells, edges } => {
            for x in cells {
                gizmos.hex(x.position(), Color::RED);
            }
            for (a, other) in edges {
                for b in other {
                    let (start, end) = (cells.get(a).unwrap(), cells.get(b).unwrap());
                    let (start, end) = (start.position(), end.position());
                    let (start, end) = (start.lerp(end, 0.35), start.lerp(end, 0.65));
                    gizmos
                        .arrow_2d(start, end, Color::ORANGE)
                        .with_tip_length(0.3);
                }
            }
        }
    };
}

fn count_capacity(ui: EguiContexts, grid: Res<Grid>) {
    egui::Window::new("Capacity").show(ui.ctx(), |ui| {
        let board: Board = grid.clone().into();
        let cells = board.cells.iter();
        let capacity: usize = cells.map(|x| x.neighbors.len().max(1) - 1).sum();
        ui.label(format!("Capacity: {capacity}"));
    });
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_build_board() {
        let cells = vec![
            square::Cell { x: 0, y: 0 },
            square::Cell { x: 1, y: 0 },
            square::Cell { x: 0, y: 1 },
            square::Cell { x: 1, y: 1 },
        ];
        let mut edges = Edges::<square::Cell>::default();
        edges.add_one_way_edge(cells[0], cells[1]);
        build_board(cells, edges);
    }
}
