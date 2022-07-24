use std::f32::consts::{PI, TAU};

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute, PrimitiveTopology},
        render_resource::VertexFormat,
    },
};

#[derive(Copy, Clone)]
struct XYQuarter(u32);

impl XYQuarter {
    fn coords(self) -> Vec2 {
        match self.0 % 4 {
            0 => Vec2::new(1.0, 1.0),
            1 => Vec2::new(-1.0, 1.0),
            2 => Vec2::new(-1.0, -1.0),
            3 => Vec2::new(1.0, -1.0),
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum ZHalf {
    Top,
    Bottom,
}

impl ZHalf {
    fn from(n: u32) -> Self {
        if n == 0 {
            ZHalf::Top
        } else {
            ZHalf::Bottom
        }
    }

    fn coord(self) -> f32 {
        match self {
            ZHalf::Top => 1.0,
            ZHalf::Bottom => -1.0,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum StackType {
    Ultimate(ZHalf),
    Penultimate(ZHalf),
    Ordinary,
}

struct PhysicalIndexer {
    subdivisions: u32,
    extra_levels: u32,
    sectors: u32,
    stacks: u32,
}

impl PhysicalIndexer {
    const ULTIMATE_SECTORS: u32 = 1;
    const PENULTIMATE_SECTORS: u32 = 4;
    const TOTAL_END_SECTORS: u32 = Self::ULTIMATE_SECTORS + Self::PENULTIMATE_SECTORS;
    const END_STACKS: u32 = 2;
    const BOTH_END_STACKS: u32 = 2 * Self::END_STACKS;

    fn decode_stack(&self, stack: u32) -> (u32, ZHalf) {
        debug_assert!(stack < self.stacks);
        let clamped = stack.max(1) - 1;
        let half = (self.stacks - 2) / 2;
        let z_half = clamped / (half / self.extra_levels);
        (clamped - z_half, ZHalf::from(z_half / self.extra_levels))
    }

    fn stack_type(&self, stack: u32) -> StackType {
        debug_assert!(stack < self.stacks);
        if stack == 0 {
            StackType::Ultimate(ZHalf::Top)
        } else if stack == self.stacks - 1 {
            StackType::Ultimate(ZHalf::Bottom)
        } else if stack == 1 {
            StackType::Penultimate(ZHalf::Top)
        } else if stack == self.stacks - 2 {
            StackType::Penultimate(ZHalf::Bottom)
        } else {
            StackType::Ordinary
        }
    }

    fn decode_sector(&self, sector: u32, stack: u32) -> (u32, XYQuarter) {
        let quarter = self.sectors / 4;
        match self.stack_type(stack) {
            StackType::Ultimate(_) => {
                debug_assert!(sector == 0);
                (0, XYQuarter(0))
            }
            StackType::Penultimate(_) => {
                debug_assert!(sector < Self::PENULTIMATE_SECTORS);
                (sector * (quarter - self.extra_levels), XYQuarter(sector))
            }
            StackType::Ordinary => {
                debug_assert!(sector < self.sectors);
                let xy_quarter = sector / (quarter / self.extra_levels);
                (
                    sector - xy_quarter,
                    XYQuarter(xy_quarter / self.extra_levels),
                )
            }
        }
    }

    fn sectors(&self, stack: u32) -> u32 {
        match self.stack_type(stack) {
            StackType::Ultimate(_) => Self::ULTIMATE_SECTORS,
            StackType::Penultimate(_) => Self::PENULTIMATE_SECTORS,
            StackType::Ordinary => self.sectors,
        }
    }

    fn stretch_xy(&self, stack: u32) -> bool {
        match self.stack_type(stack) {
            StackType::Ultimate(_) => false,
            StackType::Penultimate(_) | StackType::Ordinary => true,
        }
    }

    fn index(&self, sector: u32, stack: u32) -> u32 {
        let quarter = self.sectors / Self::PENULTIMATE_SECTORS;
        match self.stack_type(stack) {
            StackType::Ultimate(ZHalf::Top) => 0,
            StackType::Penultimate(ZHalf::Top) => {
                Self::ULTIMATE_SECTORS + ((sector / quarter) % Self::PENULTIMATE_SECTORS)
            }
            StackType::Ordinary => {
                Self::TOTAL_END_SECTORS
                    + (stack - Self::END_STACKS) * self.sectors
                    + (sector % self.sectors)
            }
            StackType::Penultimate(ZHalf::Bottom) => {
                Self::TOTAL_END_SECTORS
                    + (self.stacks - Self::BOTH_END_STACKS) * self.sectors
                    + ((sector / quarter) % Self::PENULTIMATE_SECTORS)
            }
            StackType::Ultimate(ZHalf::Bottom) => {
                Self::TOTAL_END_SECTORS
                    + Self::PENULTIMATE_SECTORS
                    + (self.stacks - Self::BOTH_END_STACKS) * self.sectors
            }
        }
    }

    fn total_vertices(&self) -> usize {
        (self.sectors * (self.stacks - Self::BOTH_END_STACKS) + 2 * Self::TOTAL_END_SECTORS)
            as usize
    }

    fn total_indices(&self) -> usize {
        6 * ((self.sectors - Self::PENULTIMATE_SECTORS * (self.extra_levels - 1))
            * (self.stacks - (Self::BOTH_END_STACKS - 1) - 2 * (self.extra_levels - 1))
            - (Self::PENULTIMATE_SECTORS * (self.subdivisions - 1))) as usize
    }

    fn face(&self, sector: u32, stack: u32) -> u32 {
        let half_subdivisions = self.subdivisions / 2;
        if stack < Self::END_STACKS + half_subdivisions {
            0
        } else if stack > self.stacks - Self::END_STACKS - half_subdivisions - 1 {
            5
        } else {
            1 + ((sector + self.sectors - half_subdivisions - 1) / (self.sectors / 4)) % 4
        }
    }

    fn uv_coords(&self, rounded_length: f32, core_size: Vec3, sector: u32, stack: u32) -> Vec2 {
        let half_subdivisions = self.subdivisions / 2;
        let (logical_sector, xy_quarter) = self.decode_sector(sector, stack);
        let face = self.face(sector, stack);
        match face {
            0 | 5 => match self.stack_type(stack) {
                StackType::Ultimate(_) => Vec2::new(0.5, 0.5),
                StackType::Penultimate(_) | StackType::Ordinary => {
                    let dist = (stack
                        .min(self.stacks - stack - 1)
                        .clamp(1, half_subdivisions + 1)
                        - 1) as f32
                        / half_subdivisions as f32;
                    let corner_sector = logical_sector % self.subdivisions;
                    let octant = logical_sector / half_subdivisions;
                    let mirrored_sector = if corner_sector <= half_subdivisions {
                        corner_sector
                    } else {
                        2 * half_subdivisions - corner_sector
                    };
                    let edge_len = mirrored_sector as f32 / half_subdivisions as f32;
                    let edge_vec = if ((octant + 1) / 2) % 2 == 1 {
                        Vec2::new(edge_len, 1.0)
                    } else {
                        Vec2::new(1.0, edge_len)
                    };
                    let unmirror = match face {
                        0 => Vec2::new(1.0, -1.0),
                        _ => Vec2::ONE,
                    };
                    let v = unmirror
                        * xy_quarter.coords()
                        * (dist * edge_vec * rounded_length + 0.5 * core_size.truncate());
                    0.5 + (v / (core_size.truncate() + 2.0 * rounded_length))
                }
            },
            1..=4 => {
                let u_core_len = match face {
                    1 | 3 => core_size.x,
                    2 | 4 => core_size.y,
                    _ => unreachable!(),
                };
                let u_offset = (4 * self.subdivisions + half_subdivisions + logical_sector
                    - face * self.subdivisions)
                    % (4 * self.subdivisions);
                let u_off_len = rounded_length * u_offset as f32 / half_subdivisions as f32
                    + if face % 4 == xy_quarter.0 {
                        u_core_len
                    } else {
                        0.0
                    };
                let v_offset = stack - half_subdivisions - 2;
                let v_off_len = rounded_length
                    * ((v_offset % (half_subdivisions + 1)) as f32 / half_subdivisions as f32)
                    + if v_offset > half_subdivisions {
                        core_size.z + rounded_length
                    } else {
                        0.0
                    };
                Vec2::new(
                    u_off_len / (u_core_len + 2.0 * rounded_length),
                    v_off_len / (core_size.z + 2.0 * rounded_length),
                )
            }
            _ => unreachable!(),
        }
    }
}

pub const ATTRIBUTE_FACE: MeshVertexAttribute =
    MeshVertexAttribute::new("Face", 15543717107074212298, VertexFormat::Uint32);

/// Options for generating the mesh of a [`RoundedBox`](RoundedBox)
#[derive(Copy, Clone, Debug, Default)]
pub struct BoxMeshOptions {
    // Generate ATTRIBUTE_UV_0
    pub generate_uv: bool,
    // Generate ATTRIBUTE_FACE
    pub generate_face: bool,
}

impl BoxMeshOptions {
    fn is_generate_uv(&self) -> bool {
        self.generate_uv
    }

    fn is_generate_face(&self) -> bool {
        self.generate_face
    }

    fn is_split_faces(&self) -> bool {
        self.generate_uv || self.generate_face
    }
}

/// A rounded box.
#[derive(Copy, Clone, Debug)]
pub struct RoundedBox {
    /// The dimensions of the box
    pub size: Vec3,
    /// The radius of the corners and edges
    pub radius: f32,
    /// The number of sectors and stacks in each corner
    pub subdivisions: usize,
    /// Mesh generation options
    pub options: BoxMeshOptions,
}

impl From<RoundedBox> for Mesh {
    // Based on bevy_render::mesh::shape::UVSphere
    fn from(rbox: RoundedBox) -> Self {
        debug_assert!(rbox.subdivisions > 0);
        let subdivisions = if rbox.options.is_split_faces() {
            rbox.subdivisions + rbox.subdivisions % 2
        } else {
            rbox.subdivisions
        } as u32;
        let logical_sectors = 4 * subdivisions;
        let logical_stacks = 2 * subdivisions;
        let extra_levels = if rbox.options.is_split_faces() { 2 } else { 1 };
        let physical = PhysicalIndexer {
            subdivisions,
            extra_levels,
            sectors: (logical_sectors + 4 * extra_levels),
            stacks: (logical_stacks + 2 + 2 * extra_levels),
        };

        let core_size = rbox.size - 2.0 * rbox.radius;
        let core_offset = core_size / 2.0;
        let sector_step = TAU / logical_sectors as f32;
        let stack_step = PI / logical_stacks as f32;
        let rounded_length = 0.125 * TAU * rbox.radius;

        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(physical.total_vertices());
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(physical.total_vertices());
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(physical.total_vertices());
        let mut faces: Vec<u32> = Vec::with_capacity(physical.total_vertices());
        let mut indices: Vec<u32> = Vec::with_capacity(physical.total_indices());

        // Generate vertices
        for p_stack in 0..physical.stacks {
            let (logical_stack, z_half) = physical.decode_stack(p_stack);
            let stack_angle = PI / 2. - (logical_stack as f32) * stack_step;
            let xy = stack_angle.cos();

            // Calculate Z component of normal
            let normal_z = stack_angle.sin();

            // Calculate Z coordinate
            let pos_z = rbox.radius * normal_z + core_offset.z * z_half.coord();
            let stretch_xy = if physical.stretch_xy(p_stack) {
                1.0
            } else {
                0.0
            };

            for p_sector in 0..physical.sectors(p_stack) {
                let (logical_sector, xy_quarter) = physical.decode_sector(p_sector, p_stack);
                let sector_angle = (logical_sector as f32) * sector_step;

                // Calculate X and Y components of normal
                let normal_xy = xy * Vec2::new(sector_angle.cos(), sector_angle.sin());
                normals.push(normal_xy.extend(normal_z).to_array());

                // Calculate X and Y coordinates
                let pos_xy = rbox.radius * normal_xy
                    + stretch_xy * core_offset.truncate() * xy_quarter.coords();
                positions.push(pos_xy.extend(pos_z).to_array());

                // Calculate texture coordinates
                if rbox.options.is_generate_uv() {
                    uvs.push(
                        physical
                            .uv_coords(rounded_length, core_size, p_sector, p_stack)
                            .to_array(),
                    );
                }

                // Calculate face index
                if rbox.options.is_generate_face() {
                    faces.push(physical.face(p_sector, p_stack));
                }
            }
        }

        // Generate indices
        for p_stack in 0..physical.stacks - 1 {
            for p_sector in 0..physical.sectors {
                // Skip degenerate triangles between split faces
                if rbox.options.is_split_faces() {
                    if p_stack == PhysicalIndexer::END_STACKS + subdivisions / 2 - 1
                        || p_stack
                            == physical.stacks - PhysicalIndexer::END_STACKS - subdivisions / 2 - 1
                    {
                        continue;
                    }
                    if (p_sector + (subdivisions + extra_levels) / 2 + 1)
                        % (subdivisions + extra_levels)
                        == 0
                    {
                        continue;
                    }
                }
                // Calculate indicies for quad
                let jj = physical.index(p_sector, p_stack);
                let jk = physical.index(p_sector, p_stack + 1);
                let kj = physical.index(p_sector + 1, p_stack);
                let kk = physical.index(p_sector + 1, p_stack + 1);
                // Exclude degenerate triangles near the end stacks
                if (jj != jk) && (jj != kj) && (jk != kj) {
                    indices.push(jj);
                    indices.push(jk);
                    indices.push(kj);
                }
                if (kj != jk) && (kj != kk) && (jk != kk) {
                    indices.push(kj);
                    indices.push(jk);
                    indices.push(kk);
                }
            }
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        debug_assert_eq!(indices.len(), physical.total_indices());
        mesh.set_indices(Some(Indices::U32(indices)));
        debug_assert_eq!(positions.len(), physical.total_vertices());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        debug_assert_eq!(normals.len(), physical.total_vertices());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        if rbox.options.generate_uv {
            debug_assert_eq!(uvs.len(), physical.total_vertices());
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }
        if rbox.options.generate_face {
            debug_assert_eq!(faces.len(), physical.total_vertices());
            mesh.insert_attribute(ATTRIBUTE_FACE, faces);
        }
        mesh
    }
}

#[test]
fn test_create_mesh() {
    // Check debug assertions
    for subdivisions in 1..=10 {
        for uvf in [false, true] {
            println!("subdivions={} uvf={}", subdivisions, uvf);
            let mesh = Mesh::from(RoundedBox {
                size: Vec3::new(1.0, 1.0, 1.0),
                radius: 0.1,
                subdivisions,
                options: BoxMeshOptions {
                    generate_uv: uvf,
                    generate_face: uvf,
                },
            });
            println!(
                "indices={} vertices={}",
                mesh.indices().unwrap().len(),
                mesh.count_vertices()
            )
        }
    }
}
