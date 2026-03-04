use deimos_data::tfx::geometry::AxisAlignedBBox;

use crate::visibility::ViewVisibility;

#[derive(Debug, Clone)]
struct FlatNode {
    aabb: AxisAlignedBBox,
    data: NodeData,
}

#[derive(Debug, Clone)]
enum NodeData {
    Internal { right_child: u32 },
    Leaf,
}

pub struct Bvh {
    nodes: Vec<FlatNode>,
}

const LEAF_SIZE: usize = 1;

impl Bvh {
    pub fn build(aabbs: &[AxisAlignedBBox]) -> Self {
        assert!(!aabbs.is_empty(), "Cannot build BVH from empty slice");

        let mut work: Vec<u32> = (0..aabbs.len() as u32).collect();
        let mut nodes = Vec::with_capacity(2 * aabbs.len() / LEAF_SIZE);

        build_recursive(aabbs, &mut work, &mut nodes);

        Self { nodes }
    }

    pub fn is_visible(&self, vis: &ViewVisibility) -> bool {
        if self.nodes.is_empty() {
            return false;
        }

        let mut stack = Vec::with_capacity(64);
        stack.push(0u32);

        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];

            if !vis.is_visible(&node.aabb) {
                continue;
            }

            match node.data {
                NodeData::Leaf => return true,
                NodeData::Internal { right_child } => {
                    stack.push(right_child);
                    stack.push(idx + 1);
                }
            }
        }

        false
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn depth(&self) -> usize {
        fn recurse(nodes: &[FlatNode], idx: usize) -> usize {
            match nodes[idx].data {
                NodeData::Leaf => 1,
                NodeData::Internal { right_child } => {
                    let l = recurse(nodes, idx + 1);
                    let r = recurse(nodes, right_child as usize);
                    1 + l.max(r)
                }
            }
        }
        if self.nodes.is_empty() {
            0
        } else {
            recurse(&self.nodes, 0)
        }
    }
}

fn merged_aabb(aabbs: &[AxisAlignedBBox], work: &[u32]) -> AxisAlignedBBox {
    work.iter()
        .skip(1)
        .fold(aabbs[work[0] as usize].clone(), |acc, &i| {
            acc.union(&aabbs[i as usize])
        })
}

fn build_recursive(aabbs: &[AxisAlignedBBox], work: &mut [u32], nodes: &mut Vec<FlatNode>) {
    let aabb = merged_aabb(aabbs, work);

    if work.len() <= LEAF_SIZE {
        nodes.push(FlatNode {
            aabb,
            data: NodeData::Leaf,
        });
        return;
    }

    let axis = aabb.longest_axis();
    work.sort_unstable_by(|&a, &b| {
        aabbs[a as usize].centroid()[axis]
            .partial_cmp(&aabbs[b as usize].centroid()[axis])
            .unwrap()
    });
    let mid = work.len() / 2;

    let this_index = nodes.len();
    nodes.push(FlatNode {
        aabb,
        data: NodeData::Internal { right_child: 0 },
    });

    build_recursive(aabbs, &mut work[..mid], nodes);

    let right_child = nodes.len() as u32;
    nodes[this_index].data = NodeData::Internal { right_child };

    build_recursive(aabbs, &mut work[mid..], nodes);
}
