use std::f32::consts::{PI, TAU};

use bevy::{
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

struct PhysicalIndexer {
    sectors: u32,
    stacks: u32,
}

impl PhysicalIndexer {
    const END_SECTORS: u32 = 5;
    const END_STACKS: u32 = 2;
    const BOTH_END_STACKS: u32 = 4;

    fn decode_stack(&self, stack: u32) -> (u32, u32) {
        debug_assert!(stack < self.stacks);
        let clamped = stack.max(1) - 1;
        let half = (self.stacks - 2) / 2;
        let z_half = clamped / half;
        (clamped - z_half, z_half)
    }

    fn is_end_stack(&self, stack: u32) -> bool {
        debug_assert!(stack < self.stacks);
        stack < Self::END_STACKS || stack >= self.stacks - Self::END_STACKS
    }

    fn decode_sector(&self, sector: u32, stack: u32) -> (u32, u32) {
        debug_assert!(stack < self.stacks);
        let quarter = self.sectors / 4;
        if self.is_end_stack(stack) {
            debug_assert!(sector < Self::END_SECTORS);
            (sector * (quarter - 1), sector)
        } else {
            debug_assert!(sector < self.sectors);
            let xy_quarter = sector / quarter;
            (sector - xy_quarter, xy_quarter)
        }
    }

    fn sectors(&self, stack: u32) -> u32 {
        debug_assert!(stack < self.stacks);
        if self.is_end_stack(stack) {
            Self::END_SECTORS
        } else {
            self.sectors
        }
    }

    fn stretch(&self, stack: u32) -> bool {
        debug_assert!(stack < self.stacks);
        stack != 0 && stack != self.stacks - 1
    }

    fn index(&self, sector: u32, stack: u32) -> u32 {
        debug_assert!(sector < self.sectors);
        debug_assert!(stack < self.stacks);
        let quarter = self.sectors / 4;
        if stack < Self::END_STACKS {
            stack * Self::END_SECTORS + (sector / quarter)
        } else if stack >= self.stacks - Self::END_STACKS {
            let full_stacks = self.stacks - Self::BOTH_END_STACKS;
            (stack - full_stacks) * Self::END_SECTORS
                + full_stacks * self.sectors
                + (sector / quarter)
        } else {
            Self::END_STACKS * Self::END_SECTORS
                + (stack - Self::END_STACKS) * self.sectors
                + sector
        }
    }

    fn total_vertices(&self) -> usize {
        (self.sectors * (self.stacks - Self::BOTH_END_STACKS)
            + Self::END_SECTORS * Self::BOTH_END_STACKS) as usize
    }

    fn total_indices(&self) -> usize {
        6 * ((self.sectors - 1) * (self.stacks - 3) - (self.sectors - 9) + (Self::END_SECTORS - 1))
            as usize
    }
}

/// A rounded box.
#[derive(Debug, Copy, Clone)]
pub struct RoundedBox {
    /// The dimensions of the box
    pub size: Vec3,
    /// The radius of the corners and edges
    pub radius: f32,
    /// The number of sectors and stacks in each corner
    pub subdivisions: usize,
}

impl From<RoundedBox> for Mesh {
    // Based on bevy_render::mesh::shape::UVSphere
    fn from(rbox: RoundedBox) -> Self {
        debug_assert!(rbox.subdivisions > 0);
        let logical_sectors = 4 * rbox.subdivisions;
        let logical_stacks = 2 * rbox.subdivisions;
        let physical = PhysicalIndexer {
            sectors: (logical_sectors + 4 + 1) as u32,
            stacks: (logical_stacks + 2 + 2) as u32,
        };

        let core_size = rbox.size - 2.0 * rbox.radius;
        let core_offset = core_size / 2.0;
        let sector_step = TAU / logical_sectors as f32;
        let stack_step = PI / logical_stacks as f32;

        let sector_len = TAU * rbox.radius + 2.0 * (core_size.x + core_size.y);
        let top_radius = (core_offset.x + core_offset.y) / 2.0;
        let stack_len = PI * rbox.radius + core_size.z + 2.0 * top_radius;

        let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(physical.total_vertices());
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(physical.total_vertices());
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(physical.total_vertices());
        let mut indices: Vec<u32> = Vec::with_capacity(physical.total_indices());

        // Generate vertices
        for p_stack in 0..physical.stacks {
            let (logical_stack, z_half) = physical.decode_stack(p_stack);
            let stack_angle = PI / 2. - (logical_stack as f32) * stack_step;
            let xy = stack_angle.cos();
            let nz = stack_angle.sin();

            // Calculate Z coordinate
            let cz = rbox.radius * nz
                + match z_half {
                    0 => core_offset.z,
                    _ => -core_offset.z,
                };
            let xy_stretch = if physical.stretch(p_stack) { 1.0 } else { 0.0 };

            // Calculate V texture coordinate
            let stack_corner_dist =
                PI * rbox.radius * (logical_stack as f32) / (logical_stacks as f32);
            let stack_face_dist = match (p_stack, z_half) {
                (0, _) => 0.0,
                (_, 0) => top_radius,
                (_, 1) => top_radius + core_size.z,
                (_, 2) => 2.0 * top_radius + core_size.z,
                _ => unreachable!(),
            };
            let v_coord = (stack_corner_dist + stack_face_dist) / stack_len;

            for p_sector in 0..physical.sectors(p_stack) {
                let (logical_sector, xy_quarter) = physical.decode_sector(p_sector, p_stack);
                let sector_angle = (logical_sector as f32) * sector_step;
                let nx = xy * sector_angle.cos();
                let ny = xy * sector_angle.sin();

                // Calculate X and Y coordinates
                let cx = rbox.radius * nx
                    + xy_stretch
                        * match xy_quarter % 4 {
                            0 | 3 => core_offset.x,
                            1 | 2 => -core_offset.x,
                            _ => unreachable!(),
                        };
                let cy = rbox.radius * ny
                    + xy_stretch
                        * match xy_quarter % 4 {
                            0 | 1 => core_offset.y,
                            2 | 3 => -core_offset.y,
                            _ => unreachable!(),
                        };

                // Calculate U texture coordinate
                // FIXME: This is twisted on the end stacks in part because U
                // is not 0 in the middle of a corner.
                let sector_corner_dist =
                    TAU * rbox.radius * (logical_sector as f32) / (logical_sectors as f32);
                let sector_face_dist = match xy_quarter {
                    0 => 0.0,
                    1 => core_size.y,
                    2 => core_size.x + core_size.y,
                    3 => core_size.x + 2.0 * core_size.y,
                    4 => 2.0 * core_size.x + 2.0 * core_size.y,
                    _ => unreachable!(),
                };
                let u_coord = (sector_corner_dist + sector_face_dist) / sector_len;

                vertices.push([cx, cy, cz]);
                normals.push([nx, ny, nz]);
                uvs.push([u_coord, v_coord]);
            }
        }

        // Generate indices
        for p_stack in 0..physical.stacks - 1 {
            for p_sector in 0..physical.sectors - 1 {
                let jj = physical.index(p_sector, p_stack);
                let jk = physical.index(p_sector, p_stack + 1);
                let kj = physical.index(p_sector + 1, p_stack);
                let kk = physical.index(p_sector + 1, p_stack + 1);
                // Exclude degenerate triangles
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

        debug_assert_eq!(indices.len(), physical.total_indices());
        debug_assert_eq!(vertices.len(), physical.total_vertices());
        debug_assert_eq!(normals.len(), physical.total_vertices());
        debug_assert_eq!(uvs.len(), physical.total_vertices());

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}

#[test]
fn test_create_mesh() {
    // Check debug assertions
    for subdivisions in 1..5 {
        let _ = Mesh::from(RoundedBox {
            size: Vec3::new(1.0, 1.0, 1.0),
            radius: 0.1,
            subdivisions,
        });
    }
}
