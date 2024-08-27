use crate::board::{Cell, Mesh, Path, Polygon};
use bevy::{
    math::{Vec2, Vec3, Vec3Swizzles},
    utils::FloatOrd,
};
use gltf::{mesh::Mode, Gltf, Primitive, Semantic};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    iter::once,
};

#[derive(Debug, Clone)]
struct IndexedLineMesh {
    vertices: Vec<Vec3>,
    lines: Vec<[usize; 2]>,
}

impl IndexedLineMesh {
    fn neighbors(&self, v: usize) -> impl Iterator<Item = usize> + '_ {
        self.lines
            .iter()
            .filter(move |x| x.iter().contains(&v))
            .map(move |&[x, y]| if x == v { y } else { x })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct Loop(Vec<usize>);

impl Loop {
    fn new(vertices: impl Into<VecDeque<usize>>) -> Self {
        let mut vertices = vertices.into();
        let min = vertices.iter().position_min().unwrap();
        vertices.rotate_left(min);
        Self(vertices.into_iter().collect())
    }

    fn edges(&self) -> impl Iterator<Item = [usize; 2]> + '_ {
        self.0
            .iter()
            .chain(once(self.0.first().unwrap()))
            .tuple_windows::<(&usize, &usize)>()
            .map(|(&x, &y)| [x.min(y), x.max(y)])
    }

    fn perimeter(&self, vertices: &[Vec3]) -> f32 {
        self.edges()
            .map(|[a, b]| vertices[a].distance(vertices[b]))
            .sum()
    }
}

fn lines<'a>(x: &Primitive, blob: &'a [u8]) -> impl Iterator<Item = [usize; 2]> + 'a {
    let accessor = x.indices().unwrap();
    let view = accessor.view().unwrap();
    let start = accessor.offset() + view.offset();
    let end = start + accessor.count() * accessor.size();
    bytemuck::cast_slice::<_, [u16; 2]>(&blob[start..end])
        .iter()
        .map(|&[a, b]| [a as usize, b as usize])
}

fn tris<'a>(x: &Primitive, blob: &'a [u8]) -> impl Iterator<Item = [usize; 3]> + 'a {
    let accessor = x.indices().unwrap();
    let view = accessor.view().unwrap();
    let start = accessor.offset() + view.offset();
    let end = start + accessor.count() * accessor.size();
    bytemuck::cast_slice::<_, [u16; 3]>(&blob[start..end])
        .iter()
        .map(|&[a, b, c]| [a as usize, b as usize, c as usize])
}

fn positions<'a>(x: &Primitive, blob: &'a [u8]) -> impl Iterator<Item = Vec3> + 'a {
    let accessor = x.get(&Semantic::Positions).unwrap();
    let view = accessor.view().unwrap();
    let start = accessor.offset() + view.offset();
    let end = start + accessor.count() * accessor.size();
    bytemuck::cast_slice(&blob[start..end]).iter().copied()
}

pub fn process_gltf(gltf: Gltf) -> (Vec<Vec<Cell>>, Vec<Mesh>) {
    let blob = &gltf.blob.as_ref().unwrap()[..];
    let mut meshes = Vec::new();
    for mesh in gltf.meshes() {
        for prim in mesh.primitives() {
            let vertices = positions(&prim, blob).collect();
            let mesh = match prim.mode() {
                Mode::Lines => {
                    let lines = lines(&prim, blob).collect();
                    Mesh::IndexedLineMesh { vertices, lines }
                }
                Mode::LineLoop => todo!(),
                Mode::LineStrip => todo!(),
                Mode::Triangles => {
                    let triangles = tris(&prim, blob).collect();
                    Mesh::IndexedTriMesh {
                        vertices,
                        triangles,
                    }
                }
                Mode::TriangleStrip => todo!(),
                Mode::TriangleFan => todo!(),
                Mode::Points => {
                    eprintln!("Can't load point meshes");
                    continue;
                }
            };
            meshes.push(mesh);
        }
    }

    let boards = meshes
        .iter()
        .filter_map(|x| match x {
            Mesh::IndexedLineMesh { vertices, lines } => Some(IndexedLineMesh {
                vertices: vertices.clone(),
                lines: lines.clone(),
            }),
            Mesh::IndexedTriMesh { .. } => None,
        })
        .map(|mesh| {
            let mut loops = HashSet::new();
            for v in 0..mesh.vertices.len() {
                for next in mesh.neighbors(v) {
                    let mut current = VecDeque::new();
                    let mut a = v;
                    let mut b = next;
                    let mut seen = HashSet::new();

                    loop {
                        current.push_back(a);
                        if b == v {
                            loops.insert(Loop::new(current));
                            break;
                        }
                        // Protection from infinite loops for line meshes that don't have only loops
                        if seen.contains(&a) {
                            // Not a loop :(
                            break;
                        }
                        seen.insert(a);
                        let ab = mesh.vertices[b].xy() - mesh.vertices[a].xy();
                        let c = mesh
                            .neighbors(b)
                            .filter(|&x| x != a)
                            .max_by_key(|&x| {
                                let bx = mesh.vertices[x].xy() - mesh.vertices[b].xy();
                                FloatOrd(ab.angle_between(bx))
                            })
                            .unwrap();
                        a = b;
                        b = c;
                    }
                }
            }
            let max_loop_perimeter = loops
                .iter()
                .map(|x| FloatOrd(x.perimeter(&mesh.vertices)))
                .max()
                .unwrap();
            loops.retain(|x| FloatOrd(x.perimeter(&mesh.vertices)) < max_loop_perimeter);
            let loops = loops.into_iter().collect::<Vec<_>>();
            println!("{} loops: {loops:?}", loops.len());
            let shapes = loops
                .iter()
                .map(|x| Polygon {
                    points: x.0.iter().map(|&x| mesh.vertices[x].xy()).collect(),
                })
                .collect::<Vec<_>>();
            let positions = shapes
                .iter()
                .map(|x| x.points.iter().fold(Vec2::ZERO, |x, &y| x + y) / x.points.len() as f32)
                .collect::<Vec<_>>();
            let meta = shapes.iter().zip(&positions);

            loops
                .iter()
                .zip(meta)
                .map(|(l, (shape, &position))| {
                    let edges = l.edges().collect::<HashSet<_>>();
                    let neighbors = loops.iter().positions(|x| {
                        let neighbor_edges = x.edges().collect::<HashSet<_>>();
                        if edges.difference(&neighbor_edges).count() == 0 {
                            false
                        } else {
                            neighbor_edges.intersection(&edges).count() > 0
                        }
                    });
                    let neighbors = neighbors
                        .map(|x| {
                            let neighbor_position = positions[x];
                            (x, Path::simple(position, neighbor_position))
                        })
                        .collect::<HashMap<_, _>>();
                    Cell {
                        neighbors,
                        shape: shape.clone(),
                        position,
                    }
                })
                .collect()
        })
        .collect();

    (boards, meshes)
}
